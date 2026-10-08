//! Downloading the feed and turning it into the [`Calendar`] the programs page shows.

use std::{sync::Arc, time::Duration};

use jiff::{tz::TimeZone, SignedDuration, Timestamp};
use leptos::logging::{error, log};

use super::{
    ical::{self, Occurrence, VEvent},
    publish, text, Calendar, Date, Event, LocalDateTime,
};
use crate::site;

/// How often the server downloads the feed again.
pub const REFRESH_EVERY: Duration = Duration::from_secs(10 * 60);
/// The site's time zone: events are shown in Budapest time.
const ZONE: &str = "Europe/Budapest";
/// Upcoming events shown, and how far ahead.
const MAX_UPCOMING: usize = 12;
const AHEAD: SignedDuration = SignedDuration::from_hours(183 * 24);
/// Recent events shown, and how far back.
const MAX_PAST: usize = 6;
const BEHIND: SignedDuration = SignedDuration::from_hours(366 * 24);
/// A larger response is refused (the real feed is about 60 KB).
const MAX_BYTES: usize = 8 * 1024 * 1024;

const MAX_TITLE: usize = 160;
const MAX_LOCATION: usize = 160;
const MAX_DESCRIPTION: usize = 280;

/// The club's calendar feed. Keeps the last good download, so a failed refresh changes nothing
/// but the clock.
pub struct Feed {
    client: Option<reqwest::Client>,
    zone: TimeZone,
    events: Option<Arc<[VEvent]>>,
    /// Last logged summary of the feed, so an unchanged feed is not logged every 10 minutes.
    summary: String,
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}

impl Feed {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(format!("kozmosz-site/{} (+{})", env!("CARGO_PKG_VERSION"), site::ORIGIN))
            .connect_timeout(Duration::from_secs(5))
            .https_only(true)
            .build()
            .map_err(|err| error!("calendar: cannot create the HTTP client: {err}"))
            .ok();
        let zone = TimeZone::get(ZONE).unwrap_or_else(|err| {
            error!("calendar: time zone {ZONE} is missing ({err}); showing UTC");
            TimeZone::UTC
        });
        Self { client, zone, events: None, summary: String::new() }
    }

    /// Downloads the feed (giving up after `timeout`), then publishes the calendar as of now,
    /// from the new data or, if the download failed, from the last good one. Never panics:
    /// parsing and expanding run on the blocking pool, where a panic on a malformed feed is
    /// caught and logged (the previous data stays) instead of killing the server at startup or
    /// ending the refresh loop.
    pub async fn refresh(&mut self, timeout: Duration) {
        let parsed = match self.download(timeout).await {
            Ok(body) => {
                let zone = self.zone.clone();
                off_thread("parsing the feed", move || ical::parse(&body, &zone)).await
            }
            Err(err) => {
                let keeping = if self.events.is_some() { "keeping the previous data" } else { "no data yet" };
                error!("calendar: {err}; {keeping}");
                None
            }
        };
        if let Some(parsed) = parsed {
            let mut summary = format!("{} events in the feed", parsed.events.len());
            let unsupported = ical::unsupported_rules(&parsed.events);
            if unsupported > 0 {
                summary += &format!(", {unsupported} with an RRULE shown only on its first date");
            }
            if parsed.unknown_tzids > 0 {
                summary += &format!(", {} times with an unknown TZID read as {ZONE}", parsed.unknown_tzids);
            }
            if summary != self.summary {
                log!("calendar: {summary}");
                self.summary = summary;
            }
            self.events = Some(parsed.events.into());
        }
        let calendar = match self.events.clone() {
            Some(events) => {
                let (now, zone) = (Timestamp::now(), self.zone.clone());
                off_thread("expanding the feed", move || build(&events, now, &zone)).await
            }
            None => Some(Calendar::default()),
        };
        // If expanding failed, the page keeps showing the calendar published last time.
        if let Some(calendar) = calendar {
            publish(calendar);
        }
    }

    async fn download(&self, timeout: Duration) -> Result<Vec<u8>, String> {
        let client = self.client.as_ref().ok_or("no HTTP client")?;
        let mut response = client
            .get(site::CALENDAR_FEED_URL)
            .timeout(timeout)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|err| format!("download failed: {err}"))?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|err| format!("download failed: {err}"))? {
            if body.len() + chunk.len() > MAX_BYTES {
                return Err(format!("feed larger than {MAX_BYTES} bytes"));
            }
            body.extend_from_slice(&chunk);
        }
        // A login or error page answered with 200 must not wipe the calendar.
        let head = &body[..body.len().min(512)];
        if !String::from_utf8_lossy(head).contains("BEGIN:VCALENDAR") {
            return Err("the response is not an iCalendar feed".to_owned());
        }
        Ok(body)
    }
}

