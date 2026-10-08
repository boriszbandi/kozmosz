use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::doc,
    icons,
    media::{Icon, Picture},
    site,
};

#[component]
pub fn Programs() -> impl IntoView {
    let page = doc("programjaink");
    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container section-tight split">
            <div class="split-text">
                // A link, not an embedded iframe: browsers load iframes even inside a closed
                // <details>, and the Google embed is heavy and third-party.
                <a class="button button-primary" href=site::CALENDAR_EMBED_URL rel="noopener">
                    <Icon svg=icons::CALENDAR_BLANK/>
                    "Naptár megnyitása"
                    <Icon svg=icons::ARROW_UP_RIGHT/>
                </a>
                <Blocks blocks=&page.intro/>
                <a class="button button-secondary" href=site::CALENDAR_SUBSCRIBE_GOOGLE_URL rel="noopener">
                    <Icon svg=icons::CALENDAR_BLANK/>
                    "Kozmosz online naptár"
                </a>
                <p class="calendar-alt">
                    <a href=site::CALENDAR_SUBSCRIBE_WEBCAL_URL>"Feliratkozás Apple Naptárban vagy Outlookban"</a>
                </p>
            </div>
            <Picture
                key="kozosseg/eloadas-szinpad"
                alt="Előadás a Kozmosz rendezvényén, egy aula színpadán"
                sizes="(width > 1100px) 620px, (width > 860px) 55vw, 92vw"
                class="split-media"
                eager=true
            />
        </section>
    }
}
