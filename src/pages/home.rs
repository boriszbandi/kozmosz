use leptos::prelude::*;

use super::{first_sentence, lecturers::lecturers, projects::project_cards};
use crate::{
    calendar::{self, Event},
    content::doc,
    icons,
    layout::PageMeta,
    media::{Icon, Picture},
};

/// Seconds each lecturer stays in the home page's carousel, and of that the glass cross-fade.
/// style/main.css times the light sweep (`lecture-sweep`: 6s, the first 20%) to these numbers.
const TURN: f32 = 6.0;
const FADE: f32 = 1.2;

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

/// Every lecturer as one linked line-up, next to their photos taking turns: a carousel without
/// JavaScript (owner's request 2026-10-20). All slides share one grid cell, and one CSS animation
/// per slide, started a turn after the previous one, frosts it in and out (style/main.css,
/// "Lecture carousel"). A toggle pauses it; without motion the first lecturer stays.
#[component]
fn Lectures() -> impl IntoView {
    let all = lecturers();
    let slides: Vec<_> = all
        .iter()
        .filter_map(|l| {
            let sentence = l.bio.first().map(|b| first_sentence(b)).unwrap_or_default();
            Some((l.name.clone(), l.photo.clone()?, sentence))
        })
        .collect();
    let turns = (slides.len() > 1).then(|| turn_keyframes(slides.len()));
    let cycle = format!("--cycle:{}s", TURN * slides.len() as f32);
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
                <div class="lecture-feature carousel" style=cycle>
                    {turns.map(|keyframes| view! {
                        // Keyframe offsets cannot use variables: these are made for the slide count.
                        <style inner_html=keyframes></style>
                        <input type="checkbox" id="eloadok-allj" class="carousel-pause visually-hidden"/>
                        <label for="eloadok-allj" class="carousel-toggle">
                            <span class="visually-hidden">"Képváltás megállítása"</span>
                            <Icon svg=icons::PAUSE/>
                            <Icon svg=icons::PLAY/>
                        </label>
                    })}
                    {slides
                        .into_iter()
                        .enumerate()
                        .map(|(i, (name, photo, sentence))| view! {
                            <figure class="frame carousel-slide" style=format!("--at:{:.1}s", i as f32 * TURN - FADE)>
                                // A 3:2 photo covering a 4:5 box: drawn about 1.9x as wide as the column.
                                <Picture key=photo.key alt=photo.alt sizes="(width >= 1024px) 720px, (width >= 768px) 62vw, 92vw"/>
                                <figcaption>
                                    <strong>{name}</strong>
                                    {sentence}
                                </figcaption>
                            </figure>
                        })
                        .collect_view()}
                </div>
            </div>
        </section>
    }
}

/// Keyframes for `n` slides taking turns in a cycle of `n` turns: each slide clears from frosted
/// glass within FADE, holds until its turn ends, then frosts over while the next one clears.
fn turn_keyframes(n: usize) -> String {
    let at = |seconds: f32| seconds / (TURN * n as f32) * 100.0;
    let frosted = "opacity:0;filter:blur(14px) saturate(1.8) brightness(1.25)";
    format!(
        "@keyframes lecture-turn{{0%{{{frosted};transform:scale(1.04)}}\
         {:.3}%,{:.3}%{{opacity:1;filter:blur(0) saturate(1) brightness(1);transform:none}}\
         {:.3}%,100%{{{frosted};transform:scale(.98)}}}}",
        at(FADE),
        at(TURN),
        at(TURN + FADE),
    )
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
    fn carousel_keyframes_fit_the_cycle() {
        // 15 lecturers: 90 s cycle, in by 1.2 s, out between 6 and 7.2 s.
        assert_eq!(
            super::turn_keyframes(15),
            "@keyframes lecture-turn{0%{opacity:0;filter:blur(14px) saturate(1.8) brightness(1.25);transform:scale(1.04)}\
             1.333%,6.667%{opacity:1;filter:blur(0) saturate(1) brightness(1);transform:none}\
             8.000%,100%{opacity:0;filter:blur(14px) saturate(1.8) brightness(1.25);transform:scale(.98)}}"
        );
    }
}
