use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::{doc, Block},
    media::{Gallery, Picture},
};

#[component]
pub fn About() -> impl IntoView {
    let page = doc("rolunk");
    let photo = page.intro.iter().find_map(|b| match b {
        Block::Images(images) => images.first().cloned(),
        Block::Html(_) => None,
    });
    let text: Vec<String> = page
        .intro
        .iter()
        .filter_map(|b| match b {
            Block::Html(html) => Some(html.clone()),
            Block::Images(_) => None,
        })
        .collect();

    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container page-body about">
            {photo.map(|p| view! {
                <figure class="frame">
                    <Picture key=p.key alt=p.alt sizes="(width > 1280px) 690px, (width > 768px) 56vw, 92vw" eager=true/>
                </figure>
            })}
            <div class="about-text">
                {text.into_iter().map(|html| view! { <div class="statement" inner_html=html></div> }).collect_view()}
            </div>
        </section>
        {page.sections.iter().map(|s| view! {
            <section class="container section" aria-label=s.heading.clone()>
                <h2 class="section-title">{s.heading.clone()}</h2>
                {s.blocks.iter().map(|b| match b {
                    Block::Images(images) => view! {
                        <Gallery
                            images=images.clone()
                            class="gallery-masonry"
                            sizes="(width > 1280px) 383px, (width > 970px) 30vw, (width > 645px) 46vw, 92vw"
                        />
                    }
                    .into_any(),
                    Block::Html(_) => view! { <Blocks blocks=std::slice::from_ref(b)/> }.into_any(),
                }).collect_view()}
            </section>
        }).collect_view()}
    }
}
