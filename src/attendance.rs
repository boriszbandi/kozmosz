//! Attendance sheets for the club's events (owner's request 2026-10-20). Every calendar event has
//! one: an admin opens it on the dashboard and shows its QR code, and whoever scans it lands on
//! /jelen/<token> and signs in one of two ways: with Discord (anyone holding a role on the club's
//! server; their server nickname, else Discord name, goes on the sheet with their user name), or
//! by typing their full name. The names are matched against the membership sheet (a Google Sheet
//! read with the Drive service account), and all sheets together are one CSV that the club's
//! Google Sheet imports by itself (IMPORTDATA), so attendance reaches the spreadsheet untyped.
//!
//! - The QR link's token is a tag of the event ([`admin::tag`]): unguessable, nothing to store,
//!   and void once admin.key is deleted. So is the export link's key.
//! - A sheet takes names from an hour before the event until three hours after it.
//! - Each event's names are a small JSON file in the state directory (jelenlet/<event>.json),
//!   written when its QR code is first shown. It holds the event too, so the code keeps working
//!   and the sheet stays listed after the event has left the calendar (owner's request).
//! - After signing, the phone keeps a ticket (a tagged cookie) that shows "Kitöltve".

use std::{
    cmp::Reverse,
    collections::HashSet,
    path::{Path as FsPath, PathBuf},
    sync::{Arc, Mutex, PoisonError, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aws_lc_rs::constant_time;
use axum::{
    extract::{Form, Path, Query, State},
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
    admin::{self, Admin, Me, Member},
    calendar::{self, Event, LocalDateTime},
    drive::{slugify, write_atomic, ServiceAccount, ENV_CREDENTIALS},
    icons,
    media::Icon,
    pages::{AdminLayout, NotFound, PageHeader, Section},
    site,
};

/// The membership sheet's ID (the long part of its URL). The sheet is shared (Viewer) with the
/// service account of KOZMOSZ_DRIVE_CREDENTIALS. None: names are not matched to members.
/// It also holds confidential data (ID card numbers, e-mail, Neptun codes): only the header row
/// and the name and Discord columns are ever requested, see [`read_members`].
const MEMBERS_SHEET: Option<&str> = Some("1CaJrts2ldl2h2JSdp3m7H5GPRKJ-_LQRrwPYBRSCcls");
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
    /// Start, Unix seconds.
    start: i64,
    /// End, Unix seconds (after the last day for all-day events). Sheets written before it was
    /// kept: two hours after the start.
    #[serde(default)]
    end: Option<i64>,
    names: Vec<Signed>,
}

#[derive(Serialize, Deserialize)]
struct Signed {
    name: String,
    /// Unix seconds.
    at: i64,
    /// The Discord user name, when signed with Discord.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    discord: Option<String>,
}

/// One person on the membership sheet.
struct Enrolled {
    name: String,
    /// Their Discord user name (lowercase), when the sheet has a Discord column.
    discord: Option<String>,
}

#[derive(Default)]
struct Members {
    people: Vec<Enrolled>,
    status: String,
}

#[derive(Deserialize)]
struct Values {
    #[serde(default)]
    values: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct Batch {
    #[serde(rename = "valueRanges")]
    value_ranges: Vec<Values>,
}

#[derive(Deserialize)]
pub struct Signature {
    nev: String,
}

#[derive(Deserialize)]
pub struct ExportKey {
    k: Option<String>,
}

struct Row {
    key: String,
    title: String,
    when: String,
    open: bool,
    count: usize,
}

/// The event a sheet is for: from the calendar, or from its sheet on disk once the calendar no
/// longer has it (deleted, or not downloaded yet).
struct Occasion {
    key: String,
    title: String,
    when: String,
    start: i64,
    end: i64,
}

impl Occasion {
    fn of_event(event: &Event) -> Self {
        Self {
            key: key(event),
            title: event.title.clone(),
            when: when(event),
            start: unix(event.start),
            end: unix(event.end) + if event.all_day { 24 * 3600 } else { 0 },
        }
    }

    fn of_sheet(key: String, sheet: &Sheet) -> Self {
        let end = sheet.end.unwrap_or(sheet.start + 2 * 3600);
        Self { key, title: sheet.title.clone(), when: sheet.when.clone(), start: sheet.start, end }
    }

    /// Names are taken from an hour before the start until three hours after the end.
    fn is_open(&self, now: i64) -> bool {
        (self.start - OPENS_BEFORE..=self.end + CLOSES_AFTER).contains(&now)
    }

    fn sheet(&self) -> Sheet {
        Sheet { title: self.title.clone(), when: self.when.clone(), start: self.start, end: Some(self.end), names: Vec::new() }
    }
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

/// The event whose sheet the token `given` opens: in the calendar, else on disk. Only real
/// tokens pass, so a `given` that does is safe in a URL.
fn find(given: &str) -> Option<Occasion> {
    let fits = |key: &str| constant_time::verify_slices_are_equal(token(key).as_bytes(), given.as_bytes()).is_ok();
    let calendar = calendar::current();
    let listed = calendar.upcoming.iter().chain(&calendar.past).find(|e| fits(&key(e))).map(Occasion::of_event);
    listed.or_else(|| stored(&dir()).into_iter().find(|(key, _)| fits(key)).map(|(key, sheet)| Occasion::of_sheet(key, &sheet)))
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

/// A Unix time in Budapest, formatted ("%H:%M", "%Y-%m-%d").
fn local(at: i64, format: &str) -> String {
    let zone = TimeZone::get(ZONE).unwrap_or(TimeZone::UTC);
    Timestamp::from_second(at).map(|t| t.to_zoned(zone).strftime(format).to_string()).unwrap_or_default()
}

fn clock(at: i64) -> String {
    local(at, "%H:%M")
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
fn member<'a>(name: &str, people: &'a [Enrolled]) -> Option<&'a str> {
    let mut typed = words(name);
    if typed.len() < 2 {
        return None;
    }
    // Longer words first, so "a" does not take the word that "anna" needs.
    typed.sort_by_key(|w| Reverse(w.len()));
    let mut fits = people.iter().filter(|p| covers(&words(&p.name), &typed));
    let first = fits.next()?;
    fits.next().is_none().then_some(first.name.as_str())
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

/// The member someone on a sheet is: by their Discord user name when the membership sheet lists
/// it, else by name ([`member`]).
fn matched<'a>(signed: &Signed, people: &'a [Enrolled]) -> Option<&'a str> {
    let by_discord = signed.discord.as_deref().and_then(|d| people.iter().find(|p| p.discord.as_deref() == Some(d)));
    by_discord.map(|p| p.name.as_str()).or_else(|| member(&signed.name, people))
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

/// Puts `name` on the event's sheet. Someone already there (the same Discord account, or the
/// same name ignoring case, accents and spacing) is not added again.
fn record(dir: &FsPath, occasion: &Occasion, name: &str, discord: Option<&str>, now: i64) -> Result<(), String> {
    let _one = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
    let key = &occasion.key;
    let mut sheet = load(dir, key).unwrap_or_else(|| occasion.sheet());
    let folded = words(name);
    let there = |s: &Signed| (discord.is_some() && s.discord.as_deref() == discord) || words(&s.name) == folded;
    if sheet.names.iter().any(there) {
        return Ok(());
    }
    if sheet.names.len() >= MAX_NAMES {
        return Err(format!("{key}: the sheet is full"));
    }
    sheet.names.push(Signed { name: name.to_owned(), at: now, discord: discord.map(str::to_owned) });
    save(dir, key, &sheet)
}

fn save(dir: &FsPath, key: &str, sheet: &Sheet) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let json = serde_json::to_vec(sheet).map_err(|e| e.to_string())?;
    write_atomic(&dir.join(format!("{key}.json")), &json)
}

/// Writes the event's (empty) sheet when its QR code is shown, so the code keeps working even if
/// the event leaves the calendar.
fn keep(dir: &FsPath, occasion: &Occasion) {
    let _one = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
    if load(dir, &occasion.key).is_none() {
        if let Err(err) = save(dir, &occasion.key, &occasion.sheet()) {
            error!("attendance: {err}");
        }
    }
}

fn members() -> Arc<Members> {
    MEMBERS.read().unwrap_or_else(PoisonError::into_inner).clone().unwrap_or_else(|| {
        Arc::new(Members { people: Vec::new(), status: "A tagsági lista nincs beállítva.".into() })
    })
}

fn publish(members: Members) {
    *MEMBERS.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(members));
}

/// The columns to read, from the header row: the one headed "Név", "Teljes név" or "Name"
/// (required: never a guess, the sheet has confidential columns), and one whose header starts
/// with "Discord", if there is one.
fn columns(header: &[String]) -> Option<(usize, Option<usize>)> {
    let header: Vec<String> = header.iter().map(|cell| slugify(cell)).collect();
    let name = header.iter().position(|h| matches!(h.as_str(), "nev" | "teljes-nev" | "name" | "full-name"))?;
    Some((name, header.iter().position(|h| h.starts_with("discord"))))
}

/// A column's letter: 0 is A, 25 is Z, 26 is AA.
fn letter(column: usize) -> String {
    let mut n = column + 1;
    let mut out = Vec::new();
    while n > 0 {
        out.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).expect("ASCII")
}

/// The people from the name column and the Discord column (same rows, below the header).
fn people(names: &[String], discords: &[String]) -> Vec<Enrolled> {
    names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| {
            let name = name.trim();
            let discord = discords
                .get(i)
                .map(|d| d.trim().trim_start_matches('@').to_lowercase())
                .filter(|d| !d.is_empty());
            (!name.is_empty()).then(|| Enrolled { name: name.to_owned(), discord })
        })
        .collect()
}

