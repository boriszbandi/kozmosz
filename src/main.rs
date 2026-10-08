#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::time::Duration;

    use axum::{
        handler::Handler,
        http::{header, HeaderValue},
        middleware::{from_fn, from_fn_with_state},
        serve::ListenerExt,
        Router,
    };
    use kozmosz::{
        app::{shell, App, STATIC_PAGES},
        calendar::Feed,
        drive::Library,
        server::{
            cache_control, redirects, serve_cached, serve_not_found, shutdown_signal,
            spawn_calendar_refresh, spawn_drive_refresh, CachePolicy, PageCache,
        },
    };
    use leptos::{logging::log, prelude::*};
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tower_http::{
        compression::{
            predicate::{DefaultPredicate, NotForContentType, Predicate},
            CompressionLayer,
        },
        services::ServeDir,
        set_header::SetResponseHeaderLayer,
    };

    let conf = get_configuration(None).expect("leptos configuration");
    let options = conf.leptos_options;
    let addr = options.site_addr;
    let routes = generate_route_list(App);

    let pages = Router::new()
        .leptos_routes(&options, routes, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .with_state(options.clone());

    // The programs page lists events from the club's Google Calendar: download it before the
    // pages are rendered (giving up after 5 s), then every 10 minutes in the background.
    let mut feed = Feed::new();
    feed.refresh(Duration::from_secs(5)).await;
    // Project galleries: the photos already encoded in the cache directory are shown at once;
    // the Drive folders are synced in the background (spawn_drive_refresh below).
    let library = Library::open();
    let photos = library.files_dir();

    // Renders the cached pages; any other path renders the 404 page.
    let renderer = pages.clone().fallback({
        let options = options.clone();
        move |req: axum::extract::Request| {
            let (context, app) = (options.clone(), options.clone());
            leptos_axum::render_app_to_stream_in_order_with_context(
                move || provide_context(context.clone()),
                move || shell(app.clone()),
            )(req)
        }
    });
    let cache = PageCache::new(renderer, STATIC_PAGES).await;
    spawn_calendar_refresh(feed, cache.clone());
    spawn_drive_refresh(library, cache.clone());

    // Static files: precompressed .br/.gz when cargo-leptos made them (--precompress), except
    // /img and /fonts, whose formats are already compressed. Anything else gets the cached 404 page.
    let not_found = serve_not_found.with_state(cache.clone());
    // No index.html / add-slash redirects for directories: they would bounce against the
    // trailing-slash redirect. Directories simply get the 404 page.
    let images = ServeDir::new(&*options.site_root)
        .append_index_html_on_directories(false)
        .fallback(not_found.clone());
    let drive = ServeDir::new(photos).append_index_html_on_directories(false).fallback(not_found.clone());
    let files = ServeDir::new(&*options.site_root)
        .append_index_html_on_directories(false)
        .precompressed_br()
        .precompressed_gzip()
        .fallback(not_found);

    // Layers run outermost-last: compression wraps everything, but skips responses that
    // already carry Content-Encoding (cached pages, precompressed files), images and fonts.
    let app = Router::new()
        .merge(pages)
        .route_service("/img/{*path}", images.clone())
        .route_service("/fonts/{*path}", images)
        // nest_service strips "/drive": the cache directory holds <album>/<file> directly.
        .nest_service("/drive", drive)
        .fallback_service(files)
        .layer(from_fn_with_state(cache, serve_cached))
        .layer(from_fn(redirects))
        .layer(from_fn_with_state(CachePolicy::new(&options), cache_control))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(CompressionLayer::new().compress_when(
            DefaultPredicate::new().and(NotForContentType::const_new("font/")),
        ));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind site address")
        .tap_io(|tcp| {
            let _ = tcp.set_nodelay(true);
        });
    log!("listening on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // The client is the WASM library (see lib.rs); there is no client-side main.
}
