//! Minimal file logger: %LOCALAPPDATA%\Tunes\tunes.log (Windows) / ~/.cache/Tunes/tunes.log.
//! Truncated on each launch so it never grows unbounded.

use crate::config::{app_dir, DirKind};
use log::{Level, LevelFilter, Log, Metadata, Record};
use parking_lot::Mutex;
use std::fs::File;
use std::io::Write;

struct FileLogger {
    file: Mutex<Option<File>>,
    start: std::time::Instant,
}

impl Log for FileLogger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= Level::Info && m.target().starts_with("tunes")
    }
    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let line = format!("[{:>8.3}s {:<5}] {}\n", self.start.elapsed().as_secs_f32(), r.level(), r.args());
        #[cfg(debug_assertions)]
        eprint!("{line}");
        if let Some(f) = self.file.lock().as_mut() {
            let _ = f.write_all(line.as_bytes());
        }
    }
    fn flush(&self) {
        if let Some(f) = self.file.lock().as_mut() {
            let _ = f.flush();
        }
    }
}

pub fn init() {
    let file = File::create(app_dir(DirKind::Cache).join("tunes.log")).ok();
    let logger = Box::leak(Box::new(FileLogger { file: Mutex::new(file), start: std::time::Instant::now() }));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
    log::info!("Tunes {} starting", env!("CARGO_PKG_VERSION"));
}
