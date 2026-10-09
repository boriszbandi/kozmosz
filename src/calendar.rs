//! The club's programme, taken from its public Google Calendar.
//!
//! The server downloads the iCal feed ([`Feed`], server only), expands it into the upcoming and
//! recent events as of "now" ([`Calendar`]) and publishes the result; pages read it with
//! [`current`]. The types and the Hungarian formatting helpers compile in both builds; the
//! hydrate build never receives data, so there `current()` is simply empty.

use std::sync::{Arc, PoisonError, RwLock};

#[cfg(feature = "ssr")]
mod feed;
#[cfg(feature = "ssr")]
mod ical;
#[cfg(feature = "ssr")]
mod text;
#[cfg(all(test, feature = "ssr"))]
mod tests;

#[cfg(feature = "ssr")]
pub use feed::{Feed, REFRESH_EVERY};

/// What the programs page shows.
#[derive(Clone, Debug, Default)]
pub struct Calendar {
    /// The feed has been downloaded at least once. When false the lists are empty because the
    /// data is missing, not because nothing is planned.
    pub loaded: bool,
    /// Events that have not ended yet, soonest first.
    pub upcoming: Vec<Event>,
    /// Events that have ended, most recent first.
    pub past: Vec<Event>,
    /// Budapest date at the time of the refresh (for "Ma" labels).
    pub today: Option<Date>,
}

/// One occurrence of a calendar event, ready to display. All text is plain text (no HTML, no
/// e-mail addresses); render it escaped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub title: String,
    pub location: Option<String>,
    pub description: Option<String>,
    /// Start in Budapest time. All-day events: the first day, 00:00.
    pub start: LocalDateTime,
    /// End in Budapest time (equal to `start` when the feed gives no end). All-day events: the
    /// last day itself (inclusive), 00:00.
    pub end: LocalDateTime,
    pub all_day: bool,
}

/// A calendar date (proleptic Gregorian).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i16,
    /// 1..=12
    pub month: u8,
    /// 1..=31
    pub day: u8,
}

/// Wall-clock time in Budapest, with the UTC offset in force at that moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDateTime {
    pub date: Date,
    pub hour: u8,
    pub minute: u8,
    /// Minutes east of UTC: 60 in winter (CET), 120 in summer (CEST).
    pub offset_minutes: i16,
}

static CURRENT: RwLock<Option<Arc<Calendar>>> = RwLock::new(None);

/// The calendar as of the last refresh (empty before the first one and in the browser).
pub fn current() -> Arc<Calendar> {
    CURRENT.read().unwrap_or_else(PoisonError::into_inner).clone().unwrap_or_default()
}

/// Replaces what [`current`] returns.
#[cfg(feature = "ssr")]
fn publish(calendar: Calendar) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(calendar));
}

const MONTHS: [&str; 12] = [
    "január",
    "február",
    "március",
    "április",
    "május",
    "június",
    "július",
    "augusztus",
    "szeptember",
    "október",
    "november",
    "december",
];
const WEEKDAYS: [&str; 7] = ["hétfő", "kedd", "szerda", "csütörtök", "péntek", "szombat", "vasárnap"];
/// The usual Hungarian month abbreviations.
const MONTHS_SHORT: [&str; 12] =
    ["jan.", "febr.", "márc.", "ápr.", "máj.", "jún.", "júl.", "aug.", "szept.", "okt.", "nov.", "dec."];

impl Date {
    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn days_since_epoch(self) -> i64 {
        let month = i64::from(self.month);
        let year = i64::from(self.year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// 0 = Monday … 6 = Sunday.
    pub fn weekday(self) -> usize {
        // 1970-01-01 was a Thursday (3).
        (self.days_since_epoch() + 3).rem_euclid(7) as usize
    }

    /// "2026. október 8., csütörtök"
    pub fn long(self) -> String {
        format!("{}. {}", self.year, self.without_year())
    }

    /// "október 8., csütörtök"
    pub fn without_year(self) -> String {
        let month = MONTHS.get(usize::from(self.month).wrapping_sub(1)).copied().unwrap_or("");
        format!("{month} {}., {}", self.day, WEEKDAYS[self.weekday()])
    }

    /// "okt."
    pub fn month_short(self) -> &'static str {
        MONTHS_SHORT.get(usize::from(self.month).wrapping_sub(1)).copied().unwrap_or("")
    }

    /// "okt. 8."
    pub fn short(self) -> String {
        format!("{} {}.", self.month_short(), self.day)
    }

    /// "csütörtök"
    pub fn weekday_name(self) -> &'static str {
        WEEKDAYS[self.weekday()]
    }

