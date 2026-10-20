//! Attendance sheets for the club's events (owner's request 2026-10-20). Every calendar event has
//! one: an admin opens it on the dashboard and shows its QR code, and whoever scans it lands on
//! /jelen/<token>, types their name and is on the list. The names are matched against the
//! membership sheet (a Google Sheet read with the Drive service account), so the admin sees which
//! members came.
//!
//! - The QR link's token is a tag of the event ([`admin::tag`]): unguessable, nothing to store,
//!   and void once admin.key is deleted.
//! - A sheet takes names from an hour before the event until three hours after it.
//! - Each event's names are a small JSON file in the state directory (jelenlet/<event>.json), so
//!   a sheet stays listed after its event has left the calendar.

use std::{
    cmp::Reverse,
    collections::HashSet,
    path::{Path as FsPath, PathBuf},
    sync::{Arc, Mutex, PoisonError, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aws_lc_rs::constant_time;
use axum::{
    extract::{Form, Path},
    http::{header, request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jiff::{tz::TimeZone, Timestamp};
use leptos::{
    logging::{error, log},
    prelude::*,
};
use leptos_meta::{Meta, Title};
use qrcode::{render::svg, QrCode};
use serde::{Deserialize, Serialize};

use crate::{
    admin,
    calendar::{self, Event, LocalDateTime},
    drive::{slugify, write_atomic, ServiceAccount, ENV_CREDENTIALS},
    icons,
    media::Icon,
    pages::{AdminLayout, NotFound, PageHeader, Section},
    site,
};

/// The membership sheet's ID (the long part of its URL). The sheet is shared (Viewer) with the
/// service account of KOZMOSZ_DRIVE_CREDENTIALS. None: names are not matched to members.
const MEMBERS_SHEET: Option<&str> = None;
const ZONE: &str = "Europe/Budapest";
const OPENS_BEFORE: i64 = 60 * 60;
const CLOSES_AFTER: i64 = 3 * 60 * 60;
/// More names than any event has: a sheet whose link leaked cannot grow without end.
const MAX_NAMES: usize = 500;
const REFRESH_EVERY: Duration = Duration::from_secs(10 * 60);
/// The cookie that keeps a signed name's "Kitöltve" ticket on that phone.
const TICKET: &str = "kozmosz_jelen";

/// The "Kitöltve" ticket's picture (owner's request 2026-10-20, like a transit app's live
/// ticket): a planet with a moon in orbit, a pulse and twinkling stars, in the site's yellow and
/// white. It keeps moving, so the real page can be told from a screenshot (style/main.css,
/// "Ticket"); the moon moves by SMIL, everything else by CSS.
const LIFTOFF: &str = r#"<svg viewBox="0 0 240 200" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false"><g transform="translate(36 44)"><path class="lift-star" d="M0-9C1-3 3-1 9 0C3 1 1 3 0 9C-1 3-3 1-9 0C-3-1-1-3 0-9Z"/></g><g transform="translate(206 38) scale(.7)"><path class="lift-star" style="--d:.7s" d="M0-9C1-3 3-1 9 0C3 1 1 3 0 9C-1 3-3 1-9 0C-3-1-1-3 0-9Z"/></g><g transform="translate(22 156) scale(.6)"><path class="lift-star" style="--d:1.4s" d="M0-9C1-3 3-1 9 0C3 1 1 3 0 9C-1 3-3 1-9 0C-3-1-1-3 0-9Z"/></g><g transform="translate(214 160) scale(.85)"><path class="lift-star" style="--d:.35s" d="M0-9C1-3 3-1 9 0C3 1 1 3 0 9C-1 3-3 1-9 0C-3-1-1-3 0-9Z"/></g><g transform="translate(124 14) scale(.5)"><path class="lift-star" style="--d:1.05s" d="M0-9C1-3 3-1 9 0C3 1 1 3 0 9C-1 3-3 1-9 0C-3-1-1-3 0-9Z"/></g><circle class="lift-pulse" cx="120" cy="100" r="26"/><circle class="lift-pulse" style="--d:1.2s" cx="120" cy="100" r="26"/><g transform="rotate(-16 120 100)"><ellipse class="lift-orbit" cx="120" cy="100" rx="90" ry="34"/><g class="lift-moon"><circle r="6"><animateMotion dur="6s" repeatCount="indefinite" path="M210,100A90,34 0 1 1 30,100A90,34 0 1 1 210,100"/></circle></g><circle class="lift-moon-still" cx="210" cy="100" r="6"/></g><circle class="lift-planet" cx="120" cy="100" r="26"/><path class="lift-check" d="M108 101l8 8 16-18"/></svg>"#;

#[derive(Serialize, Deserialize)]
struct Sheet {
    title: String,
    /// When the event was, as the dashboard shows it.
    when: String,
    /// Start, Unix seconds (for sorting).
    start: i64,
    names: Vec<Signed>,
}

#[derive(Serialize, Deserialize)]
struct Signed {
    name: String,
    /// Unix seconds.
    at: i64,
}

#[derive(Default)]
struct Members {
    names: Vec<String>,
    status: String,
}

#[derive(Deserialize)]
struct Values {
    #[serde(default)]
    values: Vec<Vec<String>>,
}

#[derive(Deserialize)]
pub struct Signature {
    nev: String,
}

struct Row {
    key: String,
    title: String,
    when: String,
    open: bool,
    count: usize,
}

static MEMBERS: RwLock<Option<Arc<Members>>> = RwLock::new(None);
/// One writer at a time: a sheet is read, extended and written back.
static WRITING: Mutex<()> = Mutex::new(());

/// The event's sheet name: start and title, e.g. "2026-10-21-1800-csillagaszati-est".
fn key(event: &Event) -> String {
    let start = event.start;
    format!("{}-{:02}{:02}-{}", start.date.iso(), start.hour, start.minute, slugify(&event.title))
}

/// Keys come from URLs: only what [`key`] makes, so no path can be smuggled in.
fn valid_key(key: &str) -> bool {
    (1..=160).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn token(key: &str) -> String {
    admin::tag(&format!("jelenlet:{key}"))
}

/// The cookie value of a ticket: the name and the time it was signed, tagged for the event, so
/// it cannot be made up for another name, time or event.
fn ticket_value(key: &str, name: &str, at: i64) -> String {
    let tag = admin::tag(&format!("jelen-ok:{key}:{at}:{name}"));
    format!("{}.{at}.{tag}", URL_SAFE_NO_PAD.encode(name))
}

/// The name and time on this phone's ticket for the event, if it checks out.
fn ticket(headers: &HeaderMap, key: &str) -> Option<(String, i64)> {
    let mut parts = admin::cookie(headers, TICKET)?.splitn(3, '.');
    let name = String::from_utf8(URL_SAFE_NO_PAD.decode(parts.next()?).ok()?).ok()?;
    let at: i64 = parts.next()?.parse().ok()?;
    let tag = parts.next()?;
    let expected = ticket_value(key, &name, at);
    let ok = constant_time::verify_slices_are_equal(expected.rsplit('.').next()?.as_bytes(), tag.as_bytes()).is_ok();
    ok.then_some((name, at))
}

/// The calendar event whose sheet the token `given` opens.
fn find(given: &str) -> Option<Event> {
    let calendar = calendar::current();
    calendar
        .upcoming
        .iter()
        .chain(&calendar.past)
        .find(|e| constant_time::verify_slices_are_equal(token(&key(e)).as_bytes(), given.as_bytes()).is_ok())
        .cloned()
}

fn when(event: &Event) -> String {
    match event.time_label() {
        Some(time) => format!("{} {time}", event.date_label()),
        None => event.date_label(),
    }
}

fn unix(time: LocalDateTime) -> i64 {
    (time.date.days_since_epoch() * 24 + i64::from(time.hour)) * 3600 + i64::from(time.minute) * 60
        - i64::from(time.offset_minutes) * 60
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// "18:05", Budapest time.
fn clock(at: i64) -> String {
    let zone = TimeZone::get(ZONE).unwrap_or(TimeZone::UTC);
    Timestamp::from_second(at).map(|t| t.to_zoned(zone).strftime("%H:%M").to_string()).unwrap_or_default()
}

/// Names are taken from an hour before the start until three hours after the end (all-day
/// events: after their last day).
fn is_open(event: &Event, now: i64) -> bool {
    let end = unix(event.end) + if event.all_day { 24 * 3600 } else { 0 };
    (unix(event.start) - OPENS_BEFORE..=end + CLOSES_AFTER).contains(&now)
}

/// A typed name, spaces tidied, if it looks like one: 2 to 60 characters, letters with spaces,
/// dots, hyphens and apostrophes.
fn clean_name(raw: &str) -> Option<String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let letters = name.chars().filter(|c| c.is_alphabetic()).count();
    let ok = (2..=60).contains(&name.chars().count())
        && letters >= 2
        && name.chars().all(|c| c.is_alphabetic() || matches!(c, ' ' | '.' | '-' | '\''));
    ok.then_some(name)
}

/// Lowercase ASCII words without titles: "Dr. Kovács-Tóth Anna" gives kovacs, toth, anna.
fn words(name: &str) -> Vec<String> {
    // Dots to spaces first: slugify would take "A.B" for a file extension.
    slugify(&name.replace('.', " "))
        .split('-')
        .filter(|w| !w.is_empty() && !matches!(*w, "dr" | "id" | "ifj" | "prof"))
        .map(str::to_owned)
        .collect()
}

/// The member a typed name stands for: each of its words (at least two) begins a different word
/// of exactly one member's name, in any order ("Kovács A." and "anna kovacs" both find "Kovács
/// Anna Petra"). None when no member or several fit.
fn member<'a>(name: &str, members: &'a [String]) -> Option<&'a str> {
    let mut typed = words(name);
    if typed.len() < 2 {
        return None;
    }
    // Longer words first, so "a" does not take the word that "anna" needs.
    typed.sort_by_key(|w| Reverse(w.len()));
    let mut fits = members.iter().filter(|m| covers(&words(m), &typed));
    let first = fits.next()?;
    fits.next().is_none().then_some(first.as_str())
}

fn covers(member: &[String], typed: &[String]) -> bool {
    let mut free = vec![true; member.len()];
    typed.iter().all(|t| match (0..member.len()).find(|&i| free[i] && member[i].starts_with(t.as_str())) {
        Some(i) => {
            free[i] = false;
            true
        }
        None => false,
    })
}

fn dir() -> PathBuf {
    std::env::var_os("STATE_DIRECTORY").map_or_else(|| PathBuf::from("jelenlet"), |d| PathBuf::from(d).join("jelenlet"))
}

fn load(dir: &FsPath, key: &str) -> Option<Sheet> {
    serde_json::from_slice(&std::fs::read(dir.join(format!("{key}.json"))).ok()?).ok()
}

/// Every sheet on disk.
fn stored(dir: &FsPath) -> Vec<(String, Sheet)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            let key = name.strip_suffix(".json")?.to_owned();
            let sheet = load(dir, &key)?;
            Some((key, sheet))
        })
        .collect()
}

