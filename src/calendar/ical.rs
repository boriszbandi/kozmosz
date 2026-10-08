//! A small iCalendar (RFC 5545) reader: exactly what a public Google Calendar feed needs.
//!
//! Hand-written instead of a crate: the subset is small (VEVENT, three DTSTART forms, a common
//! RRULE subset, EXDATE, RECURRENCE-ID), the iCal crates either do not expand recurrences or
//! pull in chrono + chrono-tz, and reading only the properties we show means ATTENDEE,
//! ORGANIZER and the like are never even stored.
//!
//! Time zones come from jiff's bundled tzdb, so daylight saving time is exact for any IANA TZID.

use std::collections::HashMap;

use jiff::{
    civil::{self, Weekday},
    tz::TimeZone,
    SignedDuration, Span, Timestamp, ToSpan,
};

/// Upper bound on VEVENTs read from one feed.
const MAX_EVENTS: usize = 20_000;
/// Upper bound on recurrence periods (days, weeks, months or years) walked for one event.
const MAX_PERIODS: i64 = 20_000;

/// A VEVENT, reduced to the properties the site uses.
#[derive(Clone, Debug)]
pub struct VEvent {
    pub uid: String,
    /// SUMMARY, LOCATION, DESCRIPTION with iCal escapes undone (may still contain HTML).
    pub summary: String,
    pub location: String,
    pub description: String,
    pub start: Moment,
    pub end: Option<Moment>,
    pub duration: Option<Span>,
    pub rrule: Option<String>,
    pub exdates: Vec<Moment>,
    pub recurrence_id: Option<Moment>,
    pub cancelled: bool,
}

/// A DTSTART / DTEND / EXDATE / RECURRENCE-ID value.
#[derive(Clone, Debug)]
pub enum Moment {
    /// `VALUE=DATE`: a whole day, independent of time zones.
    Date(civil::Date),
    /// A wall-clock time in a zone: UTC for "…Z", the TZID, or the calendar's zone (floating).
    DateTime(civil::DateTime, TimeZone),
}

/// What [`parse`] found, plus problems worth one log line.
#[derive(Debug, Default)]
pub struct Parsed {
    pub events: Vec<VEvent>,
    /// TZIDs the tzdb does not know; their times were read in the calendar's own zone.
    pub unknown_tzids: usize,
}

/// Parses a feed. `zone` is the calendar's zone, used for floating times and unknown TZIDs.
pub fn parse(raw: &[u8], zone: &TimeZone) -> Parsed {
    let text = unfold(raw);
    let mut parsed = Parsed::default();
    // Open components, innermost last. Properties count only directly inside a VEVENT, so a
    // VALARM's DESCRIPTION or a VTIMEZONE's DTSTART/RRULE never leak into an event.
    let mut stack: Vec<String> = Vec::new();
    let mut event: Option<Builder> = None;

    for line in text.lines() {
        let Some(line) = ContentLine::parse(line) else { continue };
        match line.name.as_str() {
            "BEGIN" => {
                let component = line.value.trim().to_ascii_uppercase();
                if component == "VEVENT" {
                    event = Some(Builder::default());
                }
                stack.push(component);
            }
            "END" => {
                let component = line.value.trim().to_ascii_uppercase();
                if let Some(depth) = stack.iter().rposition(|c| *c == component) {
                    stack.truncate(depth);
                }
                if component == "VEVENT" {
                    if let Some(built) = event.take().and_then(Builder::build) {
                        if parsed.events.len() < MAX_EVENTS {
                            parsed.events.push(built);
                        }
                    }
                }
            }
            _ if stack.last().map(String::as_str) == Some("VEVENT") => {
                if let Some(event) = event.as_mut() {
                    event.property(&line, zone, &mut parsed.unknown_tzids);
                }
            }
            _ => {}
        }
    }
    parsed
}

