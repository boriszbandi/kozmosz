use leptos::prelude::*;
use leptos_meta::{Meta, Title};
use leptos_router::components::A;

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <Title text=""/>
        <Meta name="description" content="A Kozmosz közösség: hírek, események és beszélgetések az univerzumról."/>
        <section class="container hero">
            <h1>"Köszöntünk a Kozmosz közösségben!"</h1>
            <p class="lead">
                "Fedezd fel velünk az univerzum legfrissebb híreit, eseményeit és beszélgetéseit."
            </p>
        </section>
        <section class="container grid">
            <article class="card card-wide">
                <h2>"Közösség"</h2>
                <p>"Ismerkedj meg más űrrajongókkal, és oszd meg velük, mi nyűgöz le a kozmoszban."</p>
                <A href="/kozosseg" attr:class="card-link">"Tovább a közösséghez"</A>
            </article>
            <article class="card">
                <h2>"Rólunk"</h2>
                <p>"Kik vagyunk, és miért csináljuk."</p>
                <A href="/rolunk" attr:class="card-link">"Bemutatkozás"</A>
            </article>
            <article class="card">
                <h2>"Hamarosan"</h2>
                <p>"Hírek, események és cikkek – dolgozunk rajtuk."</p>
            </article>
        </section>
    }
}

#[component]
pub fn Community() -> impl IntoView {
    view! {
        <Title text="Közösség"/>
        <Meta name="description" content="A Kozmosz közösség oldala."/>
        <section class="container page">
            <h1>"Közösség"</h1>
            <p class="lead">"Ez az oldal hamarosan megtelik tartalommal."</p>
        </section>
    }
}

#[component]
pub fn About() -> impl IntoView {
    view! {
        <Title text="Rólunk"/>
        <Meta name="description" content="Bemutatkozik a Kozmosz közösség."/>
        <section class="container page">
            <h1>"Rólunk"</h1>
            <p class="lead">"Ez az oldal hamarosan megtelik tartalommal."</p>
        </section>
    }
}

#[component]
pub fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::NOT_FOUND);
    }

    view! {
        <Title text="Nem található"/>
        <section class="container page">
            <h1>"404 – Elveszett az űrben"</h1>
            <p class="lead">"Ez az oldal nem létezik."</p>
            <p><a href="/" class="card-link">"Vissza a kezdőlapra"</a></p>
        </section>
    }
}
