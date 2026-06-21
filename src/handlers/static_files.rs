//! Static file serving for the SPA frontend.
//!
//! Serves files from `./mainpage/dist/` with path-traversal protection.
//! If a file is not found, falls back to `index.html` for SPA routing.

use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::extract::OriginalUri;

use crate::error::SubconvError;

/// Root directory for static files.
const STATIC_DIR: &str = "./mainpage/dist";

/// Serve `index.html` for the root path.
pub async fn serve_index() -> Result<Response, SubconvError> {
    let path = Path::new(STATIC_DIR).join("index.html");
    if path.is_file() {
        return serve_file(&path).await;
    }
    Ok(StatusCode::NOT_FOUND.into_response())
}

/// Serve a static file, with SPA fallback to `index.html`.
///
/// This is the fallback handler for `GET /*path`.
/// Uses `OriginalUri` to get the full request path, since Axum 0.8's
/// fallback routes don't reliably populate the `Path` extractor.
pub async fn serve_static(
    OriginalUri(uri): OriginalUri,
) -> Result<Response, SubconvError> {
    // Strip leading "/" to get a relative path under STATIC_DIR.
    let raw_path = uri.path().trim_start_matches('/');
    let resolved = resolve_safe_path(raw_path)?;

    match resolved {
        Some(file_path) if file_path.is_file() => serve_file(&file_path).await,
        _ => {
            // SPA fallback: serve index.html for any non-file path.
            let index = Path::new(STATIC_DIR).join("index.html");
            if index.is_file() {
                serve_file(&index).await
            } else {
                Ok(StatusCode::NOT_FOUND.into_response())
            }
        }
    }
}

/// Resolve a request path to a filesystem path, guarding against traversal.
///
/// Returns `None` if the path would escape the static root directory.
fn resolve_safe_path(request_path: &str) -> Result<Option<PathBuf>, SubconvError> {
    let static_root = Path::new(STATIC_DIR);

    // Canonicalize the root if it exists; otherwise use it as-is.
    let root = if static_root.is_dir() {
        static_root
            .canonicalize()
            .map_err(SubconvError::Io)?
    } else {
        // Static dir doesn't exist; nothing to serve.
        return Ok(None);
    };

    let candidate = static_root.join(request_path);

    // Canonicalize the candidate. If the file doesn't exist yet, we can't
    // canonicalize, so we do a prefix check on the joined path instead.
    if let Ok(canonical) = candidate.canonicalize() {
        // Ensure the canonical path starts with the root.
        if canonical.starts_with(&root) {
            return Ok(Some(canonical));
        }
        // Path traversal detected.
        return Ok(None);
    }

    // File doesn't exist on disk. Check the non-canonicalized path for
    // obvious traversal patterns before returning it.
    // This handles the SPA fallback case where the path is a client-side route.
    let normalized = normalize_path(request_path);
    let candidate = static_root.join(&normalized);

    // Verify the joined path is still under the root.
    // Use starts_with on the display form as a secondary check.
    if candidate.starts_with(static_root) {
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
        // Not a file; let the caller handle SPA fallback.
        return Ok(None);
    }

    Ok(None)
}

/// Normalize a path by collapsing `..` and `.` segments.
///
/// This is a simple defense-in-depth measure. The primary protection is
/// the `canonicalize` + `starts_with` check above.
fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(segment),
        }
    }
    parts.join("/")
}

/// Serve a file with an appropriate Content-Type header.
async fn serve_file(path: &Path) -> Result<Response, SubconvError> {
    let bytes = tokio::fs::read(path).await.map_err(SubconvError::Io)?;

    let content_type = guess_content_type(path);

    let mut response = bytes.into_response();
    response.headers_mut().insert(
        "Content-Type",
        axum::http::HeaderValue::from_str(content_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );

    Ok(response)
}

/// Guess MIME type from file extension.
fn guess_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("webp") => "image/webp",
        Some("webm") => "video/webm",
        Some("mp4") => "video/mp4",
        Some("wasm") => "application/wasm",
        Some("map") => "application/json",
        _ => "application/octet-stream",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_collapses_dots() {
        assert_eq!(normalize_path("a/b/../c"), "a/c");
        assert_eq!(normalize_path("a/./b"), "a/b");
        assert_eq!(normalize_path("../etc/passwd"), "etc/passwd");
        assert_eq!(normalize_path("a/../../b"), "b");
    }

    #[test]
    fn normalize_path_empty_and_root() {
        assert_eq!(normalize_path(""), "");
        assert_eq!(normalize_path("/"), "");
    }

    #[test]
    fn guess_content_type_common_extensions() {
        assert_eq!(
            guess_content_type(Path::new("style.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            guess_content_type(Path::new("app.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            guess_content_type(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(guess_content_type(Path::new("logo.png")), "image/png");
        assert_eq!(
            guess_content_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }
}
