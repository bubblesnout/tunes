//! Minimal Subsonic / OpenSubsonic client (works with Navidrome).

use anyhow::{anyhow, bail, Context, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const API_VERSION: &str = "1.16.1";
const CLIENT: &str = "Tunes";

#[derive(Clone, Debug)]
pub struct Credentials {
    pub server: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct Client {
    creds: Credentials,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    #[serde(default, alias = "title")]
    pub name: String,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub artist_id: Option<String>,
    #[serde(default)]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub song_count: Option<u32>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub genre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub album_id: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub track: Option<u32>,
    #[serde(default)]
    pub disc_number: Option<u32>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    #[serde(flatten)]
    pub album: Album,
    #[serde(default)]
    pub song: Vec<Song>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub song_count: Option<u32>,
    #[serde(default)]
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    #[serde(flatten)]
    pub playlist: Playlist,
    #[serde(default)]
    pub entry: Vec<Song>,
}

impl Client {
    pub fn new(mut creds: Credentials) -> Self {
        creds.server = normalise_server(&creds.server);
        let http = reqwest::Client::builder()
            .user_agent(CLIENT)
            .connect_timeout(std::time::Duration::from_secs(8))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("http client");
        Self { creds, http }
    }

    pub fn creds(&self) -> &Credentials {
        &self.creds
    }

    /// Query string with token auth (`t = md5(password + salt)`).
    pub fn auth_query(&self) -> String {
        let salt: String = rand::thread_rng().sample_iter(&Alphanumeric).take(12).map(char::from).collect();
        let token = format!("{:x}", md5::compute(format!("{}{}", self.creds.password, salt)));
        format!(
            "u={}&t={}&s={}&v={}&c={}&f=json",
            urlencoding::encode(&self.creds.username),
            token,
            salt,
            API_VERSION,
            CLIENT
        )
    }

    pub fn url(&self, endpoint: &str, params: &[(&str, &str)]) -> String {
        let mut url = format!("{}/rest/{}?{}", self.creds.server, endpoint, self.auth_query());
        for (k, v) in params {
            url.push('&');
            url.push_str(k);
            url.push('=');
            url.push_str(&urlencoding::encode(v));
        }
        url
    }

    async fn call(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Value> {
        let resp = self
            .http
            .get(self.url(endpoint, params))
            .send()
            .await
            .with_context(|| format!("Could not reach server at {}", self.creds.server))?;
        if !resp.status().is_success() {
            bail!("Server returned HTTP {}", resp.status());
        }
        let body: Value = resp.json().await.context("Server did not return a Subsonic JSON response")?;
        let root = body
            .get("subsonic-response")
            .cloned()
            .ok_or_else(|| anyhow!("Not a Subsonic server"))?;
        if root.get("status").and_then(Value::as_str) != Some("ok") {
            let msg = root
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("Unknown server error");
            bail!("{msg}");
        }
        Ok(root)
    }

    pub async fn ping(&self) -> Result<()> {
        self.call("ping", &[]).await.map(|_| ())
    }

    /// `kind`: newest | recent | frequent | random | starred | highest | alphabeticalByName ...
    pub async fn album_list(&self, kind: &str, size: u32) -> Result<Vec<Album>> {
        let size = size.to_string();
        let root = self.call("getAlbumList2", &[("type", kind), ("size", &size)]).await?;
        Ok(from_list(root.pointer("/albumList2/album")))
    }

    pub async fn album(&self, id: &str) -> Result<AlbumDetail> {
        let root = self.call("getAlbum", &[("id", id)]).await?;
        Ok(serde_json::from_value(root.get("album").cloned().unwrap_or_default())?)
    }

    pub async fn playlists(&self) -> Result<Vec<Playlist>> {
        let root = self.call("getPlaylists", &[]).await?;
        Ok(from_list(root.pointer("/playlists/playlist")))
    }

    pub async fn playlist(&self, id: &str) -> Result<PlaylistDetail> {
        let root = self.call("getPlaylist", &[("id", id)]).await?;
        Ok(serde_json::from_value(root.get("playlist").cloned().unwrap_or_default())?)
    }

    pub async fn scrobble(&self, id: &str, submission: bool) -> Result<()> {
        let sub = if submission { "true" } else { "false" };
        self.call("scrobble", &[("id", id), ("submission", sub)]).await.map(|_| ())
    }

    pub async fn cover_bytes(&self, id: &str, size: u32) -> Result<(Vec<u8>, String)> {
        let size = size.to_string();
        let resp = self.http.get(self.url("getCoverArt", &[("id", id), ("size", &size)])).send().await?;
        if !resp.status().is_success() {
            bail!("cover HTTP {}", resp.status());
        }
        let ctype = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        if ctype.contains("json") || ctype.contains("xml") {
            bail!("no cover");
        }
        Ok((resp.bytes().await?.to_vec(), ctype))
    }

    /// Stream URL. `transcode` forces MP3 for codecs the local decoder can't handle.
    pub fn stream_url(&self, id: &str, transcode: bool) -> String {
        if transcode {
            self.url("stream", &[("id", id), ("format", "mp3"), ("maxBitRate", "320")])
        } else {
            self.url("stream", &[("id", id), ("format", "raw")])
        }
    }
}

fn from_list<T: for<'de> Deserialize<'de>>(v: Option<&Value>) -> Vec<T> {
    match v {
        Some(Value::Array(items)) => items.iter().filter_map(|i| serde_json::from_value(i.clone()).ok()).collect(),
        Some(obj @ Value::Object(_)) => serde_json::from_value(obj.clone()).ok().into_iter().collect(),
        _ => Vec::new(),
    }
}

pub fn normalise_server(s: &str) -> String {
    let s = s.trim().trim_end_matches('/');
    if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        format!("http://{s}")
    }
}