/// Puts `name` on the event's sheet; a name already there (ignoring case, accents and spacing)
/// is not added again.
fn record(dir: &FsPath, event: &Event, name: &str, now: i64) -> Result<(), String> {
    let _one = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
    let key = key(event);
    let mut sheet = load(dir, &key).unwrap_or_else(|| Sheet {
        title: event.title.clone(),
        when: when(event),
        start: unix(event.start),
        names: Vec::new(),
    });
    let folded = words(name);
    if sheet.names.iter().any(|s| words(&s.name) == folded) {
        return Ok(());
    }
    if sheet.names.len() >= MAX_NAMES {
        return Err(format!("{key}: the sheet is full"));
    }
    sheet.names.push(Signed { name: name.to_owned(), at: now });
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let json = serde_json::to_vec(&sheet).map_err(|e| e.to_string())?;
    write_atomic(&dir.join(format!("{key}.json")), &json)
}

fn members() -> Arc<Members> {
    MEMBERS.read().unwrap_or_else(PoisonError::into_inner).clone().unwrap_or_else(|| {
        Arc::new(Members { names: Vec::new(), status: "A tagsági lista nincs beállítva.".into() })
    })
}

fn publish(members: Members) {
    *MEMBERS.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(members));
}

/// The names in the membership sheet: the column headed "Név", "Teljes név" or "Name" (else the
/// first column), below the header row.
fn names(rows: &[Vec<String>]) -> Vec<String> {
    let column = rows
        .first()
        .and_then(|header| {
            header.iter().position(|h| matches!(slugify(h).as_str(), "nev" | "teljes-nev" | "name" | "full-name"))
        })
        .unwrap_or(0);
    rows.iter()
        .skip(1)
        .filter_map(|row| row.get(column))
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

async fn read_members(client: &reqwest::Client, account: &mut ServiceAccount, sheet: &str) -> Result<Vec<String>, String> {
    let token = account.token(client).await?;
    let values: Values = client
        .get(format!("https://sheets.googleapis.com/v4/spreadsheets/{sheet}/values/A:Z"))
        .bearer_auth(token)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.without_url().to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(names(&values.values))
}

/// Reads the membership sheet at startup and every 10 minutes, once it is set up
/// ([`MEMBERS_SHEET`] and the service account). A failed read keeps the last names.
pub fn spawn_members_refresh() {
    let Some(sheet) = MEMBERS_SHEET else { return };
    let account = std::env::var(ENV_CREDENTIALS)
        .map_err(|_| format!("{ENV_CREDENTIALS} is not set"))
        .and_then(|path| std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}")))
        .and_then(|json| ServiceAccount::from_json(&json));
    let mut account = match account {
        Ok(account) => account,
        Err(err) => {
            error!("attendance: cannot read the membership sheet: {err}");
            publish(Members { names: Vec::new(), status: "A tagsági listához nincs beállítva a service account.".into() });
            return;
        }
    };
    let client = reqwest::Client::builder()
        .user_agent(format!("kozmosz-site/{} (+{})", env!("CARGO_PKG_VERSION"), site::ORIGIN))
        .https_only(true)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("HTTP client");
    tokio::spawn(async move {
        loop {
            match read_members(&client, &mut account, sheet).await {
                Ok(names) => {
                    log!("attendance: {} members in the membership sheet", names.len());
                    let status = format!("{} név a tagsági listában (frissítve {}).", names.len(), clock(now()));
                    publish(Members { names, status });
                }
                Err(err) => {
                    error!("attendance: cannot read the membership sheet: {err}");
                    let names = members().names.clone();
                    publish(Members { names, status: "A tagsági lista most nem olvasható, a legutóbbi névsorral egyeztetek.".into() });
                }
            }
            tokio::time::sleep(REFRESH_EVERY).await;
        }
    });
}

/// POST /jelen/{token}: puts the name on the sheet, gives this phone its ticket (a cookie for
/// the page), then back to the page, which shows the ticket.
pub async fn sign(Path(given): Path<String>, Form(form): Form<Signature>) -> Response {
    let Some(event) = find(&given) else { return StatusCode::NOT_FOUND.into_response() };
    // `given` is a token we made: safe in a header.
    let page = format!("/jelen/{given}");
    let now = now();
    if !is_open(&event, now) {
        return Redirect::to(&page).into_response();
    }
    let Some(name) = clean_name(&form.nev) else { return Redirect::to(&format!("{page}?hiba=nev")).into_response() };
    match record(&dir(), &event, &name, now) {
        Ok(()) => {
            let mut response = Redirect::to(&page).into_response();
            let cookie = format!(
                "{TICKET}={}; Path={page}; Max-Age=43200; HttpOnly; Secure; SameSite=Lax",
                ticket_value(&key(&event), &name, now),
            );
            // Base64, digits and the token: always a valid header.
            response.headers_mut().append(header::SET_COOKIE, HeaderValue::from_str(&cookie).expect("ASCII cookie"));
            response
        }
        Err(err) => {
            error!("attendance: {err}");
            Redirect::to(&format!("{page}?hiba=mentes")).into_response()
        }
    }
}

/// GET /admin/jelenlet/{key}/csv (behind admin::guard): the names and their members, the way
/// Hungarian Excel opens a CSV (semicolons, byte-order mark).
pub async fn csv(Path(key): Path<String>) -> Response {
    if !valid_key(&key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // No file yet: nobody has signed, an empty table.
    let names = load(&dir(), &key).map(|s| s.names).unwrap_or_default();
    let members = members();
    let field = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut out = String::from("\u{feff}Név;Tag;Időpont\r\n");
    for s in &names {
        let matched = member(&s.name, &members.names).unwrap_or_default();
        out.push_str(&format!("{};{};{}\r\n", field(&s.name), field(matched), clock(s.at)));
    }
    let headers = [
        (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()),
        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"jelenlet-{key}.csv\"")),
    ];
    (headers, out).into_response()
}

