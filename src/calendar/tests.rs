//! Tests on a synthetic feed. The real feed is never copied into the repository: it contains
//! attendee data.

use jiff::{civil, tz::TimeZone, Timestamp};

use super::{
    feed::{build, local},
    ical::{expand, parse, unescape, unfold, unsupported_rules, Moment, VEvent},
    linkify, text, Date, Event, LocalDateTime, Segment,
};

/// Lines of the synthetic feed; joined with CRLF. A line starting with a space or a tab
/// continues the previous one (folding).
const FIXTURE: &[&str] = &[
    "BEGIN:VCALENDAR",
    "PRODID:-//Kozmosz//Teszt//HU",
    "VERSION:2.0",
    "X-WR-TIMEZONE:Europe/Budapest",
    // A VTIMEZONE's DTSTART and RRULE must not be read as an event.
    "BEGIN:VTIMEZONE",
    "TZID:Europe/Budapest",
    "BEGIN:DAYLIGHT",
    "TZOFFSETFROM:+0100",
    "TZOFFSETTO:+0200",
    "DTSTART:19700329T020000",
    "RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU",
    "END:DAYLIGHT",
    "BEGIN:STANDARD",
    "TZOFFSETFROM:+0200",
    "TZOFFSETTO:+0100",
    "DTSTART:19701025T030000",
    "RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU",
    "END:STANDARD",
    "END:VTIMEZONE",
    // UTC form, folding, escapes, attendees, an alarm.
    "BEGIN:VEVENT",
    "UID:utc@test",
    "DTSTART:20261008T160000Z",
    "DTEND:20261008T180000Z",
    "SUMMARY:Holdfigyelés a rakparton\\, táv",
    " csövekkel",
    "LOCATION:Műegyetem rakpart\\; a K épület előtt",
    "DESCRIPTION:Hozz meleg ruhát!\\nRészletek: https://kozmosz.bme.hu/programjaink.",
    "\t Kérdés: kiss.peter@example.com vagy bme.kozmosz@gmail.com \\\\o/",
    "ATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;CN=\"Kiss Péter: tag\";X-NUM-GUESTS=0:mailto:kiss.peter@example.com",
    "ORGANIZER;CN=Kozmosz:mailto:bme.kozmosz@gmail.com",
    "STATUS:CONFIRMED",
    "BEGIN:VALARM",
    "ACTION:DISPLAY",
    "DESCRIPTION:This is an event reminder",
    "TRIGGER:-P0DT0H30M0S",
    "END:VALARM",
    "END:VEVENT",
    // TZID form.
    "BEGIN:VEVENT",
    "UID:tzid@test",
    "DTSTART;TZID=Europe/Budapest:20261015T180000",
    "DTEND;TZID=Europe/Budapest:20261015T200000",
    "SUMMARY:Előadás: a MASAT-1 története",
    "DESCRIPTION:<html-blob>Előadónk <b>a projekt</b> vezetője.<br>Belépés <a href=\"https://example.com/regisztracio\">regisztrációval</a> &amp; ingyenes.</html-blob>",
    "END:VEVENT",
    // All-day form, two days (DTEND is exclusive).
    "BEGIN:VEVENT",
    "UID:allday@test",
    "DTSTART;VALUE=DATE:20261024",
    "DTEND;VALUE=DATE:20261026",
    "SUMMARY:Túra a Pilisben",
    "LOCATION:\"Site: Iroda\" <terem@example.com>\\n1111 Budapest",
    "END:VEVENT",
    // Cancelled.
    "BEGIN:VEVENT",
    "UID:cancelled@test",
    "DTSTART:20261009T160000Z",
    "DTEND:20261009T170000Z",
    "SUMMARY:Elmarad",
    "STATUS:CANCELLED",
    "END:VEVENT",
    // Weekly on Tuesdays 18:00 Budapest time, across the March DST change, with UNTIL and EXDATE.
    "BEGIN:VEVENT",
    "UID:weekly-until@test",
    "DTSTART;TZID=Europe/Budapest:20260303T180000",
    "DTEND;TZID=Europe/Budapest:20260303T200000",
    "RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20260421T160000Z;BYDAY=TU",
    "EXDATE;TZID=Europe/Budapest:20260317T180000",
    "SUMMARY:Asztro tanfolyam",
    "END:VEVENT",
    // Every other week on Monday and Thursday, COUNT=4, across the October DST change.
    "BEGIN:VEVENT",
    "UID:weekly-count@test",
    "DTSTART;TZID=Europe/Budapest:20261019T190000",
    "DTEND;TZID=Europe/Budapest:20261019T210000",
    "RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH;COUNT=4",
    "SUMMARY:KSP tanfolyam",
    "END:VEVENT",
    // A UTC-anchored series keeps its UTC time, so the Budapest time moves with DST.
    "BEGIN:VEVENT",
    "UID:weekly-utc@test",
    "DTSTART:20261021T170000Z",
    "DTEND:20261021T180000Z",
    "RRULE:FREQ=WEEKLY;COUNT=2",
    "SUMMARY:ISS SSTV vétel",
    "END:VEVENT",
    // Second Tuesday of the month.
    "BEGIN:VEVENT",
    "UID:monthly-byday@test",
    "DTSTART;TZID=Europe/Budapest:20261013T180000",
    "DTEND;TZID=Europe/Budapest:20261013T190000",
    "RRULE:FREQ=MONTHLY;BYDAY=2TU;COUNT=3",
    "SUMMARY:Havi gyűlés",
    "END:VEVENT",
    // Monthly on the 31st: months without one are skipped, not clamped.
    "BEGIN:VEVENT",
    "UID:monthly-31@test",
    "DTSTART;VALUE=DATE:20260131",
    "DTEND;VALUE=DATE:20260201",
    "RRULE:FREQ=MONTHLY;COUNT=4",
    "SUMMARY:Hónap vége",
    "END:VEVENT",
    // Every third day.
    "BEGIN:VEVENT",
    "UID:daily@test",
    "DTSTART:20261101T090000Z",
    "DTEND:20261101T100000Z",
    "RRULE:FREQ=DAILY;INTERVAL=3;UNTIL=20261110T090000Z",
    "SUMMARY:NOAA vétel",
    "END:VEVENT",
    // A series with one instance moved by an override (RECURRENCE-ID).
    "BEGIN:VEVENT",
    "UID:series@test",
    "DTSTART;TZID=Europe/Budapest:20261105T180000",
    "DTEND;TZID=Europe/Budapest:20261105T200000",
    "RRULE:FREQ=WEEKLY;COUNT=3",
    "SUMMARY:Játékest",
    "END:VEVENT",
    "BEGIN:VEVENT",
    "UID:series@test",
    "RECURRENCE-ID;TZID=Europe/Budapest:20261112T180000",
    "DTSTART;TZID=Europe/Budapest:20261113T180000",
    "DTEND;TZID=Europe/Budapest:20261113T200000",
    "SUMMARY:Játékest (pénteken)",
    "END:VEVENT",
    // A rule this reader does not expand: only DTSTART is shown.
    "BEGIN:VEVENT",
    "UID:unsupported@test",
    "DTSTART:20261201T170000Z",
    "RRULE:FREQ=MONTHLY;BYDAY=MO,TU;BYSETPOS=1",
    "SUMMARY:Bonyolult szabály",
    "END:VEVENT",
    // Spring-forward gap: 02:30 does not exist on 2026-03-29 and becomes 03:30 CEST.
    "BEGIN:VEVENT",
    "UID:gap@test",
    "DTSTART;TZID=Europe/Budapest:20260329T023000",
    "SUMMARY:Hajnali észlelés",
    "END:VEVENT",
    "END:VCALENDAR",
];

