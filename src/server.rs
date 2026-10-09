//! Server-only plumbing: in-memory page cache, cache headers, inlined CSS.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    io::Write,
    sync::{Arc, OnceLock, PoisonError, RwLock},
    time::Duration,
};

use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{Request, State},
    http::{
        header::{
            ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, ETAG,
            IF_NONE_MATCH, LOCATION, VARY,
        },
        HeaderMap, HeaderValue, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Router,
};
use leptos::config::{Env, LeptosOptions};
use tower::ServiceExt;

use crate::{
    calendar::{self, Feed},
    drive::{self, Library},
};

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
///
/// [`PageCache::refresh`] renders everything again (after each calendar refresh, so the
/// programs page follows the feed and the clock) and swaps the new set in at once. Requests
/// only take a read lock long enough to clone one `Arc`.
pub struct PageCache {
    /// Renders the cached paths; any other path must render the 404 page.
    renderer: Router,
    paths: &'static [&'static str],
    pages: RwLock<Arc<Pages>>,
}

#[derive(Default)]
struct Pages {
    by_path: HashMap<&'static str, Arc<Page>>,
    not_found: Option<Arc<Page>>,
}

struct Page {
    identity: Bytes,
    br: Bytes,
    gzip: Bytes,
    etag: HeaderValue,
}

/// A path no route matches, rendered as the 404 page.
const NOT_FOUND_PROBE: &str = "/__not_found__";

impl PageCache {
    /// Renders every path once through `renderer` (see [`PageCache::refresh`]).
    pub async fn new(renderer: Router, paths: &'static [&'static str]) -> Arc<Self> {
        let cache = Arc::new(Self { renderer, paths, pages: RwLock::default() });
        cache.refresh().await;
        cache
    }

    /// Renders every page again and swaps the new set in. A page whose HTML did not change keeps
    /// its compressed bodies and ETag. A page that fails to render keeps its previous version;
    /// one never rendered with 200 stays uncached, so it renders per request.
    pub async fn refresh(&self) {
        let old = self.pages.read().unwrap_or_else(PoisonError::into_inner).clone();
        let mut by_path = HashMap::with_capacity(self.paths.len());
        for &path in self.paths {
            let previous = old.by_path.get(path);
            match self.render(path, StatusCode::OK, previous).await {
                Some(page) => {
                    by_path.insert(path, page);
                }
                None => {
                    if let Some(previous) = previous {
                        by_path.insert(path, previous.clone());
                    }
                }
            }
        }
        let not_found = self
            .render(NOT_FOUND_PROBE, StatusCode::NOT_FOUND, old.not_found.as_ref())
            .await
            .or_else(|| old.not_found.clone());
        *self.pages.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(Pages { by_path, not_found });
    }

    async fn render(&self, path: &str, expected: StatusCode, previous: Option<&Arc<Page>>) -> Option<Arc<Page>> {
        let Ok(response) = self.renderer.clone().oneshot(get(path)).await;
        if response.status() != expected {
            leptos::logging::error!("page cache: {path} answered {}", response.status());
            return None;
        }
        let html = body(response)
            .await
            .map_err(|err| leptos::logging::error!("page cache: {path} failed to render: {err}"))
            .ok()?;
        let html = strip_nonces(html);
        if let Some(previous) = previous.filter(|p| p.identity == html) {
            return Some(previous.clone());
        }
        // Brotli 11 takes a while: keep it off the async worker threads.
        tokio::task::spawn_blocking(move || Arc::new(Page::new(html)))
            .await
            .map_err(|err| leptos::logging::error!("page cache: compressing {path} failed: {err}"))
            .ok()
    }

    fn page(&self, path: &str) -> Option<Arc<Page>> {
        self.pages.read().unwrap_or_else(PoisonError::into_inner).by_path.get(path).cloned()
    }

    fn not_found(&self) -> Option<Arc<Page>> {
        self.pages.read().unwrap_or_else(PoisonError::into_inner).not_found.clone()
    }
}

/// Background task: downloads the calendar every [`calendar::REFRESH_EVERY`] and renders the
/// cached pages again after every attempt, successful or not, so "upcoming" and "past" also
/// follow the clock.
pub fn spawn_calendar_refresh(mut feed: Feed, cache: Arc<PageCache>) {
    tokio::spawn(async move {
        let every = calendar::REFRESH_EVERY;
        let mut ticks = tokio::time::interval_at(tokio::time::Instant::now() + every, every);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            feed.refresh(Duration::from_secs(30)).await;
            refresh_pages(&cache).await;
        }
    });
}