/// Joins folded lines (CRLF or LF followed by one space or tab). Works on bytes, because a fold
/// may split a multi-byte UTF-8 character.
pub fn unfold(raw: &[u8]) -> String {
    let raw = raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(raw);
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let fold = match &raw[i..] {
            [b'\r', b'\n', b' ' | b'\t', ..] => 3,
            [b'\n', b' ' | b'\t', ..] => 2,
            _ => 0,
        };
        if fold > 0 {
            i += fold;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Undoes TEXT escaping: `\\`, `\;`, `\,`, `\n` / `\N`.
pub fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// `NAME;PARAM=value;PARAM="quoted":value`
struct ContentLine<'a> {
    name: String,
    params: Vec<(String, &'a str)>,
    value: &'a str,
}

impl<'a> ContentLine<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        let end = line.find([';', ':'])?;
        let name = line[..end].trim().to_ascii_uppercase();
        let mut params = Vec::new();
        let mut rest = &line[end..];
        // Each round, `rest` starts at the ';' or ':' after the previous part.
        while let Some(after) = rest.strip_prefix(';') {
            let eq = after.find('=')?;
            let param = after[..eq].trim().to_ascii_uppercase();
            let value = &after[eq + 1..];
            let len = if let Some(quoted) = value.strip_prefix('"') {
                quoted.find('"')? + 2
            } else {
                value.find([';', ':']).unwrap_or(value.len())
            };
            params.push((param, value[..len].trim_matches('"')));
            rest = &value[len..];
        }
        let value = rest.strip_prefix(':')?;
        Some(Self { name, params, value })
    }

    fn param(&self, name: &str) -> Option<&'a str> {
        self.params.iter().find(|(n, _)| n == name).map(|&(_, v)| v)
    }
}

#[derive(Default)]
struct Builder {
    uid: String,
    summary: String,
    location: String,
    description: String,
    start: Option<Moment>,
    end: Option<Moment>,
    duration: Option<Span>,
    rrule: Option<String>,
    exdates: Vec<Moment>,
    recurrence_id: Option<Moment>,
    cancelled: bool,
}

impl Builder {
    /// Takes the properties the site uses and ignores everything else (ATTENDEE, ORGANIZER, …).
    fn property(&mut self, line: &ContentLine<'_>, zone: &TimeZone, unknown_tzids: &mut usize) {
        let moment = |value: &str, unknown: &mut usize| {
            parse_moment(value, line.param("VALUE"), line.param("TZID"), zone, unknown)
        };
        match line.name.as_str() {
            "UID" => self.uid = line.value.trim().to_owned(),
            "SUMMARY" => self.summary = unescape(line.value),
            "LOCATION" => self.location = unescape(line.value),
            "DESCRIPTION" => self.description = unescape(line.value),
            "DTSTART" => self.start = moment(line.value, unknown_tzids),
            "DTEND" => self.end = moment(line.value, unknown_tzids),
            "DURATION" => self.duration = parse_duration(line.value),
            "RRULE" => self.rrule = Some(line.value.trim().to_owned()),
            "EXDATE" => {
                for value in line.value.split(',') {
                    self.exdates.extend(moment(value, unknown_tzids));
                }
            }
            "RECURRENCE-ID" => self.recurrence_id = moment(line.value, unknown_tzids),
            "STATUS" => self.cancelled = line.value.trim().eq_ignore_ascii_case("CANCELLED"),
            _ => {}
        }
    }

    /// An event without a readable DTSTART is dropped.
    fn build(self) -> Option<VEvent> {
        Some(VEvent {
            uid: self.uid,
            summary: self.summary,
            location: self.location,
            description: self.description,
            start: self.start?,
            end: self.end,
            duration: self.duration,
            rrule: self.rrule,
            exdates: self.exdates,
            recurrence_id: self.recurrence_id,
            cancelled: self.cancelled,
        })
    }
}

/// `20261008` (date), `20261008T160000Z` (UTC), `20261008T180000` with or without TZID.
fn parse_moment(
    value: &str,
    value_type: Option<&str>,
    tzid: Option<&str>,
    zone: &TimeZone,
    unknown_tzids: &mut usize,
) -> Option<Moment> {
    let value = value.trim();
    if value_type.is_some_and(|t| t.eq_ignore_ascii_case("DATE")) || value.len() == 8 {
        return parse_date(value.get(..8)?).map(Moment::Date);
    }
    let (local, utc) = match value.strip_suffix(['Z', 'z']) {
        Some(local) => (local, true),
        None => (value, false),
    };
    let datetime = parse_datetime(local)?;
    let tz = if utc {
        TimeZone::UTC
    } else if let Some(tzid) = tzid {
        TimeZone::get(tzid.trim()).unwrap_or_else(|_| {
            *unknown_tzids += 1;
            zone.clone()
        })
    } else {
        zone.clone()
    };
    Some(Moment::DateTime(datetime, tz))
}

