use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    content::{doc, Block, PROJECTS},
    gallery,
    media::{Gallery, Picture},
};

pub(crate) const ASTRO_IMAGE: &str = "asztro/orion-kod";
/// Wide band of the Orion photo for the full-width page header (see tools/images.py).
pub(crate) const ASTRO_IMAGE_WIDE: &str = "asztro/orion-kod-szeles";
pub(crate) const SSTV_CARD_IMAGE: &str = "sstv/20241113-202308";

/// Gallery tiles loaded right away on the radio project pages (no header photo there, so the
/// first row is the first image on the page).
const EAGER_TILES: usize = 3;

/// One value of a readout (the data rail of a project).
pub(crate) struct Fact {
    label: &'static str,
    value: String,
    /// Shown smaller after a no-break space, e.g. "MHz".
    unit: Option<&'static str>,
    kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A frequency you can tune to: monospace, in the signal colour.
    Freq,
    /// Other measured data: monospace.
    Data,
    /// Words.
    Text,
}

fn fact(label: &'static str, value: impl Into<String>, unit: Option<&'static str>, kind: Kind) -> Fact {
    Fact { label, value: value.into(), unit, kind }
}

/// The weather satellites' downlinks.
const NOAA: [(&str, &str); 3] = [("NOAA-15", "137,620"), ("NOAA-18", "137,9125"), ("NOAA-19", "137,100")];
/// The ISS's SSTV downlink.
const ISS_MHZ: &str = "145,800";

/// Real reception data per project.
fn facts(name: &str, images: usize) -> Vec<Fact> {
    match name {
        "projektek-isstv" => vec![
            fact("Frekvencia", ISS_MHZ, Some("MHz"), Kind::Freq),
            fact("Adásmód", "SSTV", None, Kind::Data),
            fact("Vétel helye", "BME környéke, Gellérthegy", None, Kind::Text),
            fact("Felvételek", images.to_string(), None, Kind::Data),
        ],
        "projektek-idojaras-muhold" => {
            let mut facts: Vec<Fact> =
                NOAA.iter().map(|&(sat, mhz)| fact(sat, mhz, Some("MHz"), Kind::Freq)).collect();
            facts.push(fact("Adásmód", "APT", None, Kind::Data));
            facts
        }
        _ => (images > 0).then(|| fact("Felvételek", images.to_string(), None, Kind::Data)).into_iter().collect(),
    }
}

/// Values in ruled cells: numbers in monospace, frequencies in the signal colour.
#[component]
pub(crate) fn Readout(facts: Vec<Fact>, #[prop(optional, into)] class: Option<String>) -> impl IntoView {
    let class = format!("readout {}", class.unwrap_or_default());
    (!facts.is_empty()).then(|| view! {
        <dl class=class.trim_end().to_owned()>
            {facts
                .into_iter()
                .map(|f| view! {
                    <div>
                        <dt>{f.label}</dt>
                        <dd class:mono=f.kind != Kind::Text class:freq=f.kind == Kind::Freq>
                            {f.value}
                            {f.unit.map(|u| view! { <span>"\u{a0}"{u}</span> })}
                        </dd>
                    </div>
                })
                .collect_view()}
        </dl>
    })
}

/// view-transition-name of a project's title: the card title on / and /projektek flies into the
/// project page's h1 (style/main.css, "Card into page").
fn title_vt(name: &str) -> &'static str {
    match name {
        "projektek-asztrofotok" => "t-asztro",
        "projektek-isstv" => "t-isstv",
        _ => "t-noaa",
    }
}

/// The three project cells: an astrophoto, an SSTV frame and the weather satellites' downlinks.
/// `page_level`: the cells are the page's main content (h2 titles, first photo eager) rather
/// than a section of the home page (h3, lazy). Each title link stretches over its cell.
pub fn project_cards(page_level: bool) -> impl IntoView {
    PROJECTS
        .iter()
        .map(move |&name| {
            let page = doc(name);
            let link = view! { <a class="stretched-link" href=page.route.clone()>{page.title.clone()}</a> };
            let style = format!("view-transition-name:{}", title_vt(name));
            let title = if page_level {
                view! { <h2 class="cell-title" style=style>{link}</h2> }.into_any()
            } else {
                view! { <h3 class="cell-title" style=style>{link}</h3> }.into_any()
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
                            <Readout facts=vec![fact("ISS", ISS_MHZ, Some("MHz"), Kind::Freq)]/>
                        </div>
                    </article>
                }
                .into_any(),
                _ => view! {
                    <article class="cell cell-noaa">
                        <div class="cell-text">{title}{excerpt}</div>
                        <Readout facts=NOAA.iter().map(|&(sat, mhz)| fact(sat, mhz, Some("MHz"), Kind::Freq)).collect()/>
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

/// A project page, one template for all three: title (over the club's Orion photo on the
/// astrophoto page), the lead next to the reception data, the rest of the text, then the
/// gallery (or a note while it is empty).
#[component]
pub fn Project(name: &'static str) -> impl IntoView {
    let page = doc(name);
    let tiles = gallery::tiles(name);
    let mut text = page.intro.iter().filter_map(|b| match b {
        Block::Html(html) => Some(html.clone()),
        Block::Images(_) => None,
    });
    let lead = text.next();
    let rest: Vec<String> = text.collect();
    let header = move || view! {
        <PageHeader title=page.title.clone() parent=("Projektek", "/projektek") vt=title_vt(name)/>
    };
    let header = if name == "projektek-asztrofotok" {
        view! {
            <div class="page-hero">
                <Picture key=ASTRO_IMAGE_WIDE alt="" sizes="100vw" eager=true class="page-hero-photo"/>
                {header()}
            </div>
        }
        .into_any()
    } else {
        header().into_any()
    };

    view! {
        <DocMeta doc=page/>
        {header}
        <div class="container project-intro">
            {lead.map(|html| view! { <div class="lead" inner_html=html></div> })}
            <Readout facts=facts(name, tiles.len()) class="readout-v"/>
        </div>
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
            {if tiles.is_empty() {
                view! { <p class="empty-note">"A felvételek hamarosan felkerülnek ide."</p> }.into_any()
            } else {
                // Received radio frames get the reception-log look; photos a plain wall.
                let radio = matches!(name, "projektek-isstv" | "projektek-idojaras-muhold");
                let eager = if radio { EAGER_TILES } else { 0 };
                view! { <Gallery tiles class=if radio { "rxlog" } else { "" } eager/> }.into_any()
            }}
        </section>
    }
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

    #[test]
    fn readouts() {
        let sstv = facts("projektek-isstv", 13);
        assert!(sstv[0].kind == Kind::Freq && sstv[0].unit == Some("MHz"));
        assert_eq!(sstv[3].value, "13");
        assert_eq!(facts("projektek-idojaras-muhold", 0).len(), 4);
        assert!(facts("projektek-asztrofotok", 0).is_empty());
        assert_eq!(facts("projektek-asztrofotok", 3)[0].value, "3");
    }
}
