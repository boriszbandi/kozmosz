//! Server-only plumbing: in-memory page cache, cache headers, inlined CSS.

use std::{
    collections::HashMap,
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
    io::Write,
    sync::{Arc, OnceLock},
};

use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{Request, State},
    http::{
        header::{
            ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG,
            IF_NONE_MATCH, VARY,
        },
        HeaderMap, HeaderValue, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Router,
};
use leptos::config::{Env, LeptosOptions};
use tower::ServiceExt;

const HTML: HeaderValue = HeaderValue::from_static("text/html; charset=utf-8");
const NO_CACHE: HeaderValue = HeaderValue::from_static("no-cache");
const IMMUTABLE: HeaderValue = HeaderValue::from_static("public, max-age=31536000, immutable");
const ONE_DAY: HeaderValue = HeaderValue::from_static("public, max-age=86400");

/// Release builds always behave as production. cargo-leptos never sets LEPTOS_ENV, so it
/// defaults to DEV even under `cargo leptos serve --release`.
pub fn is_production(options: &LeptosOptions) -> bool {
    options.env == Env::PROD || !cfg!(debug_assertions)
}

/// The minified stylesheet cargo-leptos wrote to `<site-root>/<pkg>/<name>[.<hash>].css`, read
/// once. The hash comes from hash.txt next to the binary, exactly like `HashedStylesheet`, so a
/// stale file left over from an earlier deploy is never picked. `None` falls back to the <link>.
pub fn inline_css(options: &LeptosOptions) -> Option<&'static str> {
    static CSS: OnceLock<Option<String>> = OnceLock::new();
    CSS.get_or_init(|| {
        let mut name = options.output_name.to_string();
        if options.hash_files {
            let hash_file = std::env::current_exe().ok()?.parent()?.join(&*options.hash_file);
            let hashes = std::fs::read_to_string(hash_file).ok()?;
            let hash = hashes.lines().find_map(|line| line.trim().strip_prefix("css:"))?;
            name = format!("{name}.{}", hash.trim());
        }
        let path = format!("{}/{}/{name}.css", options.site_root, options.site_pkg_dir);
        std::fs::read_to_string(&path)
            .map_err(|err| leptos::logging::error!("inline css: cannot read {path}: {err}"))
            .ok()
    })
    .as_deref()
}

/// Rendered HTML kept in memory in every encoding we serve: the static pages, plus the
/// 404 page (its HTML does not depend on the requested path).
pub struct PageCache {
    pages: HashMap<&'static str, Page>,
    not_found: Option<Page>,
}

struct Page {
    identity: Bytes,
    br: Bytes,
    gzip: Bytes,
    etag: HeaderValue,
}

impl PageCache {
    /// Renders every path once through `app`, and the 404 page through `render_not_found`.
    /// Pages that do not answer 200 are left out (and logged), so they render normally.
    pub async fn warm<F, Fut>(app: Router, paths: &[&'static str], render_not_found: F) -> Arc<Self>
    where
        F: FnOnce(Request) -> Fut,
        Fut: Future<Output = Response>,
    {
        let mut pages = HashMap::with_capacity(paths.len());
        for &path in paths {
            let response = app.clone().oneshot(get(path)).await.expect("infallible");
            if response.status() != StatusCode::OK {
                leptos::logging::error!("page cache: {path} answered {}, not cached", response.status());
                continue;
            }
            pages.insert(path, Page::new(body(response).await));
        }
        // Any path no route matches renders the NotFound page.
        let response = render_not_found(get("/__not_found__")).await;
        let not_found = Some(Page::new(body(response).await));
        Arc::new(Self { pages, not_found })
    }
}

fn get(path: &str) -> Request {
    Request::get(path).body(Body::empty()).expect("valid request")
}

async fn body(response: Response) -> Bytes {
    to_bytes(response.into_body(), usize::MAX).await.expect("in-memory body")
}

impl Page {
    fn new(html: Bytes) -> Self {
        let mut br = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut br, 4096, 11, 22);
            writer.write_all(&html).expect("in-memory write");
        }
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        gzip.write_all(&html).expect("in-memory write");
        let gzip = gzip.finish().expect("in-memory write");

        let mut hasher = DefaultHasher::new();
        html.hash(&mut hasher);
        // Weak: the same tag covers every content-encoding of this page.
        let etag = HeaderValue::from_str(&format!("W/\"{:016x}\"", hasher.finish()))
            .expect("ascii");

        Self { identity: html, br: br.into(), gzip: gzip.into(), etag }
    }