/// Reads the header row, then only the name and Discord columns: the rest of the sheet (ID card
/// numbers and the like) never leaves Google.
async fn read_members(client: &reqwest::Client, account: &mut ServiceAccount, sheet: &str) -> Result<Vec<Enrolled>, String> {
    let token = account.token(client).await?;
    let base = format!("https://sheets.googleapis.com/v4/spreadsheets/{sheet}/values");
    let fail = |e: reqwest::Error| e.without_url().to_string();
    let header: Values = client
        .get(format!("{base}/1:1"))
        .bearer_auth(&token)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(fail)?
        .json()
        .await
        .map_err(fail)?;
    let (name, discord) = header
        .values
        .first()
        .and_then(|row| columns(row))
        .ok_or("no column headed \"Név\" in the first row")?;
    let mut ranges = vec![("ranges", format!("{0}2:{0}", letter(name)))];
    ranges.extend(discord.map(|d| ("ranges", format!("{0}2:{0}", letter(d)))));
    ranges.push(("majorDimension", "COLUMNS".to_owned()));
    let batch: Batch = client
        .get(format!("{base}:batchGet"))
        .query(&ranges)
        .bearer_auth(&token)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(fail)?
        .json()
        .await
        .map_err(fail)?;
    let column = |i: usize| batch.value_ranges.get(i).and_then(|v| v.values.first().cloned()).unwrap_or_default();
    Ok(people(&column(0), &if discord.is_some() { column(1) } else { Vec::new() }))
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
            let status = "A tagsági listához nincs beállítva a service account.".into();
            publish(Members { people: Vec::new(), status });
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
                Ok(people) => {
                    log!("attendance: {} people in the membership sheet", people.len());
                    let status = format!("{} név a tagsági listában (frissítve {}).", people.len(), clock(now()));
                    publish(Members { people, status });
                }
                Err(err) => {
                    error!("attendance: cannot read the membership sheet: {err}");
                    let people = members().people.iter().map(|p| Enrolled { name: p.name.clone(), discord: p.discord.clone() }).collect();
                    let status = "A tagsági lista most nem olvasható, a legutóbbi névsorral egyeztetek.".into();
                    publish(Members { people, status });
                }
            }
            tokio::time::sleep(REFRESH_EVERY).await;
        }
    });
}

