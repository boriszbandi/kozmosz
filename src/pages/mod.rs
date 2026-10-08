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

/// Page heading area shared by inner pages: optional parent link, title, then any children.
#[component]
fn PageHeader(
    #[prop(into)] title: String,
    /// Parent page link shown above the title, e.g. ("Projektek", "/projektek").
    #[prop(optional)]
    parent: Option<(&'static str, &'static str)>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <header class="page-head container">
            {parent.map(|(label, href)| view! {
                <nav class="crumbs" aria-label="Morzsamenü">
                    <a href=href>{label}</a>
                    <span aria-hidden="true">"/"</span>
                </nav>
            })}
            <h1 class="page-title">{title}</h1>
            {children.map(|c| c())}
        </header>
    }
}

/// Plain text of an HTML fragment up to its first sentence end (". " before a capital letter).
fn first_sentence(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len().saturating_sub(2) {
        if chars[i] == '.' && chars[i + 1] == ' ' && chars[i + 2].is_uppercase() {
            return chars[..=i].iter().collect();
        }
    }
    text
}

#[cfg(test)]
mod tests {
    /// Image keys named in code (not in the Markdown) must exist in src/images.rs.
    #[test]
    fn hard_coded_images_exist() {
        use super::{contact, home, projects};
        for key in [
            home::HERO_IMAGE,
            home::HERO_IMAGE_TALL,
            home::COMMUNITY_IMAGE,
            home::PROJECTS_IMAGE,
            contact::CONTACT_IMAGE,
            projects::ASTRO_IMAGE,
            projects::ASTRO_IMAGE_WIDE,
            projects::SSTV_CARD_IMAGE,
            projects::SSTV_HERO_IMAGE,
        ] {
            assert!(crate::images::get(key).is_some(), "missing image {key}");
        }
    }

    #[test]
    fn first_sentence_skips_abbreviations() {
        assert_eq!(
            super::first_sentence("<p>A C3S Kft. magyar cég vezetője. Korábban mást csinált.</p>"),
            "A C3S Kft. magyar cég vezetője."
        );
        assert_eq!(super::first_sentence("<p>Egy mondat.</p>"), "Egy mondat.");
    }
}