fn digits(s: &str) -> Option<i32> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

fn parse_date(s: &str) -> Option<civil::Date> {
    if s.len() != 8 {
        return None;
    }
    let year = i16::try_from(digits(s.get(0..4)?)?).ok()?;
    let month = i8::try_from(digits(s.get(4..6)?)?).ok()?;
    let day = i8::try_from(digits(s.get(6..8)?)?).ok()?;
    civil::Date::new(year, month, day).ok()
}

/// `YYYYMMDDTHHMMSS`
fn parse_datetime(s: &str) -> Option<civil::DateTime> {
    if s.len() != 15 || s.as_bytes()[8] != b'T' {
        return None;
    }
    let date = parse_date(&s[..8])?;
    let hour = i8::try_from(digits(s.get(9..11)?)?).ok()?;
    let minute = i8::try_from(digits(s.get(11..13)?)?).ok()?;
    // 60 is a leap second; read it as 59.
    let second = i8::try_from(digits(s.get(13..15)?)?).ok()?.min(59);
    let time = civil::Time::new(hour, minute, second, 0).ok()?;
    Some(date.to_datetime(time))
}

/// `P1D`, `PT1H30M`, `P2W`, `-PT15M` (only positive durations are useful for an end time).
fn parse_duration(value: &str) -> Option<Span> {
    let value = value.trim().strip_prefix('+').unwrap_or(value.trim());
    let mut rest = value.strip_prefix('P')?;
    let mut span = Span::new();
    let mut in_time = false;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('T') {
            in_time = true;
            rest = after;
            continue;
        }
        let len = rest.find(|c: char| !c.is_ascii_digit())?;
        let n: i64 = rest[..len].parse().ok()?;
        let unit = rest[len..].chars().next()?;
        span = match (unit, in_time) {
            ('W', false) => span.try_weeks(n).ok()?,
            ('D', false) => span.try_days(n).ok()?,
            ('H', true) => span.try_hours(n).ok()?,
            ('M', true) => span.try_minutes(n).ok()?,
            ('S', true) => span.try_seconds(n).ok()?,
            _ => return None,
        };
        rest = &rest[len + 1..];
    }
    Some(span)
}

impl Moment {
    /// The wall-clock value (00:00 for a date).
    fn civil(&self) -> civil::DateTime {
        match self {
            Moment::Date(date) => date.to_datetime(civil::Time::midnight()),
            Moment::DateTime(datetime, _) => *datetime,
        }
    }

    /// The zone the wall-clock value is in (`calendar` for dates).
    fn zone(&self, calendar: &TimeZone) -> TimeZone {
        match self {
            Moment::Date(_) => calendar.clone(),
            Moment::DateTime(_, tz) => tz.clone(),
        }
    }

    fn timestamp(&self, calendar: &TimeZone) -> Option<Timestamp> {
        to_timestamp(self.civil(), &self.zone(calendar))
    }

    /// Whether this EXDATE / RECURRENCE-ID names the instance starting at `instance` (wall-clock
    /// time in `zone`). A date matches any instance on that day.
    fn matches(&self, instance: civil::DateTime, zone: &TimeZone, all_day: bool) -> bool {
        match self {
            Moment::Date(date) => instance.date() == *date,
            Moment::DateTime(datetime, tz) if all_day => {
                to_timestamp(*datetime, tz).map(|ts| zone.to_datetime(ts).date()) == Some(instance.date())
            }
            Moment::DateTime(datetime, tz) => {
                let exact = to_timestamp(*datetime, tz);
                exact.is_some() && exact == to_timestamp(instance, zone)
            }
        }
    }
}

