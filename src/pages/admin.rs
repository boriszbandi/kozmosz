//! The admins' dashboard (src/admin.rs lets only signed-in admins in): a sidebar with its
//! sections next to the page. The attendance sheets are server-only code (src/attendance.rs);
//! their routes here just call it, and the browser build never runs them.

use leptos::prelude::*;
use leptos_meta::{Meta, Title};
#[cfg(feature = "ssr")]
use leptos_router::hooks::{use_params_map, use_query_map};
use pulldown_cmark::{html, Parser};

use super::PageHeader;
use crate::{icons, media::Icon};

/// Text of the links section: Markdown in /etc/kozmosz/admin.md on the server, read on every
/// request (an edit shows at once) and kept out of the public repository. Without the file the
/// section is not shown.
const LINKS: &str = "/etc/kozmosz/admin.md";

/// The club's planning tools, at the top of the sidebar (opened in a new tab). None: not set up
/// yet, shown greyed out.
const TOOLS: [(&str, &str, Option<&str>); 3] = [
    ("Jira", icons::KANBAN, None),
    ("Teendők", icons::LIST_CHECKS, None),
    ("Gantt", icons::CHART_BAR_HORIZONTAL, None),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Sheets,
    Links,
}

/// The dashboard frame: the tools, the sections and sign-out in the sidebar, the page beside it.
#[component]
pub fn AdminLayout(#[prop(into)] title: String, section: Section, children: Children) -> impl IntoView {
    let current = |s: Section| (s == section).then_some("page");
    view! {
        <Title text=title.clone()/>
        <Meta name="robots" content="noindex"/>
        <div class="container admin">
            <aside class="admin-side">
                <div class="admin-tools">
                    {TOOLS
                        .into_iter()
                        .map(|(label, icon, url)| match url {
                            Some(url) => view! {
                                <a class="admin-tool" href=url target="_blank" rel="noopener">
                                    <Icon svg=icon/>
                                    {label}
                                </a>
                            }
                            .into_any(),
                            None => view! {
                                <span class="admin-tool" title="Még nincs beállítva">
                                    <Icon svg=icon/>
                                    {label}
                                    <span class="visually-hidden">" (még nincs beállítva)"</span>
                                </span>
                            }
                            .into_any(),
                        })
                        .collect_view()}
                </div>
                <p class="admin-category" id="adminisztracio">"Adminisztráció"</p>
                <nav class="admin-nav" aria-labelledby="adminisztracio">
                    <a href="/admin" aria-current=current(Section::Sheets)>
                        <Icon svg=icons::QR_CODE/>
                        "Jelenléti ívek"
                    </a>
                    {std::fs::metadata(LINKS).is_ok().then(|| view! {
                        <a href="/admin/linkek" aria-current=current(Section::Links)>
                            <Icon svg=icons::LINK_SIMPLE/>
                            "Linkek"
                        </a>
                    })}
                </nav>
                <form method="post" action="/kilepes">
                    <button class="admin-signout" type="submit">
                        <Icon svg=icons::SIGN_OUT/>
                        "Kilépés"
                    </button>
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
        let problem = use_query_map().with_untracked(|q| q.get("hiba"));
        crate::attendance::check_in_page(&token, problem.as_deref())
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
                "Ide a Kozmosz Discord-szerverének admin rangjával lehet belépni."
            </p>
            <div class="actions">
                <a class="button button-signal" href="/">"Kezdőlap"</a>
            </div>
        </PageHeader>
    }
}
