use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::doc,
    icons,
    media::{Icon, Picture},
    site,
};

#[component]
pub fn Contact() -> impl IntoView {
    let page = doc("elerhetoseg");
    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container section-tight split split-reverse">
            <div class="split-text contact">
                <Blocks blocks=&page.intro/>
                <div class="actions">
                    <a class="button button-primary" href=format!("mailto:{}", site::EMAIL)>
                        <Icon svg=icons::ENVELOPE_SIMPLE/>
                        "E-mail küldése"
                    </a>
                    <a class="button button-secondary" href=site::MAPS_URL rel="noopener">
                        <Icon svg=icons::MAP_PIN/>
                        "Megnyitás térképen"
                    </a>
                </div>
                <ul class="social-list" role="list">
                    <li>
                        <a href=site::FACEBOOK_URL rel="noopener">
                            <Icon svg=icons::FACEBOOK_LOGO/>
                            "Facebook"
                        </a>
                    </li>
                    <li>
                        <a href=site::INSTAGRAM_URL rel="noopener">
                            <Icon svg=icons::INSTAGRAM_LOGO/>
                            "Instagram"
                        </a>
                    </li>
                </ul>
            </div>
            <Picture
                key="kozosseg/csapat-terem"
                alt="A szakkollégium tagjai egy egyetemi tanteremben"
                sizes="(width > 1100px) 620px, (width > 860px) 55vw, 92vw"
                class="split-media"
                eager=true
            />
        </section>
    }
}