/// Wall-clock time to an instant. A time skipped by the spring-forward gap moves forward by the
/// gap (02:30 → 03:30 CEST), and an ambiguous autumn time takes the earlier offset (RFC 5545).
pub fn to_timestamp(datetime: civil::DateTime, zone: &TimeZone) -> Option<Timestamp> {
    zone.to_ambiguous_zoned(datetime).compatible().ok().map(|z| z.timestamp())
}

/// One instance of an event.
#[derive(Clone, Debug)]
pub struct Occurrence<'a> {
    pub event: &'a VEvent,
    pub start: Timestamp,
    /// Exclusive; equal to `start` for a timed event without an end.
    pub end: Timestamp,
    /// All-day events: the first and the last (inclusive) day.
    pub days: Option<(civil::Date, civil::Date)>,
}

/// How many recurring events have a rule this reader cannot expand (only their first instance
/// is shown).
pub fn unsupported_rules(events: &[VEvent]) -> usize {
    events.iter().filter(|e| e.rrule.as_deref().is_some_and(|r| Rule::parse(r).is_err())).count()
}

/// Every instance of `events` that overlaps `from..to`. Cancelled events and instances
/// (EXDATE, cancelled or moved RECURRENCE-ID overrides) are left out.
pub fn expand<'a>(events: &'a [VEvent], from: Timestamp, to: Timestamp, zone: &TimeZone) -> Vec<Occurrence<'a>> {
    // Instances replaced by an override, per UID.
    let mut overridden: HashMap<&str, Vec<&Moment>> = HashMap::new();
    for event in events.iter().filter(|e| !e.uid.is_empty()) {
        if let Some(id) = &event.recurrence_id {
            overridden.entry(event.uid.as_str()).or_default().push(id);
        }
    }

    let mut out = Vec::new();
    for event in events {
        if event.recurrence_id.is_some() {
            // An override of one instance of a recurring event: shown on its own.
            if !event.cancelled {
                push_occurrence(&mut out, event, event.start.civil(), from, to, zone);
            }
            continue;
        }
        if event.cancelled {
            continue;
        }
        let event_zone = event.start.zone(zone);
        let all_day = matches!(event.start, Moment::Date(_));
        let starts = match event.rrule.as_deref().map(Rule::parse) {
            Some(Ok(rule)) => rule.instances(event.start.civil(), &event_zone, all_day, from, to),
            None | Some(Err(Unsupported)) => vec![event.start.civil()],
        };
        let moved: &[&Moment] = overridden.get(event.uid.as_str()).map_or(&[], Vec::as_slice);
        for start in starts {
            let excluded = event
                .exdates
                .iter()
                .chain(moved.iter().copied())
                .any(|moment| moment.matches(start, &event_zone, all_day));
            if !excluded {
                push_occurrence(&mut out, event, start, from, to, zone);
            }
        }
    }
    out
}

/// Adds the instance of `event` starting at `start` (wall-clock time in the event's zone) when it
/// overlaps `from..to`.
fn push_occurrence<'a>(
    out: &mut Vec<Occurrence<'a>>,
    event: &'a VEvent,
    start: civil::DateTime,
    from: Timestamp,
    to: Timestamp,
    zone: &TimeZone,
) {
    let occurrence = match &event.start {
        Moment::Date(first_day) => {
            // DTEND of an all-day event is exclusive; without one the event lasts one day.
            let days = match (&event.end, event.duration) {
                (Some(end), _) => (end.civil().date() - *first_day).get_days(),
                (None, Some(duration)) => duration.get_days() + duration.get_weeks() * 7,
                (None, None) => 1,
            }
            .max(1);
            let first = start.date();
            // A DURATION of weeks plus days can exceed what a Span holds: drop the event.
            let Some(next) = day_span(i64::from(days)).and_then(|span| first.checked_add(span).ok()) else {
                return;
            };
            let Ok(last) = next.yesterday() else { return };
            let (Some(start), Some(end)) =
                (to_timestamp(first.to_datetime(civil::Time::midnight()), zone), to_timestamp(next.into(), zone))
            else {
                return;
            };
            Occurrence { event, start, end, days: Some((first, last)) }
        }
        Moment::DateTime(_, event_zone) => {
            let Some(start_ts) = to_timestamp(start, event_zone) else { return };
            // Every instance lasts exactly as long as the first one (RFC 5545 3.8.5.3).
            let end = match (&event.end, event.duration) {
                (Some(end), _) => end
                    .timestamp(zone)
                    .zip(event.start.timestamp(zone))
                    .map(|(end, first)| end.duration_since(first))
                    .and_then(|length| start_ts.checked_add(length.max(SignedDuration::ZERO)).ok()),
                (None, Some(duration)) => start_ts
                    .to_zoned(event_zone.clone())
                    .checked_add(duration)
                    .ok()
                    .map(|z| z.timestamp()),
                (None, None) => Some(start_ts),
            };
            let end = end.unwrap_or(start_ts).max(start_ts);
            Occurrence { event, start: start_ts, end, days: None }
        }
    };
    // Overlap test; a zero-length event counts when it starts inside the window.
    if occurrence.start < to && (occurrence.end > from || occurrence.start >= from) {
        out.push(occurrence);
    }
}