/// Back to the sheet's page with a problem to explain (see [`check_in_page`]).
pub fn problem(given: &str, code: &str) -> Response {
    match find(given) {
        Some(_) => Redirect::to(&format!("/jelen/{given}?hiba={code}")).into_response(),
        None => Redirect::to("/").into_response(),
    }
}

/// Puts the name on the sheet and sends the phone back to the page with its ticket.
fn enter(given: &str, occasion: &Occasion, name: &str, discord: Option<&str>) -> Response {
    let (page, now) = (format!("/jelen/{given}"), now());
    match record(&dir(), occasion, name, discord, now) {
        Ok(()) => {
            let mut response = Redirect::to(&page).into_response();
            let cookie = format!(
                "{TICKET}={}; Path={page}; Max-Age=43200; HttpOnly; Secure; SameSite=Lax",
                ticket_value(&occasion.key, name, now),
            );
            // Base64, digits and the token: always a valid header.
            response.headers_mut().append(header::SET_COOKIE, HeaderValue::from_str(&cookie).expect("ASCII cookie"));
            response
        }
        Err(err) => {
            error!("attendance: {err}");
            problem(given, "mentes")
        }
    }
}

/// POST /jelen/{token}: a typed name.
pub async fn sign(Path(given): Path<String>, Form(form): Form<Signature>) -> Response {
    let Some(occasion) = find(&given) else { return StatusCode::NOT_FOUND.into_response() };
    if !occasion.is_open(now()) {
        return Redirect::to(&format!("/jelen/{given}")).into_response();
    }
    match clean_name(&form.nev) {
        Some(name) => enter(&given, &occasion, &name, None),
        None => problem(&given, "nev"),
    }
}

