use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::{doc, Block, PROJECTS},
    icons,
    media::{Icon, Picture},
};

/// Cover image of each project card; a project without one gets a tinted cell.
fn cover(name: &str) -> Option<&'static str> {
    match name {
        "projektek-asztrofotok" => Some("asztro/orion-kod"),
        "projektek-isstv" => Some("sstv/20241113-202308"),
        _ => None,
    }
}

/// The three project cards (bento cells). `page_level`: the cards are the page's main content
/// (h2 titles, feature photo above the fold) rather than a section of another page (h3, lazy).
/// The title link stretches over the whole card, so the link's name is just the title.
pub fn project_cards(page_level: bool) -> impl IntoView {
    PROJECTS
        .iter()
        .enumerate()
        .map(move |(i, &name)| {
            let page = doc(name);
            let class = match (i, cover(name)) {
                (0, _) => "bento-cell bento-feature",
                (_, Some(_)) => "bento-cell",
                (_, None) => "bento-cell bento-tinted",
            };
            let link = view! { <a class="stretched-link" href=page.route.clone()>{page.title.clone()}</a> };
            let title = if page_level {
                view! { <h2 class="bento-title">{link}</h2> }.into_any()
            } else {
                view! { <h3 class="bento-title">{link}</h3> }.into_any()
            };
            view! {
                <article class=class>
                    // Decorative here: the card's subject is named by its title.
                    {cover(name).map(|key| view! {
                        <Picture
                            key=key
                            alt=""
                            sizes="(width > 1100px) 640px, (width > 760px) 55vw, 92vw"
                            class="bento-media"
                            eager={page_level && i == 0}
                        />
                    })}
                    <div class="bento-text">
                        {title}
                        <p>{page.excerpt.clone()}</p>
                        <span class="link-arrow" aria-hidden="true">
                            "Tovább"
                            <Icon svg=icons::ARROW_RIGHT/>
                        </span>
                    </div>
                </article>
            }
        })
        .collect_view()
}

#[component]
pub fn Projects() -> impl IntoView {
    let page = doc("projektek");
    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container section-tight">
            <div class="bento">{project_cards(true)}</div>
        </section>
    }
}

/// A project page: its text, then its gallery (or a note while the images are missing).
#[component]
pub fn Project(name: &'static str) -> impl IntoView {
    let page = doc(name);
    let has_images = page.images().next().is_some();
    let sections = page.sections.iter().map(|s| view! {
        <h2>{s.heading.clone()}</h2>
        <Blocks blocks=&s.blocks/>
    });
    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone() parent=("Projektek", "/projektek")/>
        <section class="container section-tight project">
            <div class="project-text">
                <Blocks blocks=text_only(&page.intro)/>
            </div>
            <Blocks blocks=images_only(&page.intro)/>
            {sections.collect_view()}
            {(!has_images).then(|| view! {
                <p class="empty-note">"A felvételek hamarosan felkerülnek ide."</p>
            })}
        </section>
    }
}

/// Leading text blocks (up to the first image run).
fn text_only(blocks: &'static [Block]) -> &'static [Block] {
    let end = blocks.iter().position(|b| matches!(b, Block::Images(_))).unwrap_or(blocks.len());
    &blocks[..end]
}

fn images_only(blocks: &'static [Block]) -> &'static [Block] {
    &blocks[text_only(blocks).len()..]
}
