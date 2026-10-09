use leptos::prelude::*;

use super::{first_sentence, lecturers::lecturers, projects::project_cards};
use crate::{
    calendar::{self, Event},
    content::doc,
    icons,
    layout::PageMeta,
    media::{Icon, Picture},
};

/// The lecturer shown with a photo on the home page.
const FEATURED_LECTURER: &str = "horvath-gyula";

pub(crate) const HERO_IMAGE: &str = "kozosseg/egbolt";
/// Portrait crop of the hero photo for narrow screens (see tools/images.py).
pub(crate) const HERO_IMAGE_TALL: &str = "kozosseg/egbolt-allo";
pub(crate) const COMMUNITY_IMAGE: &str = "kozosseg/csillagvizsgalo";
pub(crate) const PROJECTS_IMAGE: &str = "asztro/hold-lap";

#[component]
pub fn Home() -> impl IntoView {
    let page = doc("kezdolap");
    let lead: String = page.section("Köszöntünk").map(|s| s.html().collect()).unwrap_or_default();

    view! {
        <PageMeta title="" description=page.description.clone() path="/"/>

        // The members under the night sky, with the next programme docked on the horizon.
        <section class="hero" aria-labelledby="hero-title">
            <Picture
                key=HERO_IMAGE
                alt="Tagjaink sziluettje a csillagos égbolt alatt"
                sizes="100vw"
                eager=true
                class="hero-photo"
                art=("(width < 768px)", HERO_IMAGE_TALL, "100vw")
            />
            // A satellite crossing the sky now and then (CSS only, the site's one loop).
            <span class="pass" aria-hidden="true"></span>
            <div class="container hero-inner">
                <div class="hero-copy">
                    <h1 id="hero-title">"Köszöntünk"</h1>
                    <div class="hero-lead" inner_html=lead></div>
                    // One action: "Rólunk" is in the menu already.
                    <div class="actions">
                        <a class="button button-signal" href="/programjaink">
                            "Programjaink"
                            <Icon svg=icons::ARROW_RIGHT/>
                        </a>
                    </div>
                </div>
                <NextEvent/>
            </div>
        </section>

        <Community/>
        <Projects/>
        <Lectures/>
    }
}

/// The next programme from the club's calendar, as a frameless readout on the horizon of the
/// photo. Hidden while the calendar has never loaded.
#[component]
fn NextEvent() -> impl IntoView {
    let calendar = calendar::current();
    if !calendar.loaded {
        return None;
    }
    let today = calendar.today;
    let Some(event) = calendar.upcoming.first().cloned() else {
        return Some(
            view! {
                <aside class="signal" aria-label="Következő program">
                    <p class="signal-label">"Következő program"</p>
                    <p class="signal-empty">
                        "Most nincs kiírt program. "
                        <a href="/programjaink#feliratkozas">"Iratkozz fel a naptárunkra"</a>
                        ", és értesülsz az újakról."
                    </p>
                </aside>
            }
            .into_any(),
        );
    };
    let is_today = is_on(&event, today);
    Some(
        view! {
            <aside class="signal" aria-label="Következő program">
                <p class="signal-label">
                    "Következő program"
                    {is_today.then(|| view! { <span class="tag-today">"Ma"</span> })}
                </p>
                <EventWhen event=event.clone() today/>
                <div class="signal-what">
                    <h2>
                        <a href="/programjaink">{event.title.clone()}</a>
                    </h2>
                    {event.location.clone().map(|place| view! {
                        <p class="where"><Icon svg=icons::MAP_PIN/><span>{place}</span></p>
                    })}
                </div>
                <a class="signal-link" href="/programjaink#feliratkozas">
                    "Feliratkozás a naptárra"
                    <Icon svg=icons::ARROW_UP_RIGHT/>
                </a>
            </aside>
        }
        .into_any(),
    )
}

/// Big date and time of an event, as in a reception log, with the weekday and how far away it is.
#[component]
fn EventWhen(event: Event, today: Option<calendar::Date>) -> impl IntoView {
    let time = if event.all_day { "egész nap".to_owned() } else { event.start.clock() };
    let sub = match today.map(|t| event.start.date.days_since_epoch() - t.days_since_epoch()) {
        Some(1) => format!("{}, holnap", event.start.date.weekday_name()),
        Some(n) if n > 1 => format!("{}, {n} nap múlva", event.start.date.weekday_name()),
        _ => event.start.date.weekday_name().to_owned(),
    };
    view! {
        <time class="signal-when" datetime=event.datetime_attr()>
            <span class="big">{event.start.date.short()}</span>
            <span class="big big-time">{time}</span>
            <span class="sub">{sub}</span>
        </time>
    }
}

