use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::doc,
    icons,
    media::{Icon, Picture},
    site,
};

pub(crate) const CONTACT_IMAGE: &str = "kozosseg/csapat-terem";

#[component]
pub fn Contact() -> impl IntoView {
    let page = doc("elerhetoseg");
    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container page-body contact">
            <div class="contact-text">
                <Blocks blocks=&page.intro/>
                <dl class="contact-list">
                    <div>
                        <dt>"Térkép"</dt>
                        <dd>
                            <a class="link-arrow" href=site::MAPS_URL rel="noopener">
                                "Megnyitás térképen"
                                <Icon svg=icons::ARROW_UP_RIGHT/>
                            </a>
                        </dd>
                    </div>
                    <div>
                        <dt>"Közösségi oldalak"</dt>
                        <dd class="contact-social">
                            <a href=site::FACEBOOK_URL rel="noopener"><Icon svg=icons::FACEBOOK_LOGO/>"Facebook"</a>
                            <a href=site::INSTAGRAM_URL rel="noopener"><Icon svg=icons::INSTAGRAM_LOGO/>"Instagram"</a>
                        </dd>
                    </div>
                </dl>
            </div>
            <figure class="frame">
                <Picture
                    key=CONTACT_IMAGE
                    alt="A szakkollégium tagjai egy egyetemi tanteremben"
                    sizes="(width > 1280px) 520px, (width > 768px) 42vw, 92vw"
                    eager=true
                />
            </figure>
        </section>
    }
}