fn fixture() -> Vec<u8> {
    let mut feed = FIXTURE.join("\r\n");
    feed.push_str("\r\n");
    feed.into_bytes()
}

fn budapest() -> TimeZone {
    TimeZone::get("Europe/Budapest").unwrap()
}

fn ts(s: &str) -> Timestamp {
    s.parse().unwrap()
}

fn events() -> Vec<VEvent> {
    let parsed = parse(&fixture(), &budapest());
    assert_eq!(parsed.unknown_tzids, 0);
    parsed.events
}

/// Budapest start times ("2026-03-03 18:00 +60") of every instance of `uid` in a wide window.
fn starts(uid: &str) -> Vec<String> {
    let zone = budapest();
    let events = events();
    let mut out: Vec<(Timestamp, String)> = expand(&events, ts("2025-01-01T00:00Z"), ts("2028-01-01T00:00Z"), &zone)
        .into_iter()
        .filter(|o| o.event.uid == uid)
        .map(|o| {
            let l = local(o.start, &zone);
            (o.start, format!("{} {}:{:02} {:+}", l.date.iso(), l.hour, l.minute, l.offset_minutes))
        })
        .collect();
    out.sort();
    out.into_iter().map(|(_, s)| s).collect()
}

fn event<'a>(events: &'a [VEvent], uid: &str) -> &'a VEvent {
    events.iter().find(|e| e.uid == uid && e.recurrence_id.is_none()).unwrap()
}