/// A recurrence rule this reader can expand.
#[derive(Debug)]
pub struct Unsupported;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug)]
enum Until {
    Instant(Timestamp),
    Date(civil::Date),
    Local(civil::DateTime),
}

/// RRULE with FREQ=DAILY/WEEKLY/MONTHLY/YEARLY, INTERVAL, COUNT, UNTIL, WKST, BYDAY (an
/// ordinal like 2TU or -1FR only with MONTHLY), BYMONTHDAY (MONTHLY) and BYMONTH.
#[derive(Debug)]
struct Rule {
    freq: Freq,
    interval: i64,
    count: Option<u32>,
    until: Option<Until>,
    week_start: Weekday,
    /// (ordinal, weekday); ordinal 0 means every such weekday of the period.
    by_day: Vec<(i8, Weekday)>,
    by_month_day: Vec<i8>,
    by_month: Vec<i8>,
}

impl Rule {
    fn parse(rrule: &str) -> Result<Self, Unsupported> {
        let mut freq = None;
        let mut rule = Rule {
            freq: Freq::Daily,
            interval: 1,
            count: None,
            until: None,
            week_start: Weekday::Monday,
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
        };
        for part in rrule.split(';').filter(|p| !p.trim().is_empty()) {
            let (key, value) = part.split_once('=').ok_or(Unsupported)?;
            let value = value.trim();
            match key.trim().to_ascii_uppercase().as_str() {
                "FREQ" => {
                    freq = Some(match value.to_ascii_uppercase().as_str() {
                        "DAILY" => Freq::Daily,
                        "WEEKLY" => Freq::Weekly,
                        "MONTHLY" => Freq::Monthly,
                        "YEARLY" => Freq::Yearly,
                        _ => return Err(Unsupported),
                    })
                }
                "INTERVAL" => rule.interval = value.parse::<i64>().ok().filter(|&n| n >= 1).ok_or(Unsupported)?,
                "COUNT" => rule.count = Some(value.parse::<u32>().ok().filter(|&n| n >= 1).ok_or(Unsupported)?),
                "UNTIL" => rule.until = Some(parse_until(value).ok_or(Unsupported)?),
                "WKST" => rule.week_start = weekday(value).ok_or(Unsupported)?,
                "BYDAY" => {
                    for day in value.split(',') {
                        let day = day.trim();
                        let split = day.len().checked_sub(2).ok_or(Unsupported)?;
                        let (ordinal, name) = day.split_at_checked(split).ok_or(Unsupported)?;
                        let ordinal = match ordinal.strip_prefix('+').unwrap_or(ordinal) {
                            "" => 0,
                            n => n.parse::<i8>().ok().filter(|n| (-5..=5).contains(n) && *n != 0).ok_or(Unsupported)?,
                        };
                        rule.by_day.push((ordinal, weekday(name).ok_or(Unsupported)?));
                    }
                }
                "BYMONTHDAY" => {
                    for day in value.split(',') {
                        let day = day.trim().parse::<i8>().map_err(|_| Unsupported)?;
                        if day == 0 || !(-31..=31).contains(&day) {
                            return Err(Unsupported);
                        }
                        rule.by_month_day.push(day);
                    }
                }
                "BYMONTH" => {
                    for month in value.split(',') {
                        let month = month.trim().parse::<i8>().map_err(|_| Unsupported)?;
                        if !(1..=12).contains(&month) {
                            return Err(Unsupported);
                        }
                        rule.by_month.push(month);
                    }
                }
                // BYSETPOS, BYYEARDAY, BYWEEKNO, BYHOUR, BYMINUTE, BYSECOND: not supported.
                key if key.starts_with("BY") => return Err(Unsupported),
                // Unknown extension parts do not change the dates.
                _ => {}
            }
        }
        rule.freq = freq.ok_or(Unsupported)?;
        let ordinals = rule.by_day.iter().any(|&(n, _)| n != 0);
        let supported = match rule.freq {
            Freq::Daily | Freq::Weekly => !ordinals && rule.by_month_day.is_empty(),
            Freq::Monthly => true,
            Freq::Yearly => rule.by_day.is_empty() && rule.by_month_day.is_empty() && rule.by_month.is_empty(),
        };
        if supported {
            Ok(rule)
        } else {
            Err(Unsupported)
        }
    }