    /// Builds the response for `req`: best encoding the client accepts, body only for GET.
    /// 200 pages carry an ETag and answer If-None-Match with 304.
    fn respond(&self, req: &Request, status: StatusCode) -> Response {
        let headers = req.headers();
        let mut response = Response::new(Body::empty());
        *response.status_mut() = status;
        let out = response.headers_mut();
        out.insert(CACHE_CONTROL, NO_CACHE);
        out.insert(VARY, HeaderValue::from_static("accept-encoding"));

        if status == StatusCode::OK {
            out.insert(ETAG, self.etag.clone());
            if etag_matches(headers, &self.etag) {
                *response.status_mut() = StatusCode::NOT_MODIFIED;
                return response;
            }
        }

        let accept = headers.get(ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
        let (body, encoding) = if accepts(accept, "br") {
            (&self.br, Some("br"))
        } else if accepts(accept, "gzip") {
            (&self.gzip, Some("gzip"))
        } else {
            (&self.identity, None)
        };
        out.insert(CONTENT_TYPE, HTML);
        out.insert(CONTENT_LENGTH, HeaderValue::from(body.len()));
        if let Some(encoding) = encoding {
            out.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
        }
        if req.method() != Method::HEAD {
            *response.body_mut() = Body::from(body.clone());
        }
        response
    }
}

/// Middleware: answers GET/HEAD for cached pages straight from memory.
pub async fn serve_cached(State(cache): State<Arc<PageCache>>, req: Request, next: Next) -> Response {
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return next.run(req).await;
    }
    match cache.pages.get(req.uri().path()) {
        Some(page) => page.respond(&req, StatusCode::OK),
        None => next.run(req).await,
    }
}

