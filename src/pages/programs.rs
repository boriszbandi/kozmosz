use leptos::prelude::*;

use super::{home::is_on, Blocks, DocMeta, PageHeader};
use crate::{
    calendar::{self, Date, Event, Segment},
    content::doc,
    icons,
    media::Icon,
    site,
};

/// Events from the club's Google Calendar (downloaded by the server, see src/calendar.rs), next to
/// the ways to subscribe to it. Everything from the feed is rendered as escaped text.
#[component]
pub fn Programs() -> impl IntoView {
    let page = doc("programjaink");
    let calendar = calendar::current();
    let today = calendar.today;
    let upcoming = calendar.upcoming.clone();
    let past = calendar.past.clone();

    let upcoming = if !upcoming.is_empty() {
        view! { <EventList events=upcoming today highlight_first=true/> }.into_any()
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
        <div class="container page-body programs">
            <div class="programs-list">
                <section aria-labelledby="kozelgo-programok">
                    <h2 class="group-title" id="kozelgo-programok">"Közelgő programok"</h2>
                    {upcoming}
                </section>
                {(!past.is_empty()).then(|| view! {
                    <section class="events-past" aria-labelledby="legutobbi-programok">
                        <h2 class="group-title" id="legutobbi-programok">"Legutóbbi programok"</h2>
                        <EventList events=past today highlight_first=false/>
                    </section>
                })}
            </div>
            <aside id="feliratkozas" class="subscribe" aria-label="Feliratkozás a naptárra">
                // "Vagy iratkozz fel az online naptárunkra!"
                <Blocks blocks=&page.intro/>
                <a class="button button-signal" href=site::CALENDAR_SUBSCRIBE_GOOGLE_URL rel="noopener">
                    <Icon svg=icons::CALENDAR_BLANK/>
                    "Kozmosz online naptár"
                </a>
                <a class="button button-line" href=site::CALENDAR_SUBSCRIBE_WEBCAL_URL>
                    "Apple Naptár, Outlook"
                </a>
                // A link, not an embedded iframe: the Google embed is heavy and third-party.
                <a class="link-arrow" href=site::CALENDAR_EMBED_URL rel="noopener">
                    "Naptár megnyitása"
                    <Icon svg=icons::ARROW_UP_RIGHT/>
                </a>
            </aside>
        </div>
    }
}

#[component]
fn EventList(events: Vec<Event>, today: Option<Date>, highlight_first: bool) -> impl IntoView {
    view! {
        <ol class="events" role="list">
            {events
                .into_iter()
                .enumerate()
                .map(|(i, event)| {
                    let next = highlight_first && i == 0;
                    let is_today = highlight_first && is_on(&event, today);
                    view! { <li class="event" class:is-next=next><EventItem event is_today/></li> }
                })
                .collect_view()}
        </ol>
    }
}

#[component]
fn EventItem(event: Event, is_today: bool) -> impl IntoView {
    let date = event.start.date;
    // Same-day events show their hours; longer ones their full range.
    let when = event.time_label().unwrap_or_else(|| {
        if event.all_day && event.end.date == event.start.date { "egész nap".to_owned() } else { event.date_label() }
    });
    view! {
        <time class="ev-date" datetime=event.datetime_attr()>
            <span class="ev-day">{date.day}</span>
            <span class="ev-month">{date.month_short()}</span>
            <span class="ev-wd">{date.weekday_name()}</span>
        </time>
        <div class="ev-main">
            <p class="ev-time">{when}</p>
            <h3 class="ev-title">{event.title}</h3>
            {event.location.map(|location| view! {
                <p class="where"><Icon svg=icons::MAP_PIN/><span>{location}</span></p>
            })}
            {event.description.map(|text| view! { <p class="ev-note"><Description text/></p> })}
        </div>
        {is_today.then(|| view! { <span class="tag-today">"Ma"</span> })}
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