/// GET /jelen/{token}/discord: the "Igazolás Discorddal" button, off to Discord's sign-in. The
/// answer comes back through admin::callback to [`check_in_with`].
pub async fn discord(State(admin): State<Arc<Admin>>, Path(given): Path<String>) -> Response {
    let Some(occasion) = find(&given) else { return StatusCode::NOT_FOUND.into_response() };
    if !occasion.is_open(now()) || !admin.ready() {
        return problem(&given, "discord");
    }
    admin::to_discord(Some(&given))
}

/// A Discord sign-in for the sheet `given` came back. Anyone with a role on the club's server
/// goes on the sheet under their server nickname (else their Discord name), with their user
/// name; Discord vouches for both.
pub fn check_in_with(given: &str, me: &Me, member: Option<&Member>) -> Response {
    let Some(occasion) = find(given) else { return Redirect::to("/").into_response() };
    if !occasion.is_open(now()) {
        return Redirect::to(&format!("/jelen/{given}")).into_response();
    }
    let Some(member) = member.filter(|m| m.has_a_role()) else {
        log!("attendance: {} has no role on the club's Discord server", me.username);
        return problem(given, "tag");
    };
    let shown = member.nick.as_deref().or(me.global_name.as_deref()).unwrap_or(&me.username);
    let name: String = shown.trim().chars().take(60).collect();
    enter(given, &occasion, &name, Some(&me.username.to_lowercase()))
}

/// A CSV field: quoted, and never read as a formula (a Discord nickname could start with "=").
fn quote(field: &str) -> String {
    let safe = if field.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{field}") } else { field.to_owned() };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

/// GET /admin/jelenlet/{key}/csv (behind admin::guard): one sheet the way Hungarian Excel opens
/// a CSV (semicolons, byte-order mark).
pub async fn csv(Path(key): Path<String>) -> Response {
    if !valid_key(&key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // No file yet: nobody has signed, an empty table.
    let names = load(&dir(), &key).map(|s| s.names).unwrap_or_default();
    let members = members();
    let mut out = String::from("\u{feff}Név;Tag;Discord;Időpont\r\n");
    for s in &names {
        let fields = [
            quote(&s.name),
            quote(matched(s, &members.people).unwrap_or_default()),
            quote(s.discord.as_deref().unwrap_or_default()),
            clock(s.at),
        ];
        out.push_str(&fields.join(";"));
        out.push_str("\r\n");
    }
    let headers = [
        (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()),
        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"jelenlet-{key}.csv\"")),
    ];
    (headers, out).into_response()
}

fn export_key() -> String {
    admin::tag("jelenlet-export")
}

