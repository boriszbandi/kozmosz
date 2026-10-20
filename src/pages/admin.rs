//! The admins' page (/admin): links to the board, the Drive sheets and the like. Its text is
//! Markdown in /etc/kozmosz/admin.md on the server, read on every request (an edit shows at once)
//! and kept out of the public repository. Only signed-in admins get here (src/admin.rs).

use leptos::prelude::*;
use leptos_meta::{Meta, Title};
use pulldown_cmark::{html, Parser};

use super::PageHeader;

const TEXT: &str = "/etc/kozmosz/admin.md";

#[component]
pub fn AdminPage() -> impl IntoView {
    // Only root can write the file, so its HTML is trusted as is.
    let text = std::fs::read_to_string(TEXT).ok().map(|md| {
        let mut out = String::new();
        html::push_html(&mut out, Parser::new(&md));
        out
    });
    view! {
        <Title text="Admin"/>
        <Meta name="robots" content="noindex"/>
        <PageHeader title="Admin"/>
        <section class="container page-body admin">
            {match text {
                Some(html) => view! { <div class="prose" inner_html=html></div> }.into_any(),
                None => view! { <p class="empty-note">"A linkek helye a szerveren: " <code>{TEXT}</code></p> }.into_any(),
            }}
            <form method="post" action="/kilepes">
                <button class="button button-line" type="submit">"Kilépés"</button>
            </form>
        </section>
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