/// Background task: syncs the Drive galleries right away and then every
/// [`drive::REFRESH_EVERY`]; the cached pages are rendered again whenever a gallery changed.
/// The first sync may take minutes (every photo is encoded); pages meanwhile show what the
/// cache directory already holds, or the Markdown images.
pub fn spawn_drive_refresh(mut library: Library, cache: Arc<PageCache>) {
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(drive::REFRESH_EVERY);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            if library.refresh().await {
                refresh_pages(&cache).await;
            }
        }
    });
}

/// In its own task, so a panicking render cannot end the caller's loop.
async fn refresh_pages(cache: &Arc<PageCache>) {
    let cache = cache.clone();
    if let Err(err) = tokio::spawn(async move { cache.refresh().await }).await {
        leptos::logging::error!("page cache: refresh failed: {err}");
    }
}

fn get(path: &str) -> Request {
    Request::get(path).body(Body::empty()).expect("valid request")
}

async fn body(response: Response) -> Result<Bytes, axum::Error> {
    to_bytes(response.into_body(), usize::MAX).await
}

/// leptos_axum gives every render a random CSP nonce (`<script nonce="…">` on its streaming
/// bootstrap scripts). No Content-Security-Policy is sent and one cached copy serves every
/// visitor, so drop it: otherwise every refresh yields new bytes and a new ETag, and browsers
/// could never revalidate with a 304.
fn strip_nonces(html: Bytes) -> Bytes {
    const TAG: &[u8] = b"<script nonce=\"";
    let Some(first) = html.windows(TAG.len()).position(|w| w == TAG) else {
        return html;
    };
    let mut out = Vec::with_capacity(html.len());
    out.extend_from_slice(&html[..first]);
    let mut rest = &html[first..];
    while let Some(at) = rest.windows(TAG.len()).position(|w| w == TAG) {
        out.extend_from_slice(&rest[..at]);
        let after = &rest[at + TAG.len()..];
        let Some(end) = after.iter().position(|&b| b == b'"') else { break };
        out.extend_from_slice(b"<script");
        rest = &after[end + 1..];
    }
    out.extend_from_slice(rest);
    out.into()
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
        let etag = HeaderValue::from_str(&format!("W/\"{:016x}\"", hasher.finish())).expect("ascii");

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
    match cache.page(req.uri().path()) {
        Some(page) => page.respond(&req, StatusCode::OK),
        None => next.run(req).await,
    }
}

/// Middleware: 301 for URLs of the old WordPress site and for paths with a trailing slash.
pub async fn redirects(req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let trimmed = if path.len() > 1 { path.trim_end_matches('/') } else { path };
    let moved = crate::site::REDIRECTS.iter().find(|(from, _)| *from == trimmed).map(|(_, to)| *to);
    // Only same-site targets: "//host" would be a protocol-relative redirect to another site.
    // Browsers also read "/\host" as "//host".
    let slash = (trimmed != path && trimmed.starts_with('/') && !trimmed[1..].starts_with(['/', '\\']))
        .then_some(trimmed);
    let Some(target) = moved.or(slash) else {
        return next.run(req).await;
    };
    let location = match req.uri().query() {
        Some(query) => format!("{target}?{query}"),
        None => target.to_owned(),
    };
    (StatusCode::MOVED_PERMANENTLY, [(LOCATION, location)]).into_response()
}