    /// "2026-10-08", for `<time datetime>`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl LocalDateTime {
    /// "18:00", "9:30"
    pub fn clock(self) -> String {
        format!("{}:{:02}", self.hour, self.minute)
    }

    /// "2026-10-08T18:00+02:00", for `<time datetime>`.
    pub fn iso(self) -> String {
        let sign = if self.offset_minutes < 0 { '-' } else { '+' };
        let offset = self.offset_minutes.unsigned_abs();
        format!(
            "{}T{:02}:{:02}{sign}{:02}:{:02}",
            self.date.iso(),
            self.hour,
            self.minute,
            offset / 60,
            offset % 60
        )
    }
}

impl Event {
    /// Machine-readable start for `<time datetime>`: a date for all-day events.
    pub fn datetime_attr(&self) -> String {
        if self.all_day {
            self.start.date.iso()
        } else {
            self.start.iso()
        }
    }

    /// "2026. október 8., csütörtök". Events over several days get a range, e.g.
    /// "2026. október 9., péntek - október 11., vasárnap"; a timed one includes the clock times.
    pub fn date_label(&self) -> String {
        let (start, end) = (self.start, self.end);
        if self.all_day {
            if end.date <= start.date {
                start.date.long()
            } else {
                format!("{} - {}", start.date.long(), end_date(start.date, end.date))
            }
        } else if self.within_a_day() {
            start.date.long()
        } else {
            format!(
                "{} {} - {} {}",
                start.date.long(),
                start.clock(),
                end_date(start.date, end.date),
                end.clock()
            )
        }
    }

    /// "18:00-20:00" ("18:00" without an end). None for all-day events and for events over
    /// several days, whose times are part of [`Event::date_label`].
    pub fn time_label(&self) -> Option<String> {
        if self.all_day || !self.within_a_day() {
            return None;
        }
        Some(if self.end == self.start {
            self.start.clock()
        } else {
            format!("{}-{}", self.start.clock(), self.end.clock())
        })
    }

    /// Ends the same day, or overnight before the start time (an observation from 20:00 to 2:00).
    fn within_a_day(&self) -> bool {
        let days = self.end.date.days_since_epoch() - self.start.date.days_since_epoch();
        days == 0 || (days == 1 && (self.end.hour, self.end.minute) <= (self.start.hour, self.start.minute))
    }
}

/// The end date of a range: without the year when it is the same as the start's.
fn end_date(start: Date, end: Date) -> String {
    if start.year == end.year {
        end.without_year()
    } else {
        end.long()
    }
}

/// A piece of an event description: plain text or a web link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    /// An http(s) URL, also used as the link text.
    Link(&'a str),
}

/// Splits plain text into text and http(s) links. Trailing punctuation stays outside the link,
/// and a URL cut short by the "…" of a shortened description is left as text.
pub fn linkify(text: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    // `text[pending..]` is text not yet emitted; the next URL is searched from `pos`.
    let mut pending = 0;
    let mut pos = 0;
    while let Some(found) = find_url(&text[pos..]) {
        let at = pos + found;
        let candidate = &text[at..];
        let len = candidate
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '…'))
            .unwrap_or(candidate.len());
        let url = trim_url(&candidate[..len]);
        let cut_short = candidate[url.len()..].starts_with('…');
        let has_host = url.split_once("://").is_some_and(|(_, host)| host.len() > 1);
        if cut_short || !has_host {
            pos = at + "http".len();
            continue;
        }
        if at > pending {
            segments.push(Segment::Text(&text[pending..at]));
        }
        segments.push(Segment::Link(url));
        pos = at + url.len();
        pending = pos;
    }
    if pending < text.len() {
        segments.push(Segment::Text(&text[pending..]));
    }
    segments
}

/// Drops sentence punctuation after a URL, and a closing parenthesis it did not open.
fn trim_url(mut url: &str) -> &str {
    loop {
        let mut trimmed = url.trim_end_matches(['.', ',', ';', ':', '!', '?', '\'']);
        if let Some(inner) = trimmed.strip_suffix(')') {
            if trimmed.matches('(').count() < trimmed.matches(')').count() {
                trimmed = inner;
            }
        }
        if trimmed.len() == url.len() {
            return url;
        }
        url = trimmed;
    }
}

fn find_url(text: &str) -> Option<usize> {
    match (text.find("https://"), text.find("http://")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}
