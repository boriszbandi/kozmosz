use leptos::prelude::*;
use leptos_meta::Title;

use super::PageHeader;

#[component]
pub fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::NOT_FOUND);
    }

    view! {
        <Title text="Nem található"/>
        <PageHeader title="Ez az oldal nem létezik">
            <p class="page-lead">"Lehet, hogy elköltözött, amikor megújult az oldal."</p>
            <div class="actions">
                <a class="button button-signal" href="/">"Kezdőlap"</a>
            </div>
        </PageHeader>
    }
}
