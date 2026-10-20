//! Sign-in for the club's admins, without passwords: the sign-in icon in the footer starts
//! Discord's OAuth2 flow, and only members of the club's Discord server who hold the admin role
//! get in (no two-factor requirement: the owner relaxed it, the site keeps nothing sensitive).
//! Discord tells it through the user's own sign-in (scopes `identify` and `guilds.members.read`),
//! so the bot's token is never needed here. The same flow, with [`to_discord`]'s `next`, signs
//! people onto attendance sheets (src/attendance.rs).
//!
//! The session is a signed cookie for 30 days (the owner did not want to sign in every day), so a
//! taken-away role ends access at the latest 30 days later. The signing key lives in the
//! service's state directory: restarts and updates keep everyone signed in, and deleting the key
//! file signs everyone out.
//!
//! GET /belepes starts (or goes straight to /admin when signed in), GET /belepes/discord is where
//! Discord sends the browser back, POST /kilepes signs out, and [`guard`] lets only signed-in
//! admins reach the /admin page (src/pages/admin.rs).

use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aws_lc_rs::{
    constant_time, hmac,
    rand::{SecureRandom, SystemRandom},
};
use axum::{
    extract::{Query, Request, State},
    http::{
        header::{CACHE_CONTROL, COOKIE, LOCATION, SET_COOKIE},
        HeaderMap, HeaderValue, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use leptos::logging::{error, log};
use serde::{de::DeserializeOwned, Deserialize};

use crate::{attendance, site};

/// The club's Discord application (its OAuth2 client), server and admin role.
const CLIENT_ID: &str = "1294343076021534790";
const GUILD_ID: &str = "1139207800254373978";
const ADMIN_ROLE: &str = "1159537506056941719";
/// The application's client secret (Developer Portal, OAuth2): from the environment only.
const ENV_SECRET: &str = "KOZMOSZ_DISCORD_SECRET";

const API: &str = "https://discord.com/api/v10";
/// Where Discord sends the browser back: must be listed among the application's redirects.
const CALLBACK: &str = "/belepes/discord";
const SESSION: &str = "kozmosz_admin";
const STATE: &str = "kozmosz_state";
/// Set when a sign-in is an attendance check-in: the sheet's token (src/attendance.rs).
const NEXT: &str = "kozmosz_next";
const SESSION_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const KEY_FILE: &str = "admin.key";

pub struct Admin {
    secret: Option<String>,
    client: reqwest::Client,
}

static KEY: OnceLock<hmac::Key> = OnceLock::new();

#[derive(Deserialize)]
struct Token {
    access_token: String,
}

#[derive(Deserialize)]
pub struct Me {
    pub id: String,
    pub username: String,
    /// The display name, when the user has set one.
    #[serde(default)]
    pub global_name: Option<String>,
}

/// The user on the club's server.
#[derive(Deserialize)]
pub struct Member {
    roles: Vec<String>,
    /// The server nickname, when set.
    #[serde(default)]
    pub nick: Option<String>,
}

impl Member {
    /// Holds any role on the server (attendance by Discord: members, not passers-by).
    pub fn has_a_role(&self) -> bool {
        !self.roles.is_empty()
    }
}

#[derive(Deserialize)]
pub struct Callback {
    code: Option<String>,
    state: Option<String>,
}

impl Admin {
    pub fn from_env() -> Arc<Self> {
        let secret = std::env::var(ENV_SECRET).ok().filter(|s| !s.is_empty());
        if secret.is_none() {
            log!("admin: sign-in is off until {ENV_SECRET} is set");
        }
        let client = reqwest::Client::builder()
            .user_agent(format!("kozmosz-site/{} (+{})", env!("CARGO_PKG_VERSION"), site::ORIGIN))
            .https_only(true)
            .timeout(Duration::from_secs(15))
            .build()
            .expect("HTTP client");
        Arc::new(Self { secret, client })
    }

    fn signed_in(&self, headers: &HeaderMap) -> bool {
        cookie(headers, SESSION).is_some_and(|value| verify(key(), value, now()))
    }

    /// Whether Discord sign-in is set up (the client secret is there).
    pub fn ready(&self) -> bool {
        self.secret.is_some()
    }

    /// Trades Discord's one-time code for the user and their membership of the club's server.
    async fn identify(&self, code: &str) -> Result<(Me, Option<Member>), String> {
        let secret = self.secret.as_deref().ok_or("no client secret")?;
        let token: Token = self
            .client
            .post(format!("{API}/oauth2/token"))
            .basic_auth(CLIENT_ID, Some(secret))
            .form(&[("grant_type", "authorization_code"), ("code", code), ("redirect_uri", &callback_url())])
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| format!("token exchange failed: {}", e.without_url()))?
            .json()
            .await
            .map_err(|e| format!("bad token response: {e}"))?;
        let me: Me = self.get(&token.access_token, "/users/@me").await?.ok_or("no user")?;
        let member: Option<Member> =
            self.get(&token.access_token, &format!("/users/@me/guilds/{GUILD_ID}/member")).await?;
        Ok((me, member))
    }

    /// GET from Discord's API as the user; `None` for 404 (not a member of the server).
    async fn get<T: DeserializeOwned>(&self, token: &str, path: &str) -> Result<Option<T>, String> {
        let fail = |e: reqwest::Error| format!("{path}: {}", e.without_url());
        let response = self.client.get(format!("{API}{path}")).bearer_auth(token).send().await.map_err(fail)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response.error_for_status().map_err(fail)?.json().await.map(Some).map_err(fail)
    }
}

/// Members of the club's server with the admin role.
fn allowed(me: &Me, member: Option<&Member>) -> Result<(), String> {
    let member = member.ok_or_else(|| format!("{} is not on the club's Discord server", me.username))?;
    if !member.roles.iter().any(|r| r == ADMIN_ROLE) {
        return Err(format!("{} lacks the admin role", me.username));
    }
    Ok(())
}

/// GET /belepes.
pub async fn login(State(admin): State<Arc<Admin>>, headers: HeaderMap) -> Response {
    if admin.signed_in(&headers) {
        return redirect("/admin", &[]);
    }
    if !admin.ready() {
        return (StatusCode::SERVICE_UNAVAILABLE, "A belépés még nincs beállítva.").into_response();
    }
    to_discord(None)
}

/// Off to Discord's consent page (skipped by Discord when the app was approved before). `next`:
/// the attendance sheet whose check-in this is, if not an admin sign-in.
pub fn to_discord(next: Option<&str>) -> Response {
    let state = random_token();
    let url = format!(
        "https://discord.com/oauth2/authorize?response_type=code&client_id={CLIENT_ID}\
         &scope=identify%20guilds.members.read&prompt=none&state={state}&redirect_uri={}",
        callback_url().replace(':', "%3A").replace('/', "%2F"),
    );
    let next = match next {
        Some(token) => format!("{NEXT}={token}; Path=/belepes; Max-Age=600; HttpOnly; Secure; SameSite=Lax"),
        None => forget(NEXT),
    };
    redirect(&url, &[format!("{STATE}={state}; Path=/belepes; Max-Age=600; HttpOnly; Secure; SameSite=Lax"), next])
}

fn forget(name: &str) -> String {
    format!("{name}=; Path=/belepes; Max-Age=0; HttpOnly; Secure; SameSite=Lax")
}

/// GET /belepes/discord: Discord sends the browser back with a one-time code and the state that
/// /belepes put into a cookie, so no other site can sign someone in with a code of its own.
pub async fn callback(
    State(admin): State<Arc<Admin>>,
    headers: HeaderMap,
    Query(back): Query<Callback>,
) -> Response {
    let forget_state = forget(STATE);
    let state_ok = match (back.state.as_deref(), cookie(&headers, STATE)) {
        (Some(got), Some(sent)) => constant_time::verify_slices_are_equal(got.as_bytes(), sent.as_bytes()).is_ok(),
        _ => false,
    };
    let next = cookie(&headers, NEXT).map(str::to_owned);
    // No code: cancelled on Discord's page.
    let (Some(code), true) = (back.code, state_ok) else {
        let back = next.map_or_else(|| "/".to_owned(), |token| format!("/jelen/{token}"));
        return redirect_safe(&back, &[forget_state, forget(NEXT)]);
    };
    let identified = admin.identify(&code).await;
    if let Some(token) = next {
        let mut response = match identified {
            Ok((me, member)) => attendance::check_in_with(&token, &me, member.as_ref()),
            Err(reason) => {
                log!("attendance: Discord check-in failed: {reason}");
                attendance::problem(&token, "discord")
            }
        };
        for c in [forget_state, forget(NEXT)] {
            response.headers_mut().append(SET_COOKIE, HeaderValue::from_str(&c).expect("ASCII cookie"));
        }
        return response;
    }
    match identified.and_then(|(me, member)| allowed(&me, member.as_ref()).map(|()| me)) {
        Ok(me) => {
            log!("admin: {} ({}) signed in", me.username, me.id);
            let expires = now() + SESSION_FOR.as_secs();
            let session = format!(
                "{SESSION}={}; Path=/; Max-Age={}; HttpOnly; Secure; SameSite=Lax",
                sign(key(), &me.id, expires),
                SESSION_FOR.as_secs(),
            );
            redirect("/admin", &[forget_state, session])
        }
        Err(reason) => {
            log!("admin: sign-in refused: {reason}");
            redirect("/belepes/hiba", &[forget_state])
        }
    }
}

/// POST /kilepes.
pub async fn logout() -> Response {
    redirect("/", &[format!("{SESSION}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax")])
}

/// Middleware: /admin and everything under it only for signed-in admins (others go to
/// /belepes), and never stored in a cache.
pub async fn guard(State(admin): State<Arc<Admin>>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    if path != "/admin" && !path.starts_with("/admin/") {
        return next.run(req).await;
    }
    if !admin.signed_in(req.headers()) {
        return redirect("/belepes", &[]);
    }
    let mut response = next.run(req).await;
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// [`redirect`] to a path made from a cookie: anything but a token's characters goes home.
fn redirect_safe(to: &str, cookies: &[String]) -> Response {
    let safe = to.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'));
    redirect(if safe { to } else { "/" }, cookies)
}

/// A 303 to `to` that sets `cookies` and is never cached. Both are built here from ASCII.
fn redirect(to: &str, cookies: &[String]) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    let headers = response.headers_mut();
    headers.insert(LOCATION, HeaderValue::from_str(to).expect("ASCII location"));
    for c in cookies {
        headers.append(SET_COOKIE, HeaderValue::from_str(c).expect("ASCII cookie"));
    }
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The server's signing key: sessions, and the attendance sheets' QR links
/// (src/attendance.rs). Kept in the state directory that systemd names in STATE_DIRECTORY
/// (private to the service); without one (a local run), a new key every start.
fn key() -> &'static hmac::Key {
    KEY.get_or_init(|| hmac::Key::new(hmac::HMAC_SHA256, &load_key()))
}

/// A short, unguessable tag of `message` (22 characters, URL-safe).
pub fn tag(message: &str) -> String {
    URL_SAFE_NO_PAD.encode(&hmac::sign(key(), message.as_bytes()).as_ref()[..16])
}

fn load_key() -> [u8; 32] {
    let path = std::env::var_os("STATE_DIRECTORY").map(|dir| PathBuf::from(dir).join(KEY_FILE));
    let kept = path.as_ref().and_then(|p| std::fs::read(p).ok()).and_then(|bytes| <[u8; 32]>::try_from(bytes).ok());
    if let Some(key) = kept {
        return key;
    }
    let mut key = [0; 32];
    SystemRandom::new().fill(&mut key).expect("system random generator");
    if let Some(path) = path {
        if let Err(err) = std::fs::write(&path, key) {
            error!("admin: cannot keep the session key in {}: {err}", path.display());
        }
    }
    key
}

fn callback_url() -> String {
    format!("{}{CALLBACK}", site::ORIGIN)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn random_token() -> String {
    let mut bytes = [0; 18];
    SystemRandom::new().fill(&mut bytes).expect("system random generator");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Session cookie value: "<Discord ID>.<expiry, Unix seconds>.<HMAC of both>".
fn sign(key: &hmac::Key, user: &str, expires: u64) -> String {
    let payload = format!("{user}.{expires}");
    let tag = hmac::sign(key, payload.as_bytes());
    format!("{payload}.{}", URL_SAFE_NO_PAD.encode(tag.as_ref()))
}

fn verify(key: &hmac::Key, value: &str, now: u64) -> bool {
    let Some((payload, tag)) = value.rsplit_once('.') else { return false };
    let Some((_, expires)) = payload.rsplit_once('.') else { return false };
    URL_SAFE_NO_PAD.decode(tag).is_ok_and(|tag| hmac::verify(key, payload.as_bytes(), &tag).is_ok())
        && expires.parse::<u64>().is_ok_and(|expires| now < expires)
}

/// The value of cookie `name` (a browser may send them in several Cookie headers).
pub fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|pair| pair.trim().strip_prefix(name)?.strip_prefix('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_are_signed_and_expire() {
        let key = hmac::Key::new(hmac::HMAC_SHA256, b"test key");
        let value = sign(&key, "1234", 1000);
        assert!(verify(&key, &value, 999));
        assert!(!verify(&key, &value, 1000), "expired");
        assert!(!verify(&key, &value.replacen("1234", "1235", 1), 999), "another user");
        assert!(!verify(&key, &value.replacen(".1000.", ".9999.", 1), 999), "extended");
        assert!(!verify(&hmac::Key::new(hmac::HMAC_SHA256, b"other key"), &value, 999), "another key");
        assert!(!verify(&key, "garbage", 0));
        assert!(!verify(&key, "", 0));
    }

    #[test]
    fn cookies_by_exact_name() {
        let mut headers = HeaderMap::new();
        headers.append(COOKIE, HeaderValue::from_static("a=1; kozmosz_admin2=x"));
        headers.append(COOKIE, HeaderValue::from_static("kozmosz_admin=yes"));
        assert_eq!(cookie(&headers, SESSION), Some("yes"));
        assert_eq!(cookie(&headers, "a"), Some("1"));
        assert_eq!(cookie(&headers, STATE), None);
    }

    #[test]
    fn only_the_admin_role_gets_in() {
        let me = Me { id: "1".into(), username: "borisz".into(), global_name: None };
        let with = |roles: &[&str]| Member { roles: roles.iter().map(|r| (*r).to_owned()).collect(), nick: None };
        assert!(allowed(&me, Some(&with(&["9", ADMIN_ROLE]))).is_ok());
        assert!(allowed(&me, Some(&with(&["9"]))).is_err(), "another role");
        assert!(allowed(&me, None).is_err(), "not on the server");
        assert!(with(&["9"]).has_a_role() && !with(&[]).has_a_role());
    }
}