/// Fallback for every path that is neither a page nor a file: the cached 404 page.
pub async fn serve_not_found(State(cache): State<Arc<PageCache>>, req: Request) -> Response {
    match cache.not_found() {
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
            .any(|q| q.trim().parse::<f32>().is_ok_and(|q| q <= 0.0));
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
    /// /img, /drive and /fonts: immutable in production. Rule: a changed file gets a new file name.
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
    let value = if status == StatusCode::MOVED_PERMANENTLY {
        // Old WordPress URLs and trailing slashes: browsers may skip the hop on repeat visits,
        // but only for a day (production), so a corrected redirect map takes effect.
        policy.other.clone()
    } else if is_html || !(status.is_success() || status == StatusCode::NOT_MODIFIED) {
        NO_CACHE
    } else if path.starts_with("/pkg/") {
        policy.pkg
    } else if path.starts_with("/img/") || path.starts_with("/drive/") || path.starts_with("/fonts/") {
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
        assert_eq!(body(ok).await.unwrap(), page.br);

        let etag = page.etag.to_str().unwrap();
        let cached = page.respond(&request(Method::GET, &[("if-none-match", etag)]), StatusCode::OK);
        assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
        assert!(body(cached).await.unwrap().is_empty());

        let head = page.respond(&request(Method::HEAD, &[]), StatusCode::OK);
        assert_eq!(head.headers()[CONTENT_LENGTH], page.identity.len().to_string());
        assert!(body(head).await.unwrap().is_empty());

        // 404s never answer 304 and carry no validator.
        let missing = page.respond(&request(Method::GET, &[("if-none-match", etag)]), StatusCode::NOT_FOUND);
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert!(!missing.headers().contains_key(ETAG));
        assert_eq!(body(missing).await.unwrap(), page.identity);
    }

    #[tokio::test]
    async fn page_cache_refresh() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use axum::routing::get as route;

        static VERSION: AtomicUsize = AtomicUsize::new(1);
        let renderer = Router::new()
            .route("/valtozo", route(|| async { format!("v{}", VERSION.load(Ordering::SeqCst)) }))
            .route("/allando", route(|| async { "ugyanaz" }))
            .route("/hibas", route(|| async { StatusCode::INTERNAL_SERVER_ERROR }))
            .fallback(|| async { (StatusCode::NOT_FOUND, "nincs ilyen") });
        let cache = PageCache::new(renderer, &["/valtozo", "/allando", "/hibas"]).await;

        assert_eq!(cache.page("/valtozo").unwrap().identity, "v1");
        assert!(cache.page("/hibas").is_none(), "non-200 pages are not cached");
        assert_eq!(cache.not_found().unwrap().identity, "nincs ilyen");
        let (before, constant, not_found) =
            (cache.page("/valtozo").unwrap(), cache.page("/allando").unwrap(), cache.not_found().unwrap());

        VERSION.store(2, Ordering::SeqCst);
        cache.refresh().await;
        let after = cache.page("/valtozo").unwrap();
        assert_eq!(after.identity, "v2");
        assert_ne!(after.etag, before.etag);
        // Unchanged pages keep their compressed bodies and ETag.
        assert!(Arc::ptr_eq(&constant, &cache.page("/allando").unwrap()));
        assert!(Arc::ptr_eq(&not_found, &cache.not_found().unwrap()));

        // Served through the middleware.
        let app = Router::new()
            .fallback(|| async { "not cached" })
            .layer(axum::middleware::from_fn_with_state(cache.clone(), serve_cached));
        let response = app.oneshot(Request::get("/valtozo").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(body(response).await.unwrap(), "v2");
    }

    async fn redirect_of(uri: &str) -> Option<String> {
        use axum::{middleware::from_fn, routing::get};
        let app = Router::new().fallback(get(|| async { "ok" })).layer(from_fn(redirects));
        let response = app.oneshot(Request::get(uri).body(Body::empty()).unwrap()).await.unwrap();
        (response.status() == StatusCode::MOVED_PERMANENTLY)
            .then(|| response.headers()[LOCATION].to_str().unwrap().to_owned())
    }

    #[tokio::test]
    async fn redirects_old_urls_and_trailing_slashes() {
        assert_eq!(redirect_of("/notlikeus/").await.as_deref(), Some("/rolunk"));
        assert_eq!(redirect_of("/asztrofotoink").await.as_deref(), Some("/projektek/asztrofotok"));
        assert_eq!(redirect_of("/rolunk/?a=1").await.as_deref(), Some("/rolunk?a=1"));
        assert_eq!(redirect_of("/").await, None);
        assert_eq!(redirect_of("/rolunk").await, None);
        // never redirect off-site
        assert_eq!(redirect_of("//evil.example/").await, None);
        assert_eq!(redirect_of("///evil.example/").await, None);
        assert_eq!(redirect_of("/\\evil.example/").await, None);
        assert_eq!(redirect_of("/\\/evil.example/").await, None);
    }

    #[test]
    fn nonces_are_stripped() {
        let html = Bytes::from_static(b"<p>a</p><script nonce=\"Xy1-_\">A=[];</script><script nonce=\"Q\">B</script>");
        assert_eq!(&strip_nonces(html)[..], b"<p>a</p><script>A=[];</script><script>B</script>");
        let plain = Bytes::from_static(b"<p>no scripts</p>");
        assert_eq!(strip_nonces(plain.clone()), plain);
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