/// GET /jelenlet.csv?k=<key>: every sheet as one CSV for Google Sheets' IMPORTDATA, which
/// fetches it about hourly. The key keeps the names private; the dashboard shows the formula.
pub async fn export(Query(given): Query<ExportKey>) -> Response {
    let key_ok = given
        .k
        .as_deref()
        .is_some_and(|k| constant_time::verify_slices_are_equal(k.as_bytes(), export_key().as_bytes()).is_ok());
    if !key_ok {
        return StatusCode::NOT_FOUND.into_response();
    }
    let members = members();
    let mut sheets = stored(&dir());
    sheets.sort_by_key(|(_, sheet)| sheet.start);
    let mut out = String::from("Nap,Esemény,Név,Tag,Discord,Időpont\r\n");
    for (_, sheet) in &sheets {
        for s in &sheet.names {
            let fields = [
                local(sheet.start, "%Y-%m-%d"),
                quote(&sheet.title),
                quote(&s.name),
                quote(matched(s, &members.people).unwrap_or_default()),
                quote(s.discord.as_deref().unwrap_or_default()),
                clock(s.at),
            ];
            out.push_str(&fields.join(","));
            out.push_str("\r\n");
        }
    }
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8")], out).into_response()
}

fn row(dir: &FsPath, occasion: Occasion, now: i64) -> Row {
    let count = load(dir, &occasion.key).map_or(0, |s| s.names.len());
    let open = occasion.is_open(now);
    Row { key: occasion.key, title: occasion.title, when: occasion.when, open, count }
}