fn row(dir: &FsPath, event: &Event, now: i64) -> Row {
    let key = key(event);
    let count = load(dir, &key).map_or(0, |s| s.names.len());
    Row { title: event.title.clone(), when: when(event), open: is_open(event, now), count, key }
}

/// /admin: the sheets of the coming events, then of the earlier ones (the calendar's recent past
/// and every older sheet on disk).
pub fn events_page() -> AnyView {
    let (calendar, dir, now) = (calendar::current(), dir(), now());
    let coming: Vec<Row> = calendar.upcoming.iter().take(10).map(|e| row(&dir, e, now)).collect();
    let mut earlier: Vec<(i64, Row)> = calendar.past.iter().map(|e| (unix(e.start), row(&dir, e, now))).collect();
    let listed: HashSet<String> = coming.iter().chain(earlier.iter().map(|(_, r)| r)).map(|r| r.key.clone()).collect();
    for (key, sheet) in stored(&dir) {
        if !listed.contains(&key) {
            let row = Row { title: sheet.title, when: sheet.when, open: false, count: sheet.names.len(), key };
            earlier.push((sheet.start, row));
        }
    }
    earlier.sort_by_key(|(start, _)| Reverse(*start));
    let earlier = earlier.into_iter().map(|(_, row)| row).collect();
    view! {
        <AdminLayout title="Jelenléti ívek" section=Section::Sheets>
            <p class="admin-status">{members().status.clone()}</p>
            <h2 class="admin-group">"Következő események"</h2>
            {event_list(coming)}
            <h2 class="admin-group">"Korábbi események"</h2>
            {event_list(earlier)}
        </AdminLayout>
    }
    .into_any()
}

