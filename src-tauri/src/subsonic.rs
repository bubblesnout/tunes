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
    pub artist_id: Option<String>,
    /// OpenSubsonic: every credited track artist (Navidrome 0.55+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub cover_art: Option<String>,
    #[serde(default)]
    pub album_count: Option<u32>,
    /// OpenSubsonic: roles this artist has in the library ("albumartist", "artist", "composer"...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Index letter from the server ("A", "#"...), set by `artists()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtistRef {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    #[serde(flatten)]
    pub artist: Artist,
    #[serde(default)]
    pub album: Vec<Album>,
}

/// All artists, already grouped and sorted by the server (which honours its ignored articles).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtistIndex {
    pub artists: Vec<Artist>,
    /// Space-separated articles the server ignores when sorting ("The El La ...").
    pub ignored_articles: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    #[serde(default, alias = "value")]
    pub name: String,
    #[serde(default)]
    pub song_count: Option<u32>,
    #[serde(default)]
    pub album_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Starred {
    #[serde(default)]
    pub artist: Vec<Artist>,
    #[serde(default)]
    pub album: Vec<Album>,
    #[serde(default)]
    pub song: Vec<Song>,
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

    /// Every album matching `kind` (plus `extra` params), paging through the server's 500 limit.
    pub async fn all_albums(&self, kind: &str, extra: &[(&str, &str)]) -> Result<Vec<Album>> {
        const PAGE: usize = 500;
        let mut out = Vec::new();
        loop {
            let (size, offset) = (PAGE.to_string(), out.len().to_string());
            let mut params = vec![("type", kind), ("size", size.as_str()), ("offset", offset.as_str())];
            params.extend_from_slice(extra);
            let root = self.call("getAlbumList2", &params).await?;
            let page: Vec<Album> = from_list(root.pointer("/albumList2/album"));
            let n = page.len();
            out.extend(page);
            if n < PAGE || out.len() >= 100_000 {
                return Ok(out);
            }
        }
    }

    pub async fn artists(&self) -> Result<ArtistIndex> {
        let root = self.call("getArtists", &[]).await?;
        let ignored_articles =
            root.pointer("/artists/ignoredArticles").and_then(Value::as_str).unwrap_or("").to_string();
        let mut artists = Vec::new();
        if let Some(Value::Array(index)) = root.pointer("/artists/index") {
            for group in index {
                let letter = group.get("name").and_then(Value::as_str).unwrap_or("#").to_string();
                for mut a in from_list::<Artist>(group.get("artist")) {
                    a.letter = Some(letter.clone());
                    artists.push(a);
                }
            }
        }
        Ok(ArtistIndex { artists, ignored_articles })
    }

    /// Every artist with any credit, including track-only artists, which `getArtists` leaves out
    /// (Navidrome only lists album artists there). `search3` with the query `""` matches everything.
    pub async fn all_artists(&self) -> Result<Vec<Artist>> {
        const PAGE: usize = 500;
        let mut out: Vec<Artist> = Vec::new();
        loop {
            let (count, offset) = (PAGE.to_string(), out.len().to_string());
            let root = self
                .call(
                    "search3",
                    &[("query", "\"\""), ("artistCount", &count), ("artistOffset", &offset), ("albumCount", "0"), ("songCount", "0")],
                )
                .await?;
            let page: Vec<Artist> = from_list(root.pointer("/searchResult3/artist"));
            let n = page.len();
            out.extend(page);
            if n < PAGE || out.len() >= 100_000 {
                return Ok(out);
            }
        }
    }

    /// Songs crediting this artist, found by searching for their name and keeping exact id matches.
    /// Used for "Appears On", since `getArtist` only lists albums where they're the album artist.
    pub async fn artist_songs(&self, id: &str, name: &str) -> Result<Vec<Song>> {
        const PAGE: usize = 500;
        let mut matched = Vec::new();
        let mut offset = 0;
        while offset < PAGE * 4 {
            let (count, off) = (PAGE.to_string(), offset.to_string());
            let root = self
                .call(
                    "search3",
                    &[("query", name), ("songCount", &count), ("songOffset", &off), ("artistCount", "0"), ("albumCount", "0")],
                )
                .await?;
            let page: Vec<Song> = from_list(root.pointer("/searchResult3/song"));
            let n = page.len();
            matched.extend(page.into_iter().filter(|s| credits(s, id)));
            if n < PAGE {
                break;
            }
            offset += PAGE;
        }
        Ok(matched)
    }

    pub async fn artist(&self, id: &str) -> Result<ArtistDetail> {
        let root = self.call("getArtist", &[("id", id)]).await?;
        Ok(serde_json::from_value(root.get("artist").cloned().unwrap_or_default())?)
    }

    pub async fn genres(&self) -> Result<Vec<Genre>> {
        let root = self.call("getGenres", &[]).await?;
        Ok(from_list(root.pointer("/genres/genre")))
    }

    pub async fn starred(&self) -> Result<Starred> {
        let root = self.call("getStarred2", &[]).await?;
        Ok(serde_json::from_value(root.get("starred2").cloned().unwrap_or_default())?)
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

/// Whether a song credits the artist `id` (OpenSubsonic `artists`, or the legacy single `artistId`).
fn credits(s: &Song, id: &str) -> bool {
    s.artists.iter().any(|a| a.id == id) || s.artist_id.as_deref() == Some(id)
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
