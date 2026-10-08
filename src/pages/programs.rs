use leptos::prelude::*;

use super::{Blocks, DocMeta, PageHeader};
use crate::{
    calendar::{self, Event, Segment},
    content::doc,
    icons,
    media::{Icon, Picture},
    site,
};

/// Events from the club's Google Calendar (downloaded by the server, see src/calendar.rs), then
/// the ways to subscribe to it. Everything from the feed is rendered as escaped text.
#[component]
pub fn Programs() -> impl IntoView {
    let page = doc("programjaink");
    let calendar = calendar::current();
    let upcoming = calendar.upcoming.clone();
    let past = calendar.past.clone();

    let upcoming = if !upcoming.is_empty() {
        view! { <EventList events=upcoming/> }.into_any()
    } else if calendar.loaded {
        view! {
            <p class="events-empty">
                "Most még nincs kiírva közelgő program. "
                <a href="#feliratkozas">"Iratkozz fel a naptárunkra"</a>
                ", és az új programok maguktól megjelennek nálad."
            </p>
        }
        .into_any()
    } else {
        view! {
            <p class="events-empty">
                "A programnaptárt most nem sikerült betölteni. "
                <a href=site::CALENDAR_EMBED_URL rel="noopener">"Nézd meg a Google Naptárban"</a>
                ", vagy "
                <a href="#feliratkozas">"iratkozz fel rá"</a>
                "."
            </p>
        }
        .into_any()
    };

    view! {
        <DocMeta doc=page/>
        <PageHeader title=page.title.clone()/>
        <section class="container section-tight events events-upcoming" aria-labelledby="kozelgo-programok">
            <h2 id="kozelgo-programok">"Közelgő programok"</h2>
            {upcoming}
        </section>
        <section id="feliratkozas" class="container section-tight split calendar-subscribe">
            <div class="split-text">
                // "Vagy iratkozz fel az online naptárunkra!"
                <Blocks blocks=&page.intro/>
                <div class="actions">
                    <a class="button button-primary" href=site::CALENDAR_SUBSCRIBE_GOOGLE_URL rel="noopener">
                        <Icon svg=icons::CALENDAR_BLANK/>
                        "Kozmosz online naptár"
                    </a>
                    // A link, not an embedded iframe: the Google embed is heavy and third-party.
                    <a class="button button-secondary" href=site::CALENDAR_EMBED_URL rel="noopener">
                        "Naptár megnyitása"
                        <Icon svg=icons::ARROW_UP_RIGHT/>
                    </a>
                </div>
                <p class="calendar-alt">
                    <a href=site::CALENDAR_SUBSCRIBE_WEBCAL_URL>"Feliratkozás Apple Naptárban vagy Outlookban"</a>
                </p>
            </div>
            <Picture
                key="kozosseg/eloadas-szinpad"
                alt="Előadás a Kozmosz rendezvényén, egy aula színpadán"
                sizes="(width > 1100px) 620px, (width > 860px) 55vw, 92vw"
                class="split-media"
            />
        </section>
        {(!past.is_empty()).then(|| view! {
            <section class="container section-tight events events-past" aria-labelledby="legutobbi-programok">
                <h2 id="legutobbi-programok">"Legutóbbi programok"</h2>
                <EventList events=past/>
            </section>
        })}
    }
}

#[component]
fn EventList(events: Vec<Event>) -> impl IntoView {
    view! {
        <ol class="event-list" role="list">
            {events.into_iter().map(|event| view! { <li><EventCard event/></li> }).collect_view()}
        </ol>
    }
}

#[component]
fn EventCard(event: Event) -> impl IntoView {
    let datetime = event.datetime_attr();
    let date = event.date_label();
    let hours = event.time_label();
    view! {
        <article class="event">
            <h3 class="event-title">{event.title}</h3>
            <p class="event-when">
                <time datetime=datetime>{date}</time>
                {hours.map(|hours| view! { " " <span class="event-hours">{hours}</span> })}
            </p>
            {event.location.map(|location| view! {
                <p class="event-location">
                    <Icon svg=icons::MAP_PIN/>
                    <span>{location}</span>
                </p>
            })}
            {event.description.map(|text| view! { <p class="event-description"><Description text/></p> })}
        </article>
    }
}

/// Description text with its web addresses as links (shown without the scheme, shortened).
#[component]
fn Description(text: String) -> impl IntoView {
    calendar::linkify(&text)
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => text.to_owned().into_any(),
            Segment::Link(url) => {
                let label = link_label(url);
                view! { <a href=url.to_owned() rel="noopener nofollow">{label}</a> }.into_any()
            }
        })
        .collect_view()
}

/// "https://www.example.com/a/very/long/path" → "example.com/a/very/long/pa…"
fn link_label(url: &str) -> String {
    const MAX: usize = 40;
    let label = url.split_once("://").map_or(url, |(_, rest)| rest);
    let label = label.strip_prefix("www.").unwrap_or(label).trim_end_matches('/');
    if label.chars().count() <= MAX {
        label.to_owned()
    } else {
        let cut: String = label.chars().take(MAX - 1).collect();
        format!("{cut}…")
    }
}