    /// Start of every instance up to `to` (DTSTART first, as RFC 5545 counts it), as wall-clock
    /// times in `zone`. Instances before `from` are skipped when COUNT does not need them.
    fn instances(
        &self,
        start: civil::DateTime,
        zone: &TimeZone,
        all_day: bool,
        from: Timestamp,
        to: Timestamp,
    ) -> Vec<civil::DateTime> {
        let mut out = vec![start];
        let mut count = 1u32;
        let last_date = zone.to_datetime(to).date();
        // Without COUNT, nothing before the window matters: jump close to it. One period of
        // slack covers events that started before `from` and still run.
        let first_period = if self.count.is_none() {
            let from_date = zone.to_datetime(from).date();
            (self.periods_between(start.date(), from_date) / self.interval - 1).max(0)
        } else {
            0
        };
        'periods: for k in first_period..first_period + MAX_PERIODS {
            let Some((period_start, candidates)) = self.period(start, k) else { break };
            if period_start > last_date {
                break;
            }
            for candidate in candidates {
                if candidate <= start {
                    continue;
                }
                if self.count.is_some_and(|n| count >= n) || self.after_until(candidate, zone, all_day) {
                    break 'periods;
                }
                count += 1;
                out.push(candidate);
            }
        }
        out
    }

    /// Whole periods (days, weeks, months, years) from `start` to `date`.
    fn periods_between(&self, start: civil::Date, date: civil::Date) -> i64 {
        if date <= start {
            return 0;
        }
        let days = i64::from((date - start).get_days());
        let months = (i64::from(date.year()) * 12 + i64::from(date.month()))
            - (i64::from(start.year()) * 12 + i64::from(start.month()));
        match self.freq {
            Freq::Daily => days,
            Freq::Weekly => days / 7,
            Freq::Monthly => months,
            Freq::Yearly => months / 12,
        }
    }

    /// The `k`th period: its first day, and its candidate starts in order (DTSTART's time of day).
    fn period(&self, start: civil::DateTime, k: i64) -> Option<(civil::Date, Vec<civil::DateTime>)> {
        // Checked all the way: INTERVAL comes from the feed and may be absurdly large. Going out of
        // range ends the series.
        let step = k.checked_mul(self.interval)?;
        let date = start.date();
        let (period_start, mut dates) = match self.freq {
            Freq::Daily => {
                let day = date.checked_add(day_span(step)?).ok()?;
                (day, vec![day])
            }
            Freq::Weekly => {
                let back = i64::from(date.weekday().since(self.week_start));
                let week = date.checked_add(day_span(step.checked_mul(7)?.checked_sub(back)?)?).ok()?;
                let days: Vec<Weekday> = if self.by_day.is_empty() {
                    vec![date.weekday()]
                } else {
                    self.by_day.iter().map(|&(_, wd)| wd).collect()
                };
                let dates = days
                    .into_iter()
                    .filter_map(|wd| week.checked_add(i64::from(wd.since(self.week_start)).days()).ok())
                    .collect();
                (week, dates)
            }
            Freq::Monthly => {
                let months = (i64::from(date.year()) * 12 + i64::from(date.month()) - 1).checked_add(step)?;
                let year = i16::try_from(months.div_euclid(12)).ok()?;
                let month = i8::try_from(months.rem_euclid(12) + 1).ok()?;
                let first = civil::Date::new(year, month, 1).ok()?;
                (first, self.month_days(first, date.day()))
            }
            Freq::Yearly => {
                let year = i16::try_from(i64::from(date.year()).checked_add(step)?).ok()?;
                let first = civil::Date::new(year, 1, 1).ok()?;
                // Feb 29 is skipped in common years, not moved.
                (first, civil::Date::new(year, date.month(), date.day()).ok().into_iter().collect())
            }
        };
        dates.retain(|d| {
            (self.by_month.is_empty() || self.by_month.contains(&d.month()))
                && (self.freq != Freq::Daily
                    || self.by_day.is_empty()
                    || self.by_day.iter().any(|&(_, wd)| wd == d.weekday()))
        });
        dates.sort();
        dates.dedup();
        Some((period_start, dates.into_iter().map(|d| d.to_datetime(start.time())).collect()))
    }

    /// Days of the month starting at `first` selected by BYDAY / BYMONTHDAY (both: days in both),
    /// or the day of DTSTART (`day`) when neither is given; a day the month lacks is skipped.
    fn month_days(&self, first: civil::Date, day: i8) -> Vec<civil::Date> {
        let length = first.days_in_month();
        let at = |d: i8| first.checked_add(i64::from(d - 1).days()).ok();
        if self.by_day.is_empty() && self.by_month_day.is_empty() {
            return (day <= length).then(|| at(day)).flatten().into_iter().collect();
        }
        let by_month_day: Vec<i8> = self
            .by_month_day
            .iter()
            .map(|&d| if d > 0 { d } else { length + 1 + d })
            .filter(|&d| (1..=length).contains(&d))
            .collect();
        (1..=length)
            .filter_map(at)
            .filter(|date| self.by_month_day.is_empty() || by_month_day.contains(&date.day()))
            .filter(|date| {
                self.by_day.is_empty()
                    || self.by_day.iter().any(|&(nth, wd)| {
                        date.weekday() == wd
                            && (nth == 0 || first.nth_weekday_of_month(nth, wd).ok() == Some(*date))
                    })
            })
            .collect()
    }

    fn after_until(&self, candidate: civil::DateTime, zone: &TimeZone, all_day: bool) -> bool {
        match &self.until {
            None => false,
            Some(Until::Date(date)) => candidate.date() > *date,
            Some(Until::Local(datetime)) if all_day => candidate.date() > datetime.date(),
            Some(Until::Local(datetime)) => candidate > *datetime,
            Some(Until::Instant(until)) => to_timestamp(candidate, zone).is_some_and(|ts| ts > *until),
        }
    }
}

fn parse_until(value: &str) -> Option<Until> {
    if value.len() == 8 {
        return parse_date(value).map(Until::Date);
    }
    match value.strip_suffix(['Z', 'z']) {
        Some(local) => to_timestamp(parse_datetime(local)?, &TimeZone::UTC).map(Until::Instant),
        None => parse_datetime(value).map(Until::Local),
    }
}

/// `n` days, or None when a Span cannot hold that many (`n.days()` would panic).
fn day_span(n: i64) -> Option<Span> {
    Span::new().try_days(n).ok()
}

fn weekday(name: &str) -> Option<Weekday> {
    Some(match name.trim().to_ascii_uppercase().as_str() {
        "MO" => Weekday::Monday,
        "TU" => Weekday::Tuesday,
        "WE" => Weekday::Wednesday,
        "TH" => Weekday::Thursday,
        "FR" => Weekday::Friday,
        "SA" => Weekday::Saturday,
        "SU" => Weekday::Sunday,
        _ => return None,
    })
}
