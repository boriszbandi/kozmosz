use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::{doc, Block, PROJECTS},
    media::{Captions, Gallery, Picture},
};

pub(crate) const ASTRO_IMAGE: &str = "asztro/orion-kod";
/// Wide band of the Orion photo for the full-width page header (see tools/images.py).
pub(crate) const ASTRO_IMAGE_WIDE: &str = "asztro/orion-kod-szeles";
pub(crate) const SSTV_CARD_IMAGE: &str = "sstv/20241113-202308";
pub(crate) const SSTV_HERO_IMAGE: &str = "sstv/20241114-224220";

/// Real reception data per project, shown as a data rail: (label, value, monospace).
/// Number and unit are joined by a no-break space.
fn facts(name: &str, images: usize) -> Vec<(&'static str, String, bool)> {
    match name {
        "projektek-isstv" => vec![
            ("Frekvencia", "145,800\u{a0}MHz".into(), true),
            ("Adásmód", "SSTV".into(), true),
            ("Vétel helye", "BME környéke, Gellérthegy".into(), false),
            ("Felvételek", images.to_string(), true),
        ],
        "projektek-idojaras-muhold" => vec![
            ("NOAA-15", "137,620\u{a0}MHz".into(), true),
            ("NOAA-18", "137,9125\u{a0}MHz".into(), true),
            ("NOAA-19", "137,100\u{a0}MHz".into(), true),
            ("Adásmód", "APT".into(), true),
        ],
        _ => Vec::new(),
    }
}

/// The weather satellites' downlinks, shown on the yellow plate of the bento.
const NOAA: [(&str, &str); 3] = [("NOAA-15", "137,620"), ("NOAA-18", "137,9125"), ("NOAA-19", "137,100")];

