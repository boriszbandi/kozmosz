//! The admins' dashboard (src/admin.rs lets only signed-in admins in): a sidebar with its
//! sections next to the page. The attendance sheets are server-only code (src/attendance.rs);
//! their routes here just call it, and the browser build never runs them.

use leptos::prelude::*;
use leptos_meta::{Meta, Title};
#[cfg(feature = "ssr")]
use leptos_router::hooks::{use_params_map, use_query_map};
use pulldown_cmark::{html, Parser};

use super::PageHeader;

/// Text of the links section: Markdown in /etc/kozmosz/admin.md on the server, read on every
/// request (an edit shows at once) and kept out of the public repository. Without the file the
/// section is not shown.
const LINKS: &str = "/etc/kozmosz/admin.md";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Sheets,
    Links,
}

/// The dashboard frame: sections and sign-out in the sidebar, the page beside it.
#[component]
pub fn AdminLayout(#[prop(into)] title: String, section: Section, children: Children) -> impl IntoView {
    let current = |s: Section| (s == section).then_some("page");
    view! {
        <Title text=title.clone()/>
        <Meta name="robots" content="noindex"/>
        <div class="container admin">
            <aside class="admin-side">
                <nav class="admin-nav" aria-label="Admin">
                    <a href="/admin" aria-current=current(Section::Sheets)>"Jelenléti ívek"</a>
                    {std::fs::metadata(LINKS).is_ok().then(|| view! {
                        <a href="/admin/linkek" aria-current=current(Section::Links)>"Linkek"</a>
                    })}
                </nav>
                <form method="post" action="/kilepes">
                    <button class="button button-line" type="submit">"Kilépés"</button>
                </form>
            </aside>
            <div class="admin-main">
                <h1 class="admin-title">{title}</h1>
                {children()}
            </div>
        </div>
    }
}

/// /admin: the attendance sheets of the events.
#[component]
pub fn AdminPage() -> impl IntoView {
    #[cfg(feature = "ssr")]
    let page = crate::attendance::events_page();
    #[cfg(not(feature = "ssr"))]
    let page = ();
    page
}

/// /admin/jelenlet/<key>: one event's sheet with its QR code.
#[component]
pub fn AdminEvent() -> impl IntoView {
    #[cfg(feature = "ssr")]
    let page = crate::attendance::event_page(&use_params_map().with_untracked(|p| p.get("key").unwrap_or_default()));
    #[cfg(not(feature = "ssr"))]
    let page = ();
    page
}

/// /jelen/<token>: the page behind an event's QR code (public).
#[component]
pub fn CheckIn() -> impl IntoView {
    #[cfg(feature = "ssr")]
    let page = {
        let token = use_params_map().with_untracked(|p| p.get("token").unwrap_or_default());
        let (done, problem) = use_query_map().with_untracked(|q| (q.get("kesz").is_some(), q.get("hiba")));
        crate::attendance::check_in_page(&token, done, problem.as_deref())
    };
    #[cfg(not(feature = "ssr"))]
    let page = ();
    page
}

/// /admin/linkek: the links the admins keep in [`LINKS`].
#[component]
pub fn AdminLinks() -> impl IntoView {
    // Only root can write the file, so its HTML is trusted as is.
    let text = std::fs::read_to_string(LINKS).ok().map(|md| {
        let mut out = String::new();
        html::push_html(&mut out, Parser::new(&md));
        out
    });
    view! {
        <AdminLayout title="Linkek" section=Section::Links>
            {match text {
                Some(html) => view! { <div class="prose" inner_html=html></div> }.into_any(),
                None => view! { <p class="empty-note">"A linkek helye a szerveren: " <code>{LINKS}</code></p> }.into_any(),
            }}
        </AdminLayout>
    }
}

/// Where a refused sign-in lands; the server log says why.
#[component]
pub fn SignInRefused() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::FORBIDDEN);
    }

    view! {
        <Title text="Nem sikerült belépni"/>
        <Meta name="robots" content="noindex"/>
        <PageHeader title="Nem sikerült belépni">
            <p class="page-lead">
                "Ide a Kozmosz Discord-szerverének admin rangjával és bekapcsolt kétlépcsős azonosítással lehet belépni."
            </p>
            <div class="actions">
                <a class="button button-signal" href="/">"Kezdőlap"</a>
            </div>
        </PageHeader>
    }
}