/// Runs CPU work on the blocking pool, off the async workers. None (logged) if it panicked.
pub(super) async fn off_thread<T: Send + 'static>(what: &str, work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(work).await.map_err(|err| error!("calendar: {what} failed: {err}")).ok()
}

/// The calendar as of `now`: events that have not ended (soonest first, up to 12, starting
/// within about six months) and those that have (latest first, up to 6, from the last year).
pub fn build(events: &[VEvent], now: Timestamp, zone: &TimeZone) -> Calendar {
    let from = now.checked_sub(BEHIND).unwrap_or(now);
    let to = now.checked_add(AHEAD).unwrap_or(now);
    let (mut upcoming, mut past): (Vec<Occurrence<'_>>, Vec<Occurrence<'_>>) = ical::expand(events, from, to, zone)
        .into_iter()
        .partition(|o| o.end > now || (o.end == o.start && o.start >= now));
    upcoming.sort_by_key(|o| (o.start, o.end));
    past.sort_by_key(|o| std::cmp::Reverse((o.start, o.end)));
    Calendar {
        loaded: true,
        upcoming: upcoming.iter().take(MAX_UPCOMING).map(|o| event(o, zone)).collect(),
        past: past.iter().take(MAX_PAST).map(|o| event(o, zone)).collect(),
    }
}

fn event(occurrence: &Occurrence<'_>, zone: &TimeZone) -> Event {
    let source = occurrence.event;
    let (start, end, all_day) = match occurrence.days {
        Some((first, last)) => (day(first, zone), day(last, zone), true),
        None => (local(occurrence.start, zone), local(occurrence.end, zone), false),
    };
    Event {
        title: text::plain(&source.summary, MAX_TITLE).unwrap_or_else(|| "Program".to_owned()),
        location: text::plain(&source.location, MAX_LOCATION),
        description: text::plain(&source.description, MAX_DESCRIPTION),
        start,
        end,
        all_day,
    }
}

pub(super) fn local(timestamp: Timestamp, zone: &TimeZone) -> LocalDateTime {
    let zoned = timestamp.to_zoned(zone.clone());
    LocalDateTime {
        date: date(zoned.date()),
        hour: zoned.hour().unsigned_abs(),
        minute: zoned.minute().unsigned_abs(),
        offset_minutes: i16::try_from(zoned.offset().seconds() / 60).unwrap_or(0),
    }
}

/// Midnight starting `date` in `zone` (for all-day events).
fn day(date_: jiff::civil::Date, zone: &TimeZone) -> LocalDateTime {
    let offset_minutes = ical::to_timestamp(date_.into(), zone)
        .map(|ts| zone.to_offset(ts).seconds() / 60)
        .and_then(|minutes| i16::try_from(minutes).ok())
        .unwrap_or(0);
    LocalDateTime { date: date(date_), hour: 0, minute: 0, offset_minutes }
}

fn date(date: jiff::civil::Date) -> Date {
    Date { year: date.year(), month: date.month().unsigned_abs(), day: date.day().unsigned_abs() }
}
