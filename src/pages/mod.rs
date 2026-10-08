mod about;
mod contact;
mod home;
mod lecturers;
mod not_found;
mod programs;
mod projects;

pub use about::About;
pub use contact::Contact;
pub use home::Home;
pub use lecturers::Lecturers;
pub use not_found::NotFound;
pub use programs::Programs;
pub use projects::{Project, Projects};

use leptos::prelude::*;

use crate::{
    content::{Block, Doc},
    layout::PageMeta,
    media::{Gallery, Picture},
};

/// Metadata of a content page.
#[component]
fn DocMeta(doc: &'static Doc) -> impl IntoView {
    view! { <PageMeta title=doc.title.clone() description=doc.description.clone() path=doc.route.clone()/> }
}

/// Renders blocks as prose: HTML as is, single images as figures, image runs as a gallery.
#[component]
fn Blocks(blocks: &'static [Block]) -> impl IntoView {
    blocks
        .iter()
        .map(|block| match block {
            Block::Html(html) => view! { <div class="prose" inner_html=html.clone()></div> }.into_any(),
            Block::Images(images) if images.len() == 1 => view! {
                <figure class="figure">
                    <Picture
                        key=images[0].key.clone()
                        alt=images[0].alt.clone()
                        sizes="(width > 1100px) 1040px, 92vw"
                    />
                </figure>
            }
            .into_any(),
            Block::Images(images) => view! { <Gallery images=images.clone()/> }.into_any(),
        })
        .collect_view()
}

/// Page heading area shared by inner pages.
#[component]
fn PageHeader(
    #[prop(into)] title: String,
    /// Parent page link shown above the title, e.g. ("Projektek", "/projektek").
    #[prop(optional)]
    parent: Option<(&'static str, &'static str)>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <header class="page-header container">
            {parent.map(|(label, href)| view! {
                <nav class="breadcrumb" aria-label="Morzsamenü">
                    <a href=href>{label}</a>
                </nav>
            })}
            <h1>{title}</h1>
            {children.map(|c| c())}
        </header>
    }
}