/// The three project cells: an astrophoto, an SSTV frame and the yellow frequency plate.
/// `page_level`: the cells are the page's main content (h2 titles, first photo eager) rather
/// than a section of the home page (h3, lazy). Each title link stretches over its cell.
pub fn project_cards(page_level: bool) -> impl IntoView {
    PROJECTS
        .iter()
        .map(move |&name| {
            let page = doc(name);
            let link = view! { <a class="stretched-link" href=page.route.clone()>{page.title.clone()}</a> };
            let title = if page_level {
                view! { <h2 class="cell-title">{link}</h2> }.into_any()
            } else {
                view! { <h3 class="cell-title">{link}</h3> }.into_any()
            };
            let excerpt = view! { <p class="excerpt">{page.excerpt.clone()}</p> };
            match name {
                "projektek-asztrofotok" => view! {
                    <article class="cell cell-astro">
                        <div class="cell-media">
                            // Landscape band where the cell is wide, the full portrait photo on phones.
                            <Picture
                                key=ASTRO_IMAGE_WIDE
                                alt=""
                                sizes="(width > 1280px) 790px, (width >= 768px) 60vw, 92vw"
                                eager=page_level
                                art=("(width < 768px)", ASTRO_IMAGE, "92vw")
                            />
                        </div>
                        <div class="cell-text">{title}{excerpt}</div>
                    </article>
                }
                .into_any(),
                "projektek-isstv" => view! {
                    <article class="cell cell-sstv">
                        <div class="cell-media">
                            <Picture
                                key=SSTV_CARD_IMAGE
                                alt=""
                                sizes="(width > 1280px) 390px, (width > 768px) 34vw, 92vw"
                            />
                        </div>
                        <div class="cell-text">
                            {title}
                            {excerpt}
                            <p class="data-line"><span>"ISS"</span><strong>"145,800\u{a0}MHz"</strong></p>
                        </div>
                    </article>
                }
                .into_any(),
                _ => view! {
                    <article class="cell cell-plate">
                        <div class="cell-text">{title}{excerpt}</div>
                        <dl class="freqs">
                            {NOAA
                                .iter()
                                .map(|&(sat, mhz)| view! {
                                    <div>
                                        <dt>{sat}</dt>
                                        <dd>{mhz}<span>"\u{a0}MHz"</span></dd>
                                    </div>
                                })
                                .collect_view()}
                        </dl>
                    </article>
                }
                .into_any(),
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
        <section class="container page-body">
            <div class="bento">{project_cards(true)}</div>
        </section>
    }
}

/// A project page: header with the lead and the reception data, the rest of the text, then the
/// gallery (or a note while its images are missing).
#[component]
pub fn Project(name: &'static str) -> impl IntoView {
    let page = doc(name);
    let images: Vec<_> = page.images().cloned().collect();
    let mut text = page.intro.iter().filter_map(|b| match b {
        Block::Html(html) => Some(html.clone()),
        Block::Images(_) => None,
    });
    let lead = text.next();
    let rest: Vec<String> = text.collect();
    let rail = facts(name, images.len());
    let lead_view = move || lead.clone().map(|html| view! { <div class="lead" inner_html=html></div> });

    let header = match name {
        // The club's own astrophoto behind the title.
        "projektek-asztrofotok" => view! {
            <div class="page-hero">
                <Picture key=ASTRO_IMAGE_WIDE alt="" sizes="100vw" eager=true class="page-hero-photo"/>
                <PageHeader title=page.title.clone() parent=("Projektek", "/projektek")>
                    {lead_view()}
                </PageHeader>
            </div>
        }
        .into_any(),
        // One received frame large, next to the title and the reception data.
        "projektek-isstv" => view! {
            <div class="container project-split">
                <PageHeader title=page.title.clone() parent=("Projektek", "/projektek")>
                    {lead_view()}
                    <DataRail rail=rail.clone()/>
                </PageHeader>
                <figure class="frame project-frame">
                    <Picture
                        key=SSTV_HERO_IMAGE
                        alt="SSTV-kép az ISS-ről: az első rádióamatőr-összeköttetés egy űrrepülőgépről, 1983"
                        sizes="(width >= 1024px) 480px, (width >= 768px) 560px, 92vw"
                        eager=true
                    />
                </figure>
            </div>
        }
        .into_any(),
        _ => view! {
            <PageHeader title=page.title.clone() parent=("Projektek", "/projektek")>
                {lead_view()}
                <DataRail rail=rail.clone()/>
            </PageHeader>
        }
        .into_any(),
    };

    let captions = if name == "projektek-isstv" { Captions::ReceivedAt } else { Captions::Alt };
    view! {
        <DocMeta doc=page/>
        {header}
        <section class="container page-body project">
            {(!rest.is_empty()).then(|| view! {
                <div class="project-text">
                    {rest.into_iter().map(|html| view! { <div class="prose" inner_html=html></div> }).collect_view()}
                </div>
            })}
            {page.sections.iter().map(|s| view! {
                <h2 class="block-title">{s.heading.clone()}</h2>
                <Blocks blocks=&s.blocks/>
            }).collect_view()}
            {if images.is_empty() {
                view! { <p class="empty-note">"A felvételek hamarosan felkerülnek ide."</p> }.into_any()
            } else {
                view! { <Gallery images=images captions=captions class="rxlog"/> }.into_any()
            }}
        </section>
    }
}

/// Facts in a row of ruled cells.
#[component]
fn DataRail(rail: Vec<(&'static str, String, bool)>) -> impl IntoView {
    (!rail.is_empty()).then(|| view! {
        <dl class="datarail">
            {rail
                .into_iter()
                .map(|(label, value, mono)| view! {
                    <div>
                        <dt>{label}</dt>
                        <dd class:mono=mono>{value}</dd>
                    </div>
                })
                .collect_view()}
        </dl>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_project_has_a_lead() {
        for &name in PROJECTS {
            let page = doc(name);
            assert!(page.intro.iter().any(|b| matches!(b, Block::Html(_))), "{name} has no text");
        }
    }
}
