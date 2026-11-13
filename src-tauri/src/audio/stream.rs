//! Progressive HTTP media source: downloads in a background thread while the decoder
//! reads. Reads/seeks past the downloaded point block until the data arrives.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

struct State {
    data: Vec<u8>,
    done: bool,
    error: Option<String>,
}

struct Shared {
    state: Mutex<State>,
    cond: Condvar,
    cancelled: AtomicBool,
    total: Option<u64>,
}

pub struct HttpSource {
    shared: Arc<Shared>,
    pos: u64,
}

/// Handle used to abort the download (dropping the source does this too).
#[derive(Clone)]
pub struct CancelHandle(Arc<Shared>);

impl CancelHandle {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::SeqCst);
        self.0.cond.notify_all();
    }
}

impl HttpSource {
    pub fn open(url: &str) -> anyhow::Result<(Self, CancelHandle)> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(8))
            .timeout(None)
            .build()?;
        let mut resp = client.get(url).send()?;
        if !resp.status().is_success() {
            anyhow::bail!("stream HTTP {}", resp.status());
        }
        let ctype = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ctype.contains("json") || ctype.contains("xml") {
            anyhow::bail!("server returned an error instead of audio");
        }
        let total = resp.content_length();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                data: Vec::with_capacity(total.unwrap_or(8 << 20) as usize),
                done: false,
                error: None,
            }),
            cond: Condvar::new(),
            cancelled: AtomicBool::new(false),
            total,
        });
        let dl = shared.clone();
        std::thread::Builder::new().name("tunes-download".into()).spawn(move || {
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                if dl.cancelled.load(Ordering::Relaxed) {
                    break;
                }
                match resp.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        dl.state.lock().unwrap().data.extend_from_slice(&buf[..n]);
                        dl.cond.notify_all();
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        dl.state.lock().unwrap().error = Some(e.to_string());
                        break;
                    }
                }
            }
            dl.state.lock().unwrap().done = true;
            dl.cond.notify_all();
        })?;
        let handle = CancelHandle(shared.clone());
        Ok((Self { shared, pos: 0 }, handle))
    }

    /// Block until `want` bytes exist (or the download ends). Returns bytes available.
    fn wait_for(&self, want: u64) -> io::Result<u64> {
        let mut st = self.shared.state.lock().unwrap();
        loop {
            if self.shared.cancelled.load(Ordering::Relaxed) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            let have = st.data.len() as u64;
            if have >= want || st.done {
                if have < want {
                    if let Some(e) = &st.error {
                        return Err(io::Error::other(e.clone()));
                    }
                }
                return Ok(have);
            }
            st = self.shared.cond.wait_timeout(st, Duration::from_millis(250)).unwrap().0;
        }
    }
}

impl Drop for HttpSource {
    fn drop(&mut self) {
        self.shared.cancelled.store(true, Ordering::SeqCst);
        self.shared.cond.notify_all();
    }
}

impl Read for HttpSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let have = self.wait_for(self.pos + 1)?;
        if self.pos >= have {
            return Ok(0);
        }
        let st = self.shared.state.lock().unwrap();
        let start = self.pos as usize;
        let n = out.len().min(st.data.len() - start);
        out[..n].copy_from_slice(&st.data[start..start + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for HttpSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::Current(d) => self.pos as i64 + d,
            SeekFrom::End(d) => {
                let len = match self.shared.total {
                    Some(t) => t,
                    None => self.wait_for(u64::MAX)?,
                };
                len as i64 + d
            }
        };
        if target < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before start"));
        }
        self.pos = target as u64;
        Ok(self.pos)
    }
}

impl symphonia::core::io::MediaSource for HttpSource {
    fn is_seekable(&self) -> bool {
        self.shared.total.is_some()
    }
    fn byte_len(&self) -> Option<u64> {
        self.shared.total
    }
}