fn event_list(rows: Vec<Row>) -> AnyView {
    if rows.is_empty() {
        return view! { <p class="empty-note">"Nincs ilyen esemény."</p> }.into_any();
    }
    view! {
        <ul class="admin-events" role="list">
            {rows
                .into_iter()
                .map(|r| view! {
                    <li>
                        <a href=format!("/admin/jelenlet/{}", r.key)>
                            <span class="admin-event-title">{r.title}</span>
                            <span class="admin-event-when">{r.when}</span>
                        </a>
                        {r.open.then(|| view! { <span class="admin-open">"Nyitva"</span> })}
                        <span class="admin-count mono">{r.count} <span class="visually-hidden">" név"</span></span>
                    </li>
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

/// The QR code of `link`, as inline SVG (dark on light, with the quiet zone scanners need).
fn qr_svg(link: &str) -> Option<String> {
    let svg = QrCode::new(link.as_bytes()).ok()?.render::<svg::Color<'_>>().min_dimensions(320, 320).build();
    // Without the XML declaration: the SVG goes inline into HTML.
    svg.find("<svg").map(|at| svg[at..].to_owned())
}

/// /admin/jelenlet/<key>: the event's QR code to show, and the names so far with their members.
pub fn event_page(key: &str) -> AnyView {
    let (dir, now) = (dir(), now());
    let valid = valid_key(key);
    let event = valid
        .then(|| {
            let calendar = calendar::current();
            calendar.upcoming.iter().chain(&calendar.past).find(|e| self::key(e) == key).cloned()
        })
        .flatten();
    let sheet = valid.then(|| load(&dir, key)).flatten();
    let (title, when) = match (&event, &sheet) {
        (Some(event), _) => (event.title.clone(), when(event)),
        (None, Some(sheet)) => (sheet.title.clone(), sheet.when.clone()),
        (None, None) => return view! { <NotFound/> }.into_any(),
    };
    let state = match &event {
        Some(event) if is_open(event, now) => "Nyitva: most lehet beírni a neveket.",
        Some(event) if now < unix(event.start) => "Az esemény előtt egy órával nyílik.",
        Some(_) => "Lezárva.",
        None => "Lezárva, az esemény már nincs a naptárban.",
    };
    let link = event.as_ref().map(|_| format!("{}/jelen/{}", site::ORIGIN, token(key)));
    let qr = link.as_deref().and_then(qr_svg).zip(link.clone());
    let members = members();
    let names = sheet.map(|s| s.names).unwrap_or_default();
    let listed: Vec<(String, Option<String>, String)> = names
        .iter()
        .map(|s| (s.name.clone(), member(&s.name, &members.names).map(str::to_owned), clock(s.at)))
        .collect();
    let matched = listed.iter().filter(|(_, m, _)| m.is_some()).count();
    let here = format!("/admin/jelenlet/{key}");
    view! {
        <AdminLayout title=title section=Section::Sheets>
            <p class="admin-status">{when} " · " {state}</p>
            <div class="sheet">
                {qr.map(|(svg, link)| view! {
                    <figure class="sheet-qr">
                        <div class="sheet-qr-code" inner_html=svg></div>
                        <figcaption class="mono">{link}</figcaption>
                    </figure>
                })}
                <div class="sheet-names">
                    <p class="sheet-count">
                        <strong class="mono">{listed.len()}</strong>
                        " név, ebből "
                        <strong class="mono">{matched}</strong>
                        " a tagsági listán"
                    </p>
                    {if listed.is_empty() {
                        view! { <p class="empty-note">"Még senki sem írta be a nevét."</p> }.into_any()
                    } else {
                        view! {
                            <ol class="sheet-list">
                                {listed
                                    .into_iter()
                                    .map(|(name, member, at)| view! {
                                        <li>
                                            <span class="sheet-name">{name}</span>
                                            <span class="sheet-member">
                                                {member.unwrap_or_else(|| "nem találom a tagok között".into())}
                                            </span>
                                            <span class="sheet-time mono">{at}</span>
                                        </li>
                                    })
                                    .collect_view()}
                            </ol>
                        }
                        .into_any()
                    }}
                    <p class="sheet-actions">
                        <a class="button button-line" href=here.clone()>"Frissítés"</a>
                        <a class="link-arrow" href=format!("{here}/csv")>
                            "Letöltés Excelhez"
                            <Icon svg=icons::ARROW_RIGHT/>
                        </a>
                    </p>
                </div>
            </div>
        </AdminLayout>
    }
    .into_any()
}

/// /jelen/<token>: the form behind the QR code, or this phone's ticket once its name is on the
/// sheet. `problem` after a refused name ("nev": not a name, "mentes": could not save).
pub fn check_in_page(given: &str, problem: Option<&str>) -> AnyView {
    let Some(event) = find(given) else { return view! { <NotFound/> }.into_any() };
    let open = is_open(&event, now());
    let headers = use_context::<Parts>().map(|parts| parts.headers).unwrap_or_default();
    let ticket = ticket(&headers, &key(&event));
    let problem = problem.map(|p| match p {
        "nev" => "Írd be a teljes neved: legalább 2, legfeljebb 60 betű.",
        _ => "Most nem sikerült elmenteni, próbáld újra.",
    });
    let body = if let Some((name, at)) = ticket {
        view! {
            <div class="ticket" role="status">
                <div class="liftoff" inner_html=LIFTOFF></div>
                <p class="ticket-ok">"Kitöltve"</p>
                <p class="ticket-name">{name}</p>
                <p class="ticket-meta">{event.title.clone()} " · " <span class="mono">{clock(at)}</span></p>
            </div>
        }
        .into_any()
    } else if !open {
        view! { <p class="empty-note">"Ez a jelenléti ív most nincs nyitva."</p> }.into_any()
    } else {
        view! {
            <form class="checkin" method="post">
                <label for="nev">"A teljes neved"</label>
                <input
                    id="nev"
                    name="nev"
                    type="text"
                    required
                    minlength="2"
                    maxlength="60"
                    autocomplete="name"
                    autocapitalize="words"
                    aria-describedby="nev-info"
                />
                {problem.map(|p| view! { <p class="checkin-problem" role="alert">{p}</p> })}
                <button class="button button-signal" type="submit">"Itt vagyok"</button>
                <p id="nev-info" class="checkin-info">"A neved a szervezők jelenléti ívére kerül, csak ők látják."</p>
            </form>
        }
        .into_any()
    };
    view! {
        <Title text="Jelenléti ív"/>
        <Meta name="robots" content="noindex"/>
        <PageHeader title=event.title.clone()>
            <p class="page-lead">{when(&event)}</p>
        </PageHeader>
        <section class="container page-body">{body}</section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Date;

    fn event(title: &str, (hour, end_hour): (u8, u8)) -> Event {
        let at = |hour| LocalDateTime { date: Date { year: 2026, month: 10, day: 21 }, hour, minute: 0, offset_minutes: 120 };
        Event { title: title.into(), location: None, description: None, start: at(hour), end: at(end_hour), all_day: false }
    }

    #[test]
    fn names_are_tidied_and_checked() {
        assert_eq!(clean_name("  Kovács   Anna "), Some("Kovács Anna".into()));
        assert_eq!(clean_name("O'Brien Seán-Óg"), Some("O'Brien Seán-Óg".into()));
        assert_eq!(clean_name("A"), None);
        assert_eq!(clean_name("Anna 2"), None);
        assert_eq!(clean_name("<b>Anna</b>"), None);
        assert_eq!(clean_name(&"a".repeat(61)), None);
    }

    #[test]
    fn typed_names_find_one_member() {
        let members = ["Kovács Anna Petra", "Kovács András", "Dr. Szabó József"].map(String::from);
        assert_eq!(member("anna kovacs", &members), Some("Kovács Anna Petra"));
        assert_eq!(member("Kovács Anna", &members), Some("Kovács Anna Petra"));
        assert_eq!(member("Szabó József", &members), Some("Dr. Szabó József"));
        assert_eq!(member("Kovács A.", &members), None, "two members fit");
        assert_eq!(member("Anna", &members), None, "one word is not enough");
        assert_eq!(member("Kiss Péter", &members), None);
    }

    #[test]
    fn sheets_open_an_hour_before_and_close_three_hours_after() {
        let e = event("Csillagászati est", (18, 20));
        assert!(!is_open(&e, unix(e.start) - OPENS_BEFORE - 1));
        assert!(is_open(&e, unix(e.start) - OPENS_BEFORE));
        assert!(is_open(&e, unix(e.end) + CLOSES_AFTER));
        assert!(!is_open(&e, unix(e.end) + CLOSES_AFTER + 1));
        // 18:00 in Budapest summer time is 16:00 UTC.
        assert_eq!(unix(e.start) % 86_400, 16 * 3600);
    }

    #[test]
    fn keys_are_safe_file_names() {
        assert_eq!(key(&event("Csillagászati est!", (18, 20))), "2026-10-21-1800-csillagaszati-est");
        assert!(valid_key("2026-10-21-1800-csillagaszati-est"));
        assert!(!valid_key("../admin"));
        assert!(!valid_key(""));
    }

    #[test]
    fn members_come_from_the_name_column() {
        let rows = |header: &str| vec![vec!["E-mail".into(), header.into()], vec!["a@b.hu".into(), " Kovács Anna ".into()], vec!["c@d.hu".into(), String::new()]];
        assert_eq!(names(&rows("Teljes név")), ["Kovács Anna"]);
        assert_eq!(names(&rows("Valami")), ["a@b.hu", "c@d.hu"], "no name header: the first column");
    }

    #[test]
    fn tickets_cannot_be_made_up() {
        let key = "2026-10-21-1800-teszt";
        let cookie = |value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(header::COOKIE, HeaderValue::from_str(&format!("{TICKET}={value}")).unwrap());
            headers
        };
        let value = ticket_value(key, "Kovács Anna", 1000);
        assert_eq!(ticket(&cookie(&value), key), Some(("Kovács Anna".into(), 1000)));
        assert_eq!(ticket(&cookie(&value), "2026-10-22-1800-masik"), None, "another event");
        let other = ticket_value(key, "Kiss Péter", 1000);
        let forged = format!("{}.{}", other.split('.').next().unwrap(), value.split_once('.').unwrap().1);
        assert_eq!(ticket(&cookie(&forged), key), None, "another name");
        assert_eq!(ticket(&HeaderMap::new(), key), None);
    }

    #[test]
    fn a_name_goes_on_the_sheet_once() {
        let dir = std::env::temp_dir().join(format!("kozmosz-jelenlet-{}", std::process::id()));
        let e = event("Teszt", (18, 20));
        record(&dir, &e, "Kovács Anna", 1).unwrap();
        record(&dir, &e, "kovacs  ANNA", 2).unwrap();
        record(&dir, &e, "Szabó József", 3).unwrap();
        let sheet = load(&dir, &key(&e)).unwrap();
        assert_eq!(sheet.names.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Kovács Anna", "Szabó József"]);
        assert_eq!(stored(&dir).len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
