use leptos::prelude::*;

use super::{DocMeta, PageHeader};
use crate::{
    content::{doc, Block, ImageRef},
    media::Picture,
};

pub struct Lecturer {
    pub name: String,
    /// Anchor on the lecturers page, e.g. "szabo-jozsef".
    pub slug: String,
    pub subtitle: Option<String>,
    pub bio: Vec<String>,
    pub photo: Option<ImageRef>,
}

/// Lecturers from content/oldalak/eloadoink.md: one `##` section each, an optional bold-only
/// first paragraph as subtitle, bio paragraphs and a photo.
pub fn lecturers() -> Vec<Lecturer> {
    doc("eloadoink")
        .sections
        .iter()
        .map(|section| {
            let mut subtitle = None;
            let mut bio = Vec::new();
            let mut photo = None;
            for block in &section.blocks {
                match block {
                    Block::Html(html) => match bold_only(html) {
                        Some(text) if subtitle.is_none() && bio.is_empty() => subtitle = Some(text),
                        _ => bio.push(html.clone()),
                    },
                    Block::Images(images) => {
                        if photo.is_none() {
                            photo = images.first().cloned();
                        }
                    }
                }
            }
            let slug = photo
                .as_ref()
                .and_then(|p| p.key.strip_prefix("eloadok/"))
                .map(str::to_owned)
                .unwrap_or_else(|| slugify(&section.heading));
            Lecturer { name: section.heading.clone(), slug, subtitle, bio, photo }
        })
        .collect()
}

/// The (HTML-escaped) inner markup of `<p><strong>…</strong></p>`, if the paragraph is only that.
fn bold_only(html: &str) -> Option<String> {
    let inner = html.trim().strip_prefix("<p><strong>")?.strip_suffix("</strong></p>")?;
    (!inner.contains('<')).then(|| inner.to_owned())
}

fn slugify(name: &str) -> String {
    let folded: String = name
        .trim_start_matches("Dr. ")
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' | 'ö' | 'ő' => 'o',
            'ú' | 'ü' | 'ű' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    folded.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

#[component]
pub fn Lecturers() -> impl IntoView {
    let page = doc("eloadoink");
    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container page-body">
            <ul class="lecturers" role="list">
                {lecturers()
                    .into_iter()
                    .enumerate()
                    .map(|(i, l)| {
                        view! {
                            <li class="lecturer" id=l.slug>
                                {l.photo.map(|p| view! {
                                    <Picture
                                        key=p.key
                                        alt=p.alt
                                        sizes="(width > 1280px) 588px, (width >= 768px) 46vw, 92vw"
                                        class="lecturer-photo"
                                        eager={i < 2}
                                    />
                                })}
                                <div class="lecturer-body">
                                    <h2>{l.name}</h2>
                                    {l.subtitle.map(|s| view! { <p class="lecturer-title" inner_html=s></p> })}
                                    {l.bio
                                        .into_iter()
                                        .map(|html| view! { <div class="prose" inner_html=html></div> })
                                        .collect_view()}
                                </div>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_fold_hungarian_letters() {
        assert_eq!(slugify("Dr. Szabó József"), "szabo-jozsef");
        assert_eq!(slugify("Detre Örs Hunor"), "detre-ors-hunor");
        assert_eq!(slugify("Sárneczky Krisztián"), "sarneczky-krisztian");
    }

    #[test]
    fn every_lecturer_has_a_photo_and_bio() {
        let all = lecturers();
        assert_eq!(all.len(), 15);
        for l in &all {
            assert!(l.photo.is_some(), "{} has no photo", l.name);
            assert!(!l.bio.is_empty(), "{} has no bio", l.name);
        }
    }
}