/// The event runs on `today` (it may have started earlier).
pub fn is_on(event: &Event, today: Option<calendar::Date>) -> bool {
    today.is_some_and(|day| event.start.date <= day && day <= event.end.date)
}

/// The two community paragraphs: the first as a typographic statement, then the observatory photo
/// as a full-width band that dissolves into the night, then the second paragraph.
#[component]
fn Community() -> impl IntoView {
    let section = doc("kezdolap").section("Közösségünk");
    let mut paragraphs = section.map(|s| s.html().collect::<Vec<_>>()).unwrap_or_default().into_iter();
    let statement = paragraphs.next().unwrap_or_default();
    let rest: String = paragraphs.collect();
    view! {
        <section class="section container" aria-labelledby="kozossegunk">
            <h2 class="section-title" id="kozossegunk">"Közösségünk"</h2>
            <div class="statement" inner_html=statement></div>
            <figure class="band">
                <Picture
                    key=COMMUNITY_IMAGE
                    alt="Tagjaink éjszaka egy csillagvizsgáló kupolája előtt"
                    sizes="(width >= 1680px) 1680px, 100vw"
                />
            </figure>
            <div class="community-text" inner_html=rest></div>
        </section>
    }
}

/// Projects: the introduction next to the Moon (bleeding off the page edge), then the bento.
#[component]
fn Projects() -> impl IntoView {
    let intro: String =
        doc("kezdolap").section("Projektjeink").map(|s| s.html().collect()).unwrap_or_default();
    view! {
        <section class="section projects" aria-labelledby="projektjeink">
            <div class="container">
                <h2 class="section-title" id="projektjeink">"Projektjeink"</h2>
            </div>
            <div class="container projects-head">
                <div class="projects-intro">
                    <div class="section-intro" inner_html=intro></div>
                </div>
                <figure class="moon">
                    // Named by its caption.
                    <Picture
                        key=PROJECTS_IMAGE
                        alt=""
                        sizes="(width >= 768px) min(36vw, 440px), 82vw"
                    />
                    <figcaption>"A Hold, Végh Máté felvétele"</figcaption>
                </figure>
            </div>
            <div class="container">
                <div class="bento">{project_cards(false)}</div>
            </div>
        </section>
    }
}

/// Every lecturer as one linked line-up, next to one featured lecture photo.
#[component]
fn Lectures() -> impl IntoView {
    let all = lecturers();
    let featured = all.iter().find(|l| l.slug == FEATURED_LECTURER).map(|l| {
        (l.name.clone(), l.photo.clone(), l.bio.first().map(|b| first_sentence(b)).unwrap_or_default())
    });
    view! {
        <section class="section container" aria-labelledby="eloadasaink">
            <h2 class="section-title" id="eloadasaink">"Előadásaink"</h2>
            <div class="lectures">
                <div class="lineup-wrap">
                    <ul class="lineup" role="list">
                        {all
                            .iter()
                            .map(|l| view! { <li><a href=format!("/eloadoink#{}", l.slug)>{l.name.clone()}</a></li> })
                            .collect_view()}
                    </ul>
                    <a class="link-arrow" href="/eloadoink">
                        "Előadóink"
                        <Icon svg=icons::ARROW_RIGHT/>
                    </a>
                </div>
                {featured.map(|(name, photo, sentence)| view! {
                    <figure class="frame lecture-feature">
                        {photo.map(|p| view! {
                            // A 3:2 photo covering a 4:5 box: drawn about 1.9x as wide as the column.
                            <Picture key=p.key alt=p.alt sizes="(width >= 1024px) 720px, (width >= 768px) 62vw, 92vw"/>
                        })}
                        <figcaption>
                            <strong>{name}</strong>
                            {sentence}
                        </figcaption>
                    </figure>
                })}
            </div>
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

    #[test]
    fn featured_lecturer_exists() {
        assert!(super::lecturers().iter().any(|l| l.slug == super::FEATURED_LECTURER));
    }
}