/// Fallback for every path that is neither a page nor a file: the cached 404 page.
pub async fn serve_not_found(State(cache): State<Arc<PageCache>>, req: Request) -> Response {
    match &cache.not_found {
        Some(page) => page.respond(&req, StatusCode::NOT_FOUND),
        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}

fn etag_matches(headers: &HeaderMap, etag: &HeaderValue) -> bool {
    let Ok(etag) = etag.to_str() else { return false };
    let opaque = etag.trim_start_matches("W/");
    headers.get_all(IF_NONE_MATCH).iter().filter_map(|v| v.to_str().ok()).any(|list| {
        list.split(',')
            .map(|tag| tag.trim())
            .any(|tag| tag == "*" || tag.trim_start_matches("W/") == opaque)
    })
}

/// Whether `coding` is acceptable per an Accept-Encoding header (explicitly or via `*`).
fn accepts(header: &str, coding: &str) -> bool {
    let mut wildcard = false;
    for item in header.split(',') {
        let mut parts = item.split(';');
        let name = parts.next().unwrap_or("").trim();
        let q_zero = parts
            .filter_map(|p| p.trim().strip_prefix("q="))
            .any(|q| q.trim().parse::<f32>().map_or(false, |q| q <= 0.0));
        if name.eq_ignore_ascii_case(coding) {
            return !q_zero;
        }
        if name == "*" {
            wildcard = !q_zero;
        }
    }
    wildcard
}

/// How long browsers may keep static files.
#[derive(Clone)]
pub struct CachePolicy {
    /// /pkg: JS, WASM, CSS. Immutable only when file names carry a content hash.
    pkg: HeaderValue,
    /// /img: immutable in production. Rule: a changed image gets a new file name.
    img: HeaderValue,
    /// Other files under the site root (favicon, robots.txt).
    other: HeaderValue,
}

impl CachePolicy {
    pub fn new(options: &LeptosOptions) -> Self {
        let prod = is_production(options);
        Self {
            pkg: if options.hash_files { IMMUTABLE } else { NO_CACHE },
            img: if prod { IMMUTABLE } else { NO_CACHE },
            other: if prod { ONE_DAY } else { NO_CACHE },
        }
    }
}

/// Middleware: fills in Cache-Control where the handler did not set one.
/// HTML and errors are always revalidated; static files follow [`CachePolicy`].
pub async fn cache_control(State(policy): State<CachePolicy>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_owned();
    let mut response = next.run(req).await;
    if response.headers().contains_key(CACHE_CONTROL) {
        return response;
    }
    let status = response.status();
    let is_html = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"));
    let value = if is_html || !(status.is_success() || status == StatusCode::NOT_MODIFIED) {
        NO_CACHE
    } else if path.starts_with("/pkg/") {
        policy.pkg
    } else if path.starts_with("/img/") {
        policy.img
    } else {
        policy.other
    };
    response.headers_mut().insert(CACHE_CONTROL, value);
    response
}

/// Resolves on Ctrl+C, SIGTERM (Unix) or CTRL_BREAK (Windows, sent by cargo-leptos on rebuild).
pub async fn shutdown_signal() {
    #[cfg(unix)]
    let other = async {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate()).expect("SIGTERM handler").recv().await;
    };
    #[cfg(windows)]
    let other = async {
        tokio::signal::windows::ctrl_break().expect("CTRL_BREAK handler").recv().await;
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = other => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_encoding() {
        assert!(accepts("gzip, deflate, br, zstd", "br"));
        assert!(accepts("gzip;q=1.0, br;q=0.5", "br"));
        assert!(!accepts("gzip, br;q=0", "br"));
        assert!(!accepts("gzip", "br"));
        assert!(accepts("*", "br"));
        assert!(!accepts("*;q=0", "gzip"));
        assert!(!accepts("", "gzip"));
        assert!(accepts("BR", "br"));
    }

    fn request(method: Method, headers: &[(&str, &str)]) -> Request {
        let mut builder = Request::builder().method(method).uri("/");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn page_responses() {
        let page = Page::new(Bytes::from_static(b"<!DOCTYPE html><p>szia</p>"));

        let ok = page.respond(&request(Method::GET, &[("accept-encoding", "gzip, br")]), StatusCode::OK);
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[CONTENT_ENCODING], "br");
        assert_eq!(ok.headers()[ETAG], page.etag);
        assert_eq!(body(ok).await, page.br);

        let etag = page.etag.to_str().unwrap();
        let cached = page.respond(&request(Method::GET, &[("if-none-match", etag)]), StatusCode::OK);
        assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
        assert!(body(cached).await.is_empty());

        let head = page.respond(&request(Method::HEAD, &[]), StatusCode::OK);
        assert_eq!(head.headers()[CONTENT_LENGTH], page.identity.len().to_string());
        assert!(body(head).await.is_empty());

        // 404s never answer 304 and carry no validator.
        let missing = page.respond(&request(Method::GET, &[("if-none-match", etag)]), StatusCode::NOT_FOUND);
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert!(!missing.headers().contains_key(ETAG));
        assert_eq!(body(missing).await, page.identity);
    }

    #[test]
    fn if_none_match() {
        let etag = HeaderValue::from_static("W/\"abc\"");
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"x\", W/\"abc\""));
        assert!(etag_matches(&headers, &etag));
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"abc\""));
        assert!(etag_matches(&headers, &etag));
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"other\""));
        assert!(!etag_matches(&headers, &etag));
    }
}