#[test]
fn unfolds_lines() {
    assert_eq!(unfold(b"A:x\r\n y\r\n\tz\r\nB:1\r\n"), "A:xyz\r\nB:1\r\n");
    assert_eq!(unfold(b"A:x\n y\nB:1"), "A:xy\nB:1");
    // A fold inside the two bytes of "é" (C3 A9).
    assert_eq!(unfold(b"A:caf\xC3\r\n \xA9!"), "A:café!");
    assert_eq!(unfold(b"\xEF\xBB\xBFA:1"), "A:1");
}

#[test]
fn unescapes_text() {
    assert_eq!(unescape(r"a\, b\; c\nd\Ne\\f"), "a, b; c\nd\ne\\f");
    assert_eq!(unescape(r"trailing\"), "trailing");
}

#[test]
fn reads_only_the_properties_it_shows() {
    let events = events();
    // VTIMEZONE rules are not events; the cancelled event is still parsed (skipped later).
    assert_eq!(events.len(), 14);
    let utc = event(&events, "utc@test");
    assert_eq!(utc.summary, "Holdfigyelés a rakparton, távcsövekkel");
    assert_eq!(utc.location, "Műegyetem rakpart; a K épület előtt");
    // The VALARM's DESCRIPTION did not replace the event's.
    assert!(utc.description.starts_with("Hozz meleg ruhát!\nRészletek: https://kozmosz.bme.hu/programjaink."));
    assert!(utc.description.ends_with("\\o/"));
    // Attendee and organizer data are not stored at all.
    let debug = format!("{events:?}");
    assert!(!debug.contains("Kiss Péter"));
    assert!(!debug.contains("mailto"));
}

#[test]
fn three_dtstart_forms() {
    let events = events();
    let zone = budapest();
    let utc = event(&events, "utc@test");
    assert!(matches!(utc.start, Moment::DateTime(_, ref tz) if *tz == TimeZone::UTC));
    let tzid = event(&events, "tzid@test");
    assert!(matches!(tzid.start, Moment::DateTime(dt, _) if dt == civil::date(2026, 10, 15).at(18, 0, 0, 0)));
    let all_day = event(&events, "allday@test");
    assert!(matches!(all_day.start, Moment::Date(d) if d == civil::date(2026, 10, 24)));

    let window = expand(&events, ts("2026-10-01T00:00Z"), ts("2026-11-01T00:00Z"), &zone);
    let find = |uid: &str| window.iter().find(|o| o.event.uid == uid).unwrap();
    // 16:00Z in October (CEST) is 18:00 in Budapest.
    assert_eq!(find("utc@test").start, ts("2026-10-08T16:00Z"));
    assert_eq!(find("utc@test").end, ts("2026-10-08T18:00Z"));
    assert_eq!(find("tzid@test").start, ts("2026-10-15T16:00Z"));
    let all_day = find("allday@test");
    assert_eq!(all_day.days, Some((civil::date(2026, 10, 24), civil::date(2026, 10, 25))));
    // Midnight to midnight in Budapest; the night of the 25th has 25 hours (DST ends).
    assert_eq!(all_day.start, ts("2026-10-23T22:00Z"));
    assert_eq!(all_day.end, ts("2026-10-25T23:00Z"));
}

#[test]
fn skips_cancelled_events() {
    assert!(starts("cancelled@test").is_empty());
}

#[test]
fn weekly_with_until_and_exdate_across_spring_dst() {
    assert_eq!(
        starts("weekly-until@test"),
        [
            "2026-03-03 18:00 +60",
            "2026-03-10 18:00 +60",
            // 03-17 is an EXDATE
            "2026-03-24 18:00 +60",
            // DST starts on 2026-03-29: same wall-clock time, new offset
            "2026-03-31 18:00 +120",
            "2026-04-07 18:00 +120",
            "2026-04-14 18:00 +120",
            // UNTIL is inclusive: 16:00Z is exactly 18:00 CEST
            "2026-04-21 18:00 +120",
        ]
    );
}

#[test]
fn weekly_with_interval_byday_and_count_across_autumn_dst() {
    assert_eq!(
        starts("weekly-count@test"),
        ["2026-10-19 19:00 +120", "2026-10-22 19:00 +120", "2026-11-02 19:00 +60", "2026-11-05 19:00 +60"]
    );
}

#[test]
fn utc_series_moves_in_local_time() {
    assert_eq!(starts("weekly-utc@test"), ["2026-10-21 19:00 +120", "2026-10-28 18:00 +60"]);
}

#[test]
fn monthly_rules() {
    assert_eq!(
        starts("monthly-byday@test"),
        ["2026-10-13 18:00 +120", "2026-11-10 18:00 +60", "2026-12-08 18:00 +60"]
    );
    assert_eq!(
        starts("monthly-31@test"),
        ["2026-01-31 0:00 +60", "2026-03-31 0:00 +120", "2026-05-31 0:00 +120", "2026-07-31 0:00 +120"]
    );
}

#[test]
fn daily_with_interval_and_until() {
    assert_eq!(
        starts("daily@test"),
        ["2026-11-01 10:00 +60", "2026-11-04 10:00 +60", "2026-11-07 10:00 +60", "2026-11-10 10:00 +60"]
    );
}

#[test]
fn recurrence_id_moves_one_instance() {
    assert_eq!(
        starts("series@test"),
        ["2026-11-05 18:00 +60", "2026-11-13 18:00 +60", "2026-11-19 18:00 +60"]
    );
    let events = events();
    let window = expand(&events, ts("2026-11-13T00:00Z"), ts("2026-11-14T00:00Z"), &budapest());
    assert_eq!(window.len(), 1);
    assert_eq!(window[0].event.summary, "Játékest (pénteken)");
}

#[test]
fn unsupported_rule_shows_first_instance() {
    assert_eq!(starts("unsupported@test"), ["2026-12-01 18:00 +60"]);
    assert_eq!(unsupported_rules(&events()), 1);
}

#[test]
fn dst_boundaries() {
    let zone = budapest();
    let at = |s: &str| {
        let l = local(ts(s), &zone);
        format!("{} {}:{:02} {:+}", l.date.iso(), l.hour, l.minute, l.offset_minutes)
    };
    // Spring: last Sunday of March, 01:00 UTC.
    assert_eq!(at("2026-03-29T00:59Z"), "2026-03-29 1:59 +60");
    assert_eq!(at("2026-03-29T01:00Z"), "2026-03-29 3:00 +120");
    // Autumn: last Sunday of October, 01:00 UTC; 02:00-03:00 happens twice.
    assert_eq!(at("2026-10-25T00:59Z"), "2026-10-25 2:59 +120");
    assert_eq!(at("2026-10-25T01:00Z"), "2026-10-25 2:00 +60");
    // Another year, to make sure it is the rule and not a fixed date.
    assert_eq!(at("2027-03-28T00:59Z"), "2027-03-28 1:59 +60");
    assert_eq!(at("2027-03-28T01:00Z"), "2027-03-28 3:00 +120");
    assert_eq!(at("2027-10-31T00:59Z"), "2027-10-31 2:59 +120");
    assert_eq!(at("2027-10-31T01:00Z"), "2027-10-31 2:00 +60");
    // A wall-clock time inside the spring gap moves forward.
    assert_eq!(starts("gap@test"), ["2026-03-29 3:30 +120"]);
}

#[test]
fn builds_upcoming_and_past() {
    let zone = budapest();
    let events = events();
    // 19:00 Budapest time on 8 October: the 18:00-20:00 event is still running.
    let calendar = build(&events, ts("2026-10-08T17:00Z"), &zone);
    assert!(calendar.loaded);
    let titles = |list: &[Event]| list.iter().map(|e| e.title.clone()).collect::<Vec<_>>();
    let upcoming = titles(&calendar.upcoming);
    assert_eq!(upcoming.len(), 12);
    assert_eq!(
        upcoming[..6],
        [
            "Holdfigyelés a rakparton, távcsövekkel",
            "Havi gyűlés",
            "Előadás: a MASAT-1 története",
            "KSP tanfolyam",
            "ISS SSTV vétel",
            "KSP tanfolyam",
        ]
    );
    assert!(!upcoming.iter().any(|t| t == "Elmarad"));
    // Most recent first, at most six.
    let past = titles(&calendar.past);
    assert_eq!(past.len(), 6);
    assert_eq!(past[0], "Hónap vége"); // 2026-07-31
    assert_eq!(past[1], "Hónap vége"); // 2026-05-31
    assert_eq!(past[2], "Asztro tanfolyam"); // 2026-04-21

    let first = &calendar.upcoming[0];
    assert_eq!(first.location.as_deref(), Some("Műegyetem rakpart; a K épület előtt"));
    assert_eq!(first.date_label(), "2026. október 8., csütörtök");
    assert_eq!(first.time_label().as_deref(), Some("18:00-20:00"));
    assert_eq!(first.datetime_attr(), "2026-10-08T18:00+02:00");
    let description = first.description.as_deref().unwrap();
    assert_eq!(
        description,
        "Hozz meleg ruhát! Részletek: https://kozmosz.bme.hu/programjaink. Kérdés: vagy bme.kozmosz@gmail.com \\o/"
    );

    // No e-mail address but the club's own reaches the page.
    let shown = format!("{calendar:?}").replace(crate::site::EMAIL, "");
    assert!(!shown.contains('@'), "{shown}");

    let all_day = calendar.upcoming.iter().find(|e| e.all_day).unwrap();
    assert_eq!(all_day.date_label(), "2026. október 24., szombat - október 25., vasárnap");
    assert_eq!(all_day.time_label(), None);
    assert_eq!(all_day.datetime_attr(), "2026-10-24");
    assert_eq!(all_day.location.as_deref(), Some("\"Site: Iroda\" 1111 Budapest"));

    let lecture = calendar.upcoming.iter().find(|e| e.title.starts_with("Előadás")).unwrap();
    assert_eq!(
        lecture.description.as_deref(),
        Some("Előadónk a projekt vezetője. Belépés regisztrációval & ingyenes.")
    );

    // A month later the October events are past, and nothing is lost when the feed is gone.
    let later = build(&events, ts("2026-11-30T12:00Z"), &zone);
    assert_eq!(later.past[0].title, "Játékest");
    assert_eq!(later.upcoming[0].title, "Bonyolult szabály");
}

#[test]
fn plain_text() {
    assert_eq!(text::plain("  több   sor\n\nszöveg ", 280).as_deref(), Some("több sor szöveg"));
    assert_eq!(text::plain("<p></p>", 280), None);
    assert_eq!(text::plain("1 < 2 &amp; 3 &gt; 2 &#337; &#x171; &bogus; &", 280).as_deref(), Some("1 < 2 & 3 > 2 ő ű &bogus; &"));
    assert_eq!(text::plain("<b>Kozmosz</b>-os <i>est</i>", 280).as_deref(), Some("Kozmosz-os est"));
    assert_eq!(text::plain("Írj: mailto:a.b+c@sub.example.hu!", 280).as_deref(), Some("Írj: !"));
    assert_eq!(text::plain("Tech+Art @ Kozmosz", 280).as_deref(), Some("Tech+Art @ Kozmosz"));
    assert_eq!(
        text::plain("Gyere el!\n\n-::~:~::~:~:~:~::~:~::-\nJoin with Google Meet: https://meet.google.com/abc", 280).as_deref(),
        Some("Gyere el!")
    );
    let meet = "Csatlakozás a Google Meet szolgáltatással: https://meet.google.com/abc-defg-hij\nVagy hívja a \
                következő számot: (HU) +36 1 234 5678 PIN-kód: 123456789#\n\nTovábbi információ a Meetről: \
                https://support.google.com/a/users/answer/9282720";
    assert_eq!(text::plain(meet, 280), None);
    assert_eq!(text::plain(&format!("Hozz távcsövet!\n\n{meet}"), 280).as_deref(), Some("Hozz távcsövet!"));
    let long = "csillag ".repeat(50);
    let short = text::plain(&long, 280).unwrap();
    assert!(short.chars().count() <= 280);
    assert!(short.ends_with("csillag…"), "{short}");
    let word = "a".repeat(300);
    assert_eq!(text::plain(&word, 280).unwrap().chars().count(), 280);
}

#[test]
fn hungarian_dates() {
    let date = Date { year: 2026, month: 10, day: 8 };
    assert_eq!(date.long(), "2026. október 8., csütörtök");
    assert_eq!(Date { year: 2027, month: 1, day: 1 }.long(), "2027. január 1., péntek");
    assert_eq!(Date { year: 2024, month: 2, day: 29 }.long(), "2024. február 29., csütörtök");
    assert_eq!(date.iso(), "2026-10-08");

    // The weekday arithmetic agrees with jiff for every day of a few decades.
    let mut day = civil::date(1990, 1, 1);
    while day < civil::date(2060, 1, 1) {
        let ours = Date { year: day.year(), month: day.month() as u8, day: day.day() as u8 };
        assert_eq!(ours.weekday(), day.weekday().to_monday_zero_offset() as usize, "{day}");
        day = day.tomorrow().unwrap();
    }
}

fn at(date: (i16, u8, u8), hour: u8, minute: u8, offset_minutes: i16) -> LocalDateTime {
    LocalDateTime { date: Date { year: date.0, month: date.1, day: date.2 }, hour, minute, offset_minutes }
}

fn timed(start: LocalDateTime, end: LocalDateTime) -> Event {
    Event { title: "x".into(), location: None, description: None, start, end, all_day: false }
}

#[test]
fn event_labels() {
    let evening = timed(at((2026, 10, 8), 18, 0, 120), at((2026, 10, 8), 20, 0, 120));
    assert_eq!(evening.time_label().as_deref(), Some("18:00-20:00"));
    assert_eq!(evening.datetime_attr(), "2026-10-08T18:00+02:00");

    let no_end = timed(at((2026, 12, 1), 9, 5, 60), at((2026, 12, 1), 9, 5, 60));
    assert_eq!(no_end.time_label().as_deref(), Some("9:05"));
    assert_eq!(no_end.datetime_attr(), "2026-12-01T09:05+01:00");

    let night = timed(at((2026, 10, 9), 20, 0, 120), at((2026, 10, 10), 2, 0, 120));
    assert_eq!(night.date_label(), "2026. október 9., péntek");
    assert_eq!(night.time_label().as_deref(), Some("20:00-2:00"));

    let camp = timed(at((2026, 12, 30), 18, 0, 60), at((2027, 1, 2), 14, 0, 60));
    assert_eq!(camp.date_label(), "2026. december 30., szerda 18:00 - 2027. január 2., szombat 14:00");
    assert_eq!(camp.time_label(), None);

    let one_day = Event { all_day: true, ..timed(at((2026, 10, 24), 0, 0, 120), at((2026, 10, 24), 0, 0, 120)) };
    assert_eq!(one_day.date_label(), "2026. október 24., szombat");
    assert_eq!(one_day.time_label(), None);

    // No en or em dash anywhere.
    for label in [evening.time_label().unwrap(), night.time_label().unwrap(), camp.date_label()] {
        assert!(!label.contains(['\u{2013}', '\u{2014}']), "{label}");
    }
}

#[test]
fn links_in_descriptions() {
    assert_eq!(
        linkify("Részletek: https://example.com/a?b=1. Köszi"),
        [Segment::Text("Részletek: "), Segment::Link("https://example.com/a?b=1"), Segment::Text(". Köszi")]
    );
    assert_eq!(
        linkify("(lásd https://hu.wikipedia.org/wiki/Hold_(égitest))"),
        [
            Segment::Text("(lásd "),
            Segment::Link("https://hu.wikipedia.org/wiki/Hold_(égitest)"),
            Segment::Text(")"),
        ]
    );
    assert_eq!(linkify("http://a.hu"), [Segment::Link("http://a.hu")]);
    // Cut short by the description limit, or no host: left as text.
    assert_eq!(linkify("lásd https://example.com/hossz…"), [Segment::Text("lásd https://example.com/hossz…")]);
    assert_eq!(linkify("csak https:// így"), [Segment::Text("csak https:// így")]);
    assert_eq!(linkify(""), []);
}

#[test]
fn e_mail_after_accented_text() {
    // The 7 bytes before the address start inside "ő": looking for "mailto:" must not slice there.
    assert_eq!(text::plain("Érdeklődés: kiss.peter@example.com", 280).as_deref(), Some("Érdeklődés:"));
    assert_eq!(text::plain("Szervező 🚀 anna@example.com", 280).as_deref(), Some("Szervező 🚀"));
    assert_eq!(text::plain("Írj: MAILTO:anna@example.com!", 280).as_deref(), Some("Írj: !"));
}

/// The instances of a one-event feed in 2026-2027.
fn instances_of(lines: &[&str]) -> usize {
    let zone = budapest();
    let feed = format!("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:x\r\n{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n", lines.join("\r\n"));
    let events = parse(feed.as_bytes(), &zone).events;
    assert_eq!(events.len(), 1);
    expand(&events, ts("2026-01-01T00:00Z"), ts("2028-01-01T00:00Z"), &zone).len()
}

#[test]
fn huge_numbers_do_not_panic() {
    // An interval far beyond the calendar's range leaves only DTSTART.
    for rule in [
        "RRULE:FREQ=DAILY;INTERVAL=10000000",
        "RRULE:FREQ=WEEKLY;INTERVAL=2000000",
        "RRULE:FREQ=DAILY;INTERVAL=9223372036854775807",
        "RRULE:FREQ=WEEKLY;INTERVAL=9223372036854775807",
        "RRULE:FREQ=MONTHLY;INTERVAL=9223372036854775807",
        "RRULE:FREQ=YEARLY;INTERVAL=9223372036854775807",
    ] {
        assert_eq!(instances_of(&["DTSTART:20261008T160000Z", rule]), 1, "{rule}");
    }
    // An all-day event longer than any date range is dropped, not a crash.
    assert_eq!(instances_of(&["DTSTART;VALUE=DATE:20261008", "DURATION:P1043497W7304484D"]), 0);
    assert_eq!(instances_of(&["DTSTART;VALUE=DATE:20261008", "DURATION:P2W"]), 1);
}

#[tokio::test]
async fn feed_work_survives_a_panic() {
    use super::feed::off_thread;
    assert_eq!(off_thread("adding", || 2 + 2).await, Some(4));
    // A panic while parsing or expanding is caught (and logged), so neither the server's startup
    // nor the refresh loop dies with it.
    assert_eq!(off_thread("parsing", || -> u8 { panic!("malformed feed") }).await, None);
}
