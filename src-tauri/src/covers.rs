//! `cover://` URI scheme: serves album art from a disk cache, fetching from the server
//! on first use. Keeps auth tokens out of the webview and makes the Home screen render
//! instantly on later launches.

use crate::config::{app_dir, DirKind};
use crate::AppState;
use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

fn sanitise(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

fn mime_for(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'G', b'I', b'F', ..] => "image/gif",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        _ => "image/jpeg",
    }
}

fn respond(responder: UriSchemeResponder, status: StatusCode, body: Vec<u8>, mime: &str) {
    let resp = Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Cache-Control", "max-age=31536000, immutable")
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap();
    responder.respond(resp);
}

pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = ctx.app_handle().clone();
    // Path: /{id}/{size}
    let path = request.uri().path().trim_start_matches('/').to_string();
    tauri::async_runtime::spawn(async move {
        let mut parts = path.split('/');
        let id = urlencoding::decode(parts.next().unwrap_or("")).map(|s| s.into_owned()).unwrap_or_default();
        let size: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(300).clamp(32, 1200);
        if id.is_empty() {
            return respond(responder, StatusCode::NOT_FOUND, Vec::new(), "text/plain");
        }
        let dir = app_dir(DirKind::Cache).join("covers").join(size.to_string());
        let file = dir.join(sanitise(&id));
        if let Ok(bytes) = tokio::fs::read(&file).await {
            let mime = mime_for(&bytes);
            return respond(responder, StatusCode::OK, bytes, mime);
        }
        let client = app.state::<AppState>().client.read().clone();
        let Some(client) = client else {
            return respond(responder, StatusCode::UNAUTHORIZED, Vec::new(), "text/plain");
        };
        match client.cover_bytes(&id, size).await {
            Ok((bytes, ctype)) => {
                let _ = tokio::fs::create_dir_all(&dir).await;
                let _ = tokio::fs::write(&file, &bytes).await;
                respond(responder, StatusCode::OK, bytes, &ctype);
            }
            Err(_) => respond(responder, StatusCode::NOT_FOUND, Vec::new(), "text/plain"),
        }
    });
}
