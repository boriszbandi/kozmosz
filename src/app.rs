use leptos::prelude::*;
use leptos_meta::{provide_meta_context, HashedStylesheet, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::{
    layout::{Footer, Header},
    pages::{About, Contact, Home, Lecturers, NotFound, PhotoPage, Programs, Project, Projects},
    site,
};

/// Pages rendered at startup and then served from memory, precompressed (see
/// `server::PageCache`). They are rendered again after every calendar refresh (every 10
/// minutes), which keeps /programjaink current. Only list pages whose HTML is the same for every
/// request.
pub const STATIC_PAGES: &[&str] = &[
    "/",
    "/rolunk",
    "/elerhetoseg",
    "/programjaink",
    "/eloadoink",
    "/projektek",
    "/projektek/asztrofotok",
    "/projektek/isstv",
    "/projektek/idojaras-muhold",
];

/// Pages that contain at least one `#[island]`. Only these load the WASM bundle;
/// every other page ships zero JavaScript.
const ISLAND_PAGES: &[&str] = &[];

/// Prefetch same-origin page links on hover, prerender on pointerdown (Chromium; ignored
/// elsewhere). On photo pages the previous and next photo pages are prefetched right away, and
/// photo pages are prerendered on hover, so the tile-to-photo morph lands on a decoded image.
/// /img/ and /drive/ are excluded: some gallery links point at full-size JPEGs.
const SPECULATION_RULES: &str = r#"{"prefetch":[{"where":{"selector_matches":"a[rel=next], a[rel=prev]"},"eagerness":"eager"},{"where":{"and":[{"href_matches":"/*"},{"not":{"href_matches":["/img/*","/drive/*"]}}]},"eagerness":"moderate"}],"prerender":[{"where":{"or":[{"href_matches":"/projektek/*/kep/*"},{"selector_matches":"a[rel=next], a[rel=prev]"}]},"eagerness":"moderate"},{"where":{"and":[{"href_matches":"/*"},{"not":{"href_matches":["/img/*","/drive/*"]}}]},"eagerness":"conservative"}]}"#;

/// The only inline script (approved by the user 2026-10-08): the mobile menu is a <details>, and a
/// page restored from the back/forward cache would come back with the menu still open. Closing it
/// on pagehide stores the closed state, so nothing flickers on restore.
const CLOSE_MENU_ON_LEAVE: &str =
    r#"addEventListener("pagehide",()=>{for(const d of document.querySelectorAll(".nav-mobile details[open]"))d.open=false})"#;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let islands = needs_islands();

    view! {
        <!DOCTYPE html>
        <html lang="hu">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                // Title, description, canonical, Open Graph and image preloads: first, so the
                // preload scanner sees the hero image before the inline stylesheet.
                <MetaTags/>
                <meta name="theme-color" content="#0a0c0f"/>
                <link rel="icon" href="/favicon.ico" sizes="32x32"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <link rel="apple-touch-icon" href="/apple-touch-icon.png"/>
                // Heading font (Urbanist, subset for Hungarian): fetched in parallel with the HTML.
                <link rel="preload" href="/fonts/urbanist-hu.woff2" r#as="font" r#type="font/woff2" crossorigin=""/>
                <Styles options=options.clone()/>
                <script type="speculationrules" inner_html=SPECULATION_RULES></script>
                <script inner_html=CLOSE_MENU_ON_LEAVE></script>
                <AutoReload options=options.clone()/>
                {islands.then(|| view! { <HydrationScripts options islands=true/> })}
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
            if page.is_empty() { site::NAME.to_owned() } else { format!("{page} | {}", site::NAME) }
        }/>
        <Router>
            <Header/>
            <main id="tartalom">
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=Home/>
                    <Route path=path!("/rolunk") view=About/>
                    <Route path=path!("/elerhetoseg") view=Contact/>
                    <Route path=path!("/programjaink") view=Programs/>
                    <Route path=path!("/eloadoink") view=Lecturers/>
                    <Route path=path!("/projektek") view=Projects/>
                    <Route
                        path=path!("/projektek/asztrofotok")
                        view=|| view! { <Project name="projektek-asztrofotok"/> }
                    />
                    <Route path=path!("/projektek/isstv") view=|| view! { <Project name="projektek-isstv"/> }/>
                    <Route
                        path=path!("/projektek/idojaras-muhold")
                        view=|| view! { <Project name="projektek-idojaras-muhold"/> }
                    />
                    // One gallery photo, large (rendered per request: the galleries follow Drive).
                    <Route path=path!("/projektek/:project/kep/:slug") view=PhotoPage/>
                </Routes>
            </main>
            <Footer/>
        </Router>
    }
}
