use leptos::prelude::*;
use leptos_meta::{provide_meta_context, HashedStylesheet, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes, A},
    path,
};

use crate::pages::{About, Community, Home, NotFound};

/// Pages rendered once at startup and then served from memory, precompressed
/// (see `server::page_cache`). Only list pages whose HTML is the same for every request.
pub const STATIC_PAGES: &[&str] = &["/", "/kozosseg", "/rolunk"];

/// Pages that contain at least one `#[island]`. Only these load the WASM bundle;
/// every other page ships zero JavaScript.
const ISLAND_PAGES: &[&str] = &[];

/// Prefetch same-origin links on hover, prerender on pointerdown (Chromium; ignored elsewhere).
const SPECULATION_RULES: &str = r#"{"prefetch":[{"where":{"href_matches":"/*"},"eagerness":"moderate"}],"prerender":[{"where":{"href_matches":"/*"},"eagerness":"conservative"}]}"#;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let islands = needs_islands();

    view! {
        <!DOCTYPE html>
        <html lang="hu">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#000000"/>
                <link rel="icon" href="/favicon.ico" sizes="32x32"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <link rel="apple-touch-icon" href="/apple-touch-icon.png"/>
                // Mirrors the media queries of body::before in style/main.css exactly (range
                // syntax: no gaps at fractional widths), so one background is fetched, early.
                // Browsers without AVIF skip these and use image-set().
                <link rel="preload" r#as="image" r#type="image/avif" fetchpriority="high"
                    href="/img/home_bg-1920.avif" media="(width > 1280px)"/>
                <link rel="preload" r#as="image" r#type="image/avif" fetchpriority="high"
                    href="/img/home_bg-1280.avif"
                    media="(900px < width <= 1280px), (width <= 900px) and (orientation: landscape)"/>
                <link rel="preload" r#as="image" r#type="image/avif" fetchpriority="high"
                    href="/img/home_bg-portrait-720.avif" media="(width <= 900px) and (orientation: portrait)"/>
                <Styles options=options.clone()/>
                <script type="speculationrules" inner_html=SPECULATION_RULES></script>
                <AutoReload options=options.clone()/>
                {islands.then(|| view! { <HydrationScripts options islands=true/> })}
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Decides per request whether this page needs the WASM bundle.
fn needs_islands() -> bool {
    #[cfg(feature = "ssr")]
    {
        let path = use_context::<axum::http::request::Parts>()
            .map(|parts| parts.uri.path().to_owned())
            .unwrap_or_default();
        let islands = ISLAND_PAGES.contains(&path.as_str());
        if !islands {
            // <HydrationScripts islands=true/> normally does this; without it every
            // resource would serialize its data into the HTML for a hydration that never runs.
            if let Some(sc) = Owner::current_shared_context() {
                sc.set_is_hydrating(false);
            }
        }
        islands
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = ISLAND_PAGES;
        false
    }
}

/// Production: the (already minified) stylesheet is inlined, so first paint needs no extra
/// request. Development: a <link> that cargo-leptos can hot-reload.
#[component]
fn Styles(options: LeptosOptions) -> impl IntoView {
    #[cfg(feature = "ssr")]
    if crate::server::is_production(&options) {
        if let Some(css) = crate::server::inline_css(&options) {
            return view! { <style inner_html=css></style> }.into_any();
        }
    }
    view! { <HashedStylesheet options id="leptos"/> }.into_any()
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title formatter=|page: String| {
            if page.is_empty() { "Kozmosz".to_owned() } else { format!("Kozmosz – {page}") }
        }/>
        <Router>
            <Header/>
            <main>
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=Home/>
                    <Route path=path!("/kozosseg") view=Community/>
                    <Route path=path!("/rolunk") view=About/>
                </Routes>
            </main>
            <Footer/>
        </Router>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="site-nav">
            <nav class="container nav">
                <a href="/" class="brand" aria-label="Kozmosz – kezdőlap">
                    <img src="/img/kozmosz-mark.svg" alt="" width="40" height="40"/>
                    <span>"Kozmosz"</span>
                </a>
                <ul class="nav-links">
                    <li><A href="/kozosseg">"Közösség"</A></li>
                    <li><A href="/rolunk">"Rólunk"</A></li>
                </ul>
            </nav>
        </header>
    }
}

#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <div class="container">"© Kozmosz közösség"</div>
        </footer>
    }
}
