use leptos::prelude::*;

use super::{lecturers::lecturers, projects::project_cards};
use crate::{
    content::doc,
    icons,
    layout::PageMeta,
    media::{Icon, Picture},
    site,
};

#[component]
pub fn Home() -> impl IntoView {
    let page = doc("kezdolap");
    let welcome = page.section("Köszöntünk");
    let lead: String = welcome.map(|s| s.html().collect()).unwrap_or_default();

    view! {
        <PageMeta title="" description=page.description.clone() path="/"/>

        // Split hero: message left, a real night-sky photo of the members right.
        <section class="hero">
            <div class="container hero-grid">
                <div class="hero-copy">
                    <p class="eyebrow">{site::NAME}</p>
                    <h1>"Köszöntünk"</h1>
                    <div class="hero-lead" inner_html=lead></div>
                    <div class="actions">
                        <a class="button button-primary" href="/programjaink">
                            "Programjaink"
                            <Icon svg=icons::ARROW_RIGHT/>
                        </a>
                        <a class="button button-secondary" href="/rolunk">"Rólunk"</a>
                    </div>
                </div>
                <Picture
                    key="kozosseg/egbolt"
                    alt="Tagjaink sziluettje a csillagos égbolt alatt"
                    // Drawn width of the 16:9 photo cropped into the 4:3 box of the 7/12 column.
                    sizes="(width > 1260px) 887px, (width > 1000px) calc((100vw - 120px) * 7 / 9), (width > 860px) calc((92vw - 40px) * 7 / 9), calc(92vw * 4 / 3)"
                    eager=true
                    class="hero-media"
                />
            </div>
        </section>

        <Community/>

        <figure class="band">
            <Picture
                key="kozosseg/csillagvizsgalo"
                alt="Tagjaink éjszaka egy csillagvizsgáló kupolája előtt"
                sizes="100vw"
                class="band-media"
            />
        </figure>

        <ProjectsTeaser/>
        <LecturerStrip/>
    }
}

/// Editorial block: the two community paragraphs, the first set as a lead.
#[component]
fn Community() -> impl IntoView {
    let section = doc("kezdolap").section("Közösségünk");
    let paragraphs: Vec<&str> = section.map(|s| s.html().collect()).unwrap_or_default();
    view! {
        <section class="section container editorial" aria-labelledby="kozossegunk">
            <h2 id="kozossegunk">"Közösségünk"</h2>
            <div class="editorial-body">
                {paragraphs
                    .into_iter()
                    .enumerate()
                    .map(|(i, html)| {
                        let class = if i == 0 { "prose lead" } else { "prose" };
                        view! { <div class=class inner_html=html></div> }
                    })
                    .collect_view()}
            </div>
        </section>
    }
}

/// Bento of the three projects: two photo cells and one tinted text cell.
#[component]
fn ProjectsTeaser() -> impl IntoView {
    let intro: String =
        doc("kezdolap").section("Projektjeink").map(|s| s.html().collect()).unwrap_or_default();
    view! {
        <section class="section container" aria-labelledby="projektjeink">
            <div class="section-head">
                <h2 id="projektjeink">"Projektjeink"</h2>
                <div class="prose" inner_html=intro></div>
            </div>
            <div class="bento">{project_cards(false)}</div>
        </section>
    }
}

/// Horizontally scrolling strip of lecturer portraits, linking into the lecturers page.
#[component]
fn LecturerStrip() -> impl IntoView {
    view! {
        <section class="section strip-section" aria-labelledby="eloadasaink">
            <div class="container section-head section-head-row">
                <h2 id="eloadasaink">"Előadásaink"</h2>
                <a class="link-arrow" href="/eloadoink">
                    "Előadóink"
                    <Icon svg=icons::ARROW_RIGHT/>
                </a>
            </div>
            <ul class="strip container" role="list" tabindex="0" aria-label="Előadóink">
                {lecturers()
                    .into_iter()
                    .map(|l| {
                        view! {
                            <li>
                                <a href=format!("/eloadoink#{}", l.slug)>
                                    // The visible name labels the link; the photo is decorative here.
                                    {l.photo.map(|p| view! { <Picture key=p.key alt="" sizes="210px"/> })}
                                    <span class="strip-name">{l.name}</span>
                                </a>
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
    use crate::content::doc;

    /// The home page looks its sections up by heading: renaming one in kezdolap.md must fail here.
    #[test]
    fn home_sections_exist() {
        let page = doc("kezdolap");
        for heading in ["Köszöntünk", "Közösségünk", "Projektjeink", "Előadásaink"] {
            assert!(page.section(heading).is_some(), "kezdolap.md lost its \"## {heading}\" section");
        }
    }
}