/// /admin: the sheets of the coming events, then of the earlier ones (the calendar's recent past
/// and every older sheet on disk), and the formula that brings them all into Google Sheets.
pub fn events_page() -> AnyView {
    let (calendar, dir, now) = (calendar::current(), dir(), now());
    let coming: Vec<Row> = calendar.upcoming.iter().take(10).map(|e| row(&dir, Occasion::of_event(e), now)).collect();
    let mut earlier: Vec<(i64, Row)> =
        calendar.past.iter().map(|e| (unix(e.start), row(&dir, Occasion::of_event(e), now))).collect();
    let listed: HashSet<String> = coming.iter().chain(earlier.iter().map(|(_, r)| r)).map(|r| r.key.clone()).collect();
    for (key, sheet) in stored(&dir) {
        if !listed.contains(&key) {
            earlier.push((sheet.start, row(&dir, Occasion::of_sheet(key, &sheet), now)));
        }
    }
    earlier.sort_by_key(|(start, _)| Reverse(*start));
    let earlier = earlier.into_iter().map(|(_, row)| row).collect();
    let formula = format!("=IMPORTDATA(\"{}/jelenlet.csv?k={}\")", site::ORIGIN, export_key());
    view! {
        <AdminLayout title="Jelenléti ívek" section=Section::Sheets>
            <p class="admin-status">{members().status.clone()}</p>
            <h2 class="admin-group">"Következő események"</h2>
            {event_list(coming)}
            <h2 class="admin-group">"Korábbi események"</h2>
            {event_list(earlier)}
            <h2 class="admin-group">"Google Táblázatba"</h2>
            <p class="admin-note">
                "Egy új munkalap A1 cellájába írva a táblázat magától behúzza az összes jelenléti ívet, és kb. óránként frissíti:"
            </p>
            <code class="admin-formula">{formula}</code>
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

/// How often each member came, and to what last: (times, last event's title and day), in the
/// membership sheet's order.
fn tally(people: &[Enrolled], sheets: &[(String, Sheet)]) -> Vec<(usize, Option<(String, i64)>)> {
    let mut seen: Vec<(usize, Option<(String, i64)>)> = vec![(0, None); people.len()];
    for (_, sheet) in sheets {
        // Once per event, however many entries point at the same member.
        let mut here = HashSet::new();
        for signed in &sheet.names {
            let Some(name) = matched(signed, people) else { continue };
            let Some(i) = people.iter().position(|p| p.name == name) else { continue };
            if here.insert(i) {
                seen[i].0 += 1;
                if seen[i].1.as_ref().is_none_or(|(_, start)| *start < sheet.start) {
                    seen[i].1 = Some((sheet.title.clone(), sheet.start));
                }
            }
        }
    }
    seen
}

/// /admin/tagsag: everyone on the membership sheet, with how many events they came to.
pub fn members_page() -> AnyView {
    let members = members();
    let sheets = stored(&dir());
    let counts = tally(&members.people, &sheets);
    let rows: Vec<_> = members.people.iter().map(|p| (p.name.clone(), p.discord.clone())).zip(counts).collect();
    view! {
        <AdminLayout title="Tagság" section=Section::Members>
            <p class="admin-status">{members.status.clone()}</p>
            {if rows.is_empty() {
                view! { <p class="empty-note">"Amint beállítjuk a tagsági táblázatot, itt lesz a névsor."</p> }.into_any()
            } else {
                let most = rows.iter().map(|(_, (times, _))| *times).max().unwrap_or(0).max(1);
                let came = rows.iter().filter(|(_, (times, _))| *times > 0).count();
                let total = rows.len();
                view! {
                    <div class="roster-stats">
                        <p><strong class="mono">{total}</strong>" tag"</p>
                        <p><strong class="mono">{came}</strong>" volt már eseményen"</p>
                        <p><strong class="mono">{sheets.len()}</strong>" jelenléti ív"</p>
                    </div>
                    <table class="roster">
                        <thead>
                            <tr>
                                <th scope="col">"Név"</th>
                                <th scope="col">"Discord"</th>
                                <th scope="col" class="roster-num">"Alkalom"</th>
                                <th scope="col">"Utoljára"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {rows
                                .into_iter()
                                .map(|((name, discord), (times, last))| {
                                    // The bar: attendance against the most anyone came.
                                    let bar = format!("--share:{}%", times * 100 / most);
                                    view! {
                                        <tr>
                                            <th scope="row" class="roster-name">{name}</th>
                                            <td class="roster-discord" data-label="Discord">
                                                {discord.map_or_else(|| "nincs".to_owned(), |d| format!("@{d}"))}
                                            </td>
                                            <td class="roster-num" data-label="Alkalom">
                                                <span class="roster-bar" style=bar aria-hidden="true"></span>
                                                <span class="mono">{times}</span>
                                            </td>
                                            <td class="roster-last" data-label="Utoljára">
                                                {match last {
                                                    Some((title, start)) => view! {
                                                        <span>{title}</span>
                                                        " "
                                                        <span class="mono">{local(start, "%Y.%m.%d.")}</span>
                                                    }
                                                    .into_any(),
                                                    None => view! { <span class="roster-none">"még nem volt"</span> }.into_any(),
                                                }}
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
                .into_any()
            }}
        </AdminLayout>
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
    let sheet = valid.then(|| load(&dir, key)).flatten();
    let calendar = calendar::current();
    let listed = calendar.upcoming.iter().chain(&calendar.past).find(|e| self::key(e) == key).map(Occasion::of_event);
    let occasion = match (valid, listed, &sheet) {
        (false, _, _) => None,
        (true, Some(listed), _) => Some(listed),
        (true, None, sheet) => sheet.as_ref().map(|sheet| Occasion::of_sheet(key.to_owned(), sheet)),
    };
    let Some(occasion) = occasion else { return view! { <NotFound/> }.into_any() };
    keep(&dir, &occasion);
    let state = if occasion.is_open(now) {
        "Nyitva: most lehet beírni a neveket."
    } else if now < occasion.start {
        "Az esemény előtt egy órával nyílik."
    } else {
        "Lezárva."
    };
    let (title, when) = (occasion.title.clone(), occasion.when.clone());
    let link = format!("{}/jelen/{}", site::ORIGIN, token(key));
    let qr = qr_svg(&link).map(|svg| (svg, link));
    let members = members();
    let names = sheet.map(|s| s.names).unwrap_or_default();
    let listed: Vec<(String, Option<String>, Option<String>, String)> = names
        .iter()
        .map(|s| (s.name.clone(), matched(s, &members.people).map(str::to_owned), s.discord.clone(), clock(s.at)))
        .collect();
    let in_list = listed.iter().filter(|(_, m, _, _)| m.is_some()).count();
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
                        <strong class="mono">{in_list}</strong>
                        " a tagsági listán"
                    </p>
                    {if listed.is_empty() {
                        view! { <p class="empty-note">"Még senki sem írta be a nevét."</p> }.into_any()
                    } else {
                        view! {
                            <ol class="sheet-list">
                                {listed
                                    .into_iter()
                                    .map(|(name, member, discord, at)| view! {
                                        <li>
                                            <span class="sheet-name">{name}</span>
                                            <span class="sheet-member">
                                                {member.unwrap_or_else(|| "nem találom a tagok között".into())}
                                                {discord.map(|d| format!(" · Discord: @{d}"))}
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

/// /jelen/<token>: Discord or a typed name, or this phone's ticket once it is on the sheet.
/// `problem` after a refused try: "nev" (not a name), "discord" (the Discord sign-in failed),
/// "tag" (no role on the club's server), "mentes" (could not save).
pub fn check_in_page(given: &str, problem: Option<&str>) -> AnyView {
    let Some(occasion) = find(given) else { return view! { <NotFound/> }.into_any() };
    let open = occasion.is_open(now());
    let headers = use_context::<Parts>().map(|parts| parts.headers).unwrap_or_default();
    let ticket = ticket(&headers, &occasion.key);
    let problem = problem.map(|p| match p {
        "nev" => "Írd be a teljes neved: legalább 2, legfeljebb 60 betű.",
        "discord" => "A Discordos igazolás most nem sikerült. Írd be a teljes neved.",
        "tag" => "Ezzel a Discord-fiókkal nincs rangod a Kozmosz szerverén. Írd be a teljes neved.",
        _ => "Most nem sikerült elmenteni, próbáld újra.",
    });
    let body = if let Some((name, at)) = ticket {
        view! {
            <div class="ticket" role="status">
                <div class="liftoff" inner_html=LIFTOFF></div>
                <p class="ticket-ok">"Kitöltve"</p>
                <p class="ticket-name">{name}</p>
                <p class="ticket-meta">{occasion.title.clone()} " · " <span class="mono">{clock(at)}</span></p>
            </div>
        }
        .into_any()
    } else if !open {
        view! { <p class="empty-note">"Ez a jelenléti ív most nincs nyitva."</p> }.into_any()
    } else {
        view! {
            <div class="checkin">
                {problem.map(|p| view! { <p class="checkin-problem" role="alert">{p}</p> })}
                <a class="button button-signal" href=format!("/jelen/{given}/discord")>
                    <Icon svg=icons::DISCORD_LOGO/>
                    "Igazolás Discorddal"
                </a>
                <p class="checkin-or">"vagy"</p>
                <form class="checkin-form" method="post">
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
                    <button class="button button-line" type="submit">"Itt vagyok"</button>
                </form>
                <p id="nev-info" class="checkin-info">"A neved a szervezők jelenléti ívére kerül, csak ők látják."</p>
            </div>
        }
        .into_any()
    };
    view! {
        <Title text="Jelenléti ív"/>
        <Meta name="robots" content="noindex"/>
        <PageHeader title=occasion.title.clone()>
            <p class="page-lead">{occasion.when.clone()}</p>
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

    fn enrolled(names: &[(&str, Option<&str>)]) -> Vec<Enrolled> {
        names.iter().map(|&(name, discord)| Enrolled { name: name.into(), discord: discord.map(str::to_owned) }).collect()
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
        let people = enrolled(&[("Kovács Anna Petra", None), ("Kovács András", None), ("Dr. Szabó József", None)]);
        assert_eq!(member("anna kovacs", &people), Some("Kovács Anna Petra"));
        assert_eq!(member("Kovács Anna", &people), Some("Kovács Anna Petra"));
        assert_eq!(member("Szabó József", &people), Some("Dr. Szabó József"));
        assert_eq!(member("Kovács A.", &people), None, "two members fit");
        assert_eq!(member("Anna", &people), None, "one word is not enough");
        assert_eq!(member("Kiss Péter", &people), None);
    }

    #[test]
    fn discord_user_names_match_exactly() {
        let people = enrolled(&[("Végh Máté", Some("vmate04")), ("Kovács Anna", None)]);
        let signed = |name: &str, discord: Option<&str>| Signed { name: name.into(), at: 0, discord: discord.map(str::to_owned) };
        assert_eq!(matched(&signed("Máté", Some("vmate04")), &people), Some("Végh Máté"), "a nickname alone");
        assert_eq!(matched(&signed("Anna K", Some("anna_k")), &people), Some("Kovács Anna"), "by name");
        assert_eq!(matched(&signed("Valaki", Some("valaki")), &people), None);
    }

    #[test]
    fn sheets_open_an_hour_before_and_close_three_hours_after() {
        let e = event("Csillagászati est", (18, 20));
        let o = Occasion::of_event(&e);
        assert!(!o.is_open(unix(e.start) - OPENS_BEFORE - 1));
        assert!(o.is_open(unix(e.start) - OPENS_BEFORE));
        assert!(o.is_open(unix(e.end) + CLOSES_AFTER));
        assert!(!o.is_open(unix(e.end) + CLOSES_AFTER + 1));
        // 18:00 in Budapest summer time is 16:00 UTC.
        assert_eq!(unix(e.start) % 86_400, 16 * 3600);
        // Off the calendar, the sheet on disk keeps the same hours.
        let kept = Occasion::of_sheet(o.key.clone(), &o.sheet());
        assert_eq!((kept.start, kept.end, kept.title.as_str()), (o.start, o.end, "Csillagászati est"));
    }

    #[test]
    fn keys_are_safe_file_names() {
        assert_eq!(key(&event("Csillagászati est!", (18, 20))), "2026-10-21-1800-csillagaszati-est");
        assert!(valid_key("2026-10-21-1800-csillagaszati-est"));
        assert!(!valid_key("../admin"));
        assert!(!valid_key(""));
    }

    #[test]
    fn only_the_name_and_discord_columns_are_read() {
        let header = |cells: &[&str]| cells.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>();
        let club = header(&["Név", "Mail", "BME Neptun", "Kar", "Jelenlegi tagsági státusz", "Személyi igazolvány szám", "Discord név"]);
        assert_eq!(columns(&club), Some((0, Some(6))));
        assert_eq!(columns(&header(&["E-mail", "Teljes név"])), Some((1, None)));
        assert_eq!(columns(&header(&["1. oszlop", "KSP"])), None, "no guessing which column has names");
        assert_eq!((letter(0), letter(6), letter(25), letter(26), letter(27)), ("A".into(), "G".into(), "Z".into(), "AA".into(), "AB".into()));
        let names = header(&[" Kovács Anna ", "", "Végh Máté"]);
        let found = people(&names, &header(&["@Anna_K", "x"]));
        let found: Vec<_> = found.iter().map(|p| (p.name.as_str(), p.discord.as_deref())).collect();
        assert_eq!(found, [("Kovács Anna", Some("anna_k")), ("Végh Máté", None)]);
    }

    #[test]
    fn members_are_counted_once_per_event() {
        let people = enrolled(&[("Végh Máté", Some("vmate04")), ("Kovács Anna", None)]);
        let sheet = |start: i64, names: &[(&str, Option<&str>)]| Sheet {
            title: format!("E{start}"),
            when: String::new(),
            start,
            end: None,
            names: names.iter().map(|&(n, d)| Signed { name: n.into(), at: 0, discord: d.map(str::to_owned) }).collect(),
        };
        let sheets = [
            ("a".to_owned(), sheet(1, &[("Máté", Some("vmate04")), ("Végh Máté", None)])),
            ("b".to_owned(), sheet(2, &[("Végh Máté", None), ("Valaki Más", None)])),
        ];
        let counts = tally(&people, &sheets);
        assert_eq!(counts[0], (2, Some(("E2".to_owned(), 2))));
        assert_eq!(counts[1], (0, None));
    }

    #[test]
    fn csv_fields_are_never_formulas() {
        assert_eq!(quote("Kovács \"Kovi\" Anna"), "\"Kovács \"\"Kovi\"\" Anna\"");
        assert_eq!(quote("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(quote("-Anna"), "\"'-Anna\"");
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
        let e = Occasion::of_event(&event("Teszt", (18, 20)));
        keep(&dir, &e);
        assert!(load(&dir, &e.key).unwrap().names.is_empty(), "kept when its QR code is shown");
        record(&dir, &e, "Kovács Anna", None, 1).unwrap();
        record(&dir, &e, "kovacs  ANNA", None, 2).unwrap();
        record(&dir, &e, "Máté", Some("vmate04"), 3).unwrap();
        record(&dir, &e, "Végh Máté", Some("vmate04"), 4).unwrap();
        keep(&dir, &e);
        let sheet = load(&dir, &e.key).unwrap();
        assert_eq!(sheet.names.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Kovács Anna", "Máté"]);
        assert_eq!(stored(&dir).len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
