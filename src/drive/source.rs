//! Listing a Drive folder and downloading its files, with or without credentials.

use std::time::Duration;

use reqwest::{header, Client, StatusCode, Url};
use serde::Deserialize;

use super::auth::ServiceAccount;

/// A downloaded file larger than this is refused (phone photos are 3 to 15 MB).
const MAX_BYTES: usize = 64 * 1024 * 1024;
const LIST_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// How the server reads the folders.
pub enum Access {
    /// No credentials: works for folders shared as "anyone with the link". Uses the folder's
    /// public embed page, which gives names and dates but no descriptions.
    Public,
    /// Drive API with an API key: the same public folders, through the documented API.
    ApiKey(String),
    /// Drive API as a service account: private folders shared with its e-mail address.
    ServiceAccount(Box<ServiceAccount>),
}

impl Access {
    pub fn describe(&self) -> String {
        match self {
            Self::Public => "public folder pages, no credentials".into(),
            Self::ApiKey(_) => "Drive API with an API key".into(),
            Self::ServiceAccount(sa) => format!("Drive API as service account {}", sa.email()),
        }
    }
}

/// A folder's image files, oldest first, and the names of files in formats the encoder cannot
/// read (HEIC and the like), which are left out.
pub struct Listing {
    pub files: Vec<Remote>,
    pub unsupported: Vec<String>,
}

/// One image file in a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remote {
    pub id: String,
    pub name: String,
    /// Changes when the file changes.
    pub version: String,
    /// Sort key, oldest first.
    pub modified: String,
    pub description: String,
}

#[derive(Debug)]
pub enum Error {
    /// The folder or file is not readable with this access (not shared, wrong ID).
    Denied(StatusCode),
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied(status) => write!(f, "access denied ({status}): is the folder shared with the server?"),
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        // Without the URL: in API-key mode it carries the key, and errors are logged.
        Self::Other(err.without_url().to_string())
    }
}

/// Image files of `folder` (no subfolders).
pub async fn list(client: &Client, access: &mut Access, folder: &str) -> Result<Listing, Error> {
    let all = match access {
        Access::Public => list_public(client, folder).await?,
        _ => list_api(client, access, folder).await?,
    };
    let (mut files, other): (Vec<Remote>, Vec<Remote>) = all.into_iter().partition(|f| is_image_name(&f.name));
    files.sort_by(|a, b| a.modified.cmp(&b.modified).then_with(|| a.name.cmp(&b.name)));
    Ok(Listing { files, unsupported: other.into_iter().map(|f| f.name).collect() })
}

/// The file's bytes.
pub async fn download(client: &Client, access: &mut Access, id: &str) -> Result<Vec<u8>, Error> {
    let request = match access {
        Access::Public => {
            let url = Url::parse_with_params(
                "https://drive.usercontent.google.com/download",
                [("id", id), ("export", "download")],
            )
            .map_err(|e| Error::Other(e.to_string()))?;
            client.get(url)
        }
        _ => {
            let url = format!("https://www.googleapis.com/drive/v3/files/{}", encode_id(id)?);
            authorize(client, access, client.get(url).query(&[("alt", "media"), ("supportsAllDrives", "true")])).await?
        }
    };
    let mut response = check(request.timeout(DOWNLOAD_TIMEOUT).send().await?)?;
    if response.content_length().is_some_and(|len| len > MAX_BYTES as u64) {
        return Err(Error::Other(format!("file {id} is larger than {MAX_BYTES} bytes")));
    }
    // The public download endpoint answers with an HTML warning page instead of the file when
    // it cannot serve it directly.
    let is_html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/html"));
    if is_html {
        return Err(Error::Other(format!("file {id}: Drive answered with a web page instead of the file")));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err(Error::Other(format!("file {id} is larger than {MAX_BYTES} bytes")));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn check(response: reqwest::Response) -> Result<reqwest::Response, Error> {
    match response.status() {
        s if s.is_success() => Ok(response),
        s @ (StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND) => Err(Error::Denied(s)),
        s => Err(Error::Other(format!("Drive answered {s}"))),
    }
}

/// Drive IDs are URL-safe; anything else is refused rather than put into a URL path.
fn encode_id(id: &str) -> Result<&str, Error> {
    let ok = !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then_some(id).ok_or_else(|| Error::Other(format!("unexpected file ID {id:?}")))
}

async fn authorize(
    client: &Client,
    access: &mut Access,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, Error> {
    Ok(match access {
        Access::Public => request,
        Access::ApiKey(key) => request.query(&[("key", key.as_str())]),
        Access::ServiceAccount(sa) => {
            let token = sa.token(client).await.map_err(Error::Other)?;
            request.bearer_auth(token)
        }
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    #[serde(default)]
    files: Vec<ApiFile>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiFile {
    id: String,
    name: String,
    #[serde(default)]
    mime_type: String,
    md5_checksum: Option<String>,
    #[serde(default)]
    modified_time: String,
    #[serde(default)]
    description: String,
}

async fn list_api(client: &Client, access: &mut Access, folder: &str) -> Result<Vec<Remote>, Error> {
    // A folder the caller cannot read lists as empty rather than failing: check it first, so an
    // unshared folder never empties its gallery.
    let url = format!("https://www.googleapis.com/drive/v3/files/{}", encode_id(folder)?);
    let request = client.get(url).query(&[("fields", "id,mimeType"), ("supportsAllDrives", "true")]);
    let request = authorize(client, access, request).await?;
    check(request.timeout(LIST_TIMEOUT).send().await?)?;

    let query = format!("'{}' in parents and trashed = false and mimeType contains 'image/'", encode_id(folder)?);
    let mut files = Vec::new();
    let mut page_token: Option<String> = None;
    // A folder of more than 10 000 photos is not expected; the cap keeps a broken API from
    // looping forever.
    for _ in 0..10 {
        let mut request = client.get("https://www.googleapis.com/drive/v3/files").query(&[
            ("q", query.as_str()),
            ("fields", "nextPageToken,files(id,name,mimeType,md5Checksum,modifiedTime,description)"),
            ("pageSize", "1000"),
            ("supportsAllDrives", "true"),
            ("includeItemsFromAllDrives", "true"),
        ]);
        if let Some(token) = &page_token {
            request = request.query(&[("pageToken", token.as_str())]);
        }
        let request = authorize(client, access, request).await?;
        let page: FileList = check(request.timeout(LIST_TIMEOUT).send().await?)?.json().await?;
        files.extend(page.files.into_iter().filter(|f| f.mime_type.starts_with("image/")).map(|f| Remote {
            version: f.md5_checksum.unwrap_or_else(|| f.modified_time.clone()),
            modified: f.modified_time,
            description: f.description.trim().to_owned(),
            id: f.id,
            name: f.name,
        }));
        page_token = page.next_page_token;
        if page_token.is_none() {
            return Ok(files);
        }
    }
    Ok(files)
}

async fn list_public(client: &Client, folder: &str) -> Result<Vec<Remote>, Error> {
    let url = Url::parse_with_params("https://drive.google.com/embeddedfolderview", [("id", encode_id(folder)?)])
        .map_err(|e| Error::Other(e.to_string()))?;
    // English dates (m/d/yy) whatever the server's location.
    let response = client.get(url).header(header::ACCEPT_LANGUAGE, "en-US").timeout(LIST_TIMEOUT).send().await?;
    let html = check(response)?.text().await?;
    // An empty folder still has the list container; without it the page format has changed,
    // and an empty result would wrongly empty the gallery.
    if !html.contains("flip-entries") {
        return Err(Error::Other("unexpected folder page (has Google changed its format?)".into()));
    }
    Ok(parse_embedded_folder(&html))
}

/// Files of the public embed page of a folder (`embeddedfolderview`). Each entry looks like
/// `<div class="flip-entry" id="entry-<ID>">…<a href="…/file/d/<ID>/view…">…
/// <div class="flip-entry-title">NAME</div>…<div class="flip-entry-last-modified"><div>4/15/25</div>`.
/// Subfolders link to `/drive/folders/` and are skipped.
pub fn parse_embedded_folder(html: &str) -> Vec<Remote> {
    let mut files = Vec::new();
    for entry in html.split("class=\"flip-entry\"").skip(1) {
        let Some(id) = between(entry, "id=\"entry-", "\"") else { continue };
        let is_file = between(entry, "<a href=\"", "\"").is_some_and(|href| href.contains("/file/d/"));
        let Some(name) = between(entry, "class=\"flip-entry-title\">", "<") else { continue };
        if !is_file || encode_id(id).is_err() {
            continue;
        }
        let date = between(entry, "class=\"flip-entry-last-modified\"><div>", "<").unwrap_or_default();
        files.push(Remote {
            id: id.to_owned(),
            name: unescape(name),
            version: date.to_owned(),
            modified: sortable_date(date),
            description: String::new(),
        });
    }
    files
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = s.find(start)? + start.len();
    let len = s[from..].find(end)?;
    Some(&s[from..from + len])
}

/// Decodes the entities the embed page uses in names: the five XML ones and numeric ones.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest.find(';').filter(|&semi| semi <= 10).and_then(|semi| {
            let entity = &rest[1..semi];
            let c = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => {
                    let code = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        entity.strip_prefix('#').and_then(|dec| dec.parse().ok())
                    };
                    code.and_then(char::from_u32)
                }
            };
            c.map(|c| (c, semi))
        });
        match decoded {
            Some((c, semi)) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// "4/15/25" → "2025-04-15". Today's files show a time instead ("3:41 PM"): they sort last.
fn sortable_date(date: &str) -> String {
    let parts: Vec<u32> = date.split('/').filter_map(|p| p.trim().parse().ok()).collect();
    match parts[..] {
        [m, d, y] if (1..=12).contains(&m) && (1..=31).contains(&d) => {
            let y = if y < 100 { 2000 + y } else { y };
            format!("{y:04}-{m:02}-{d:02}")
        }
        _ => "9999".into(),
    }
}

/// Formats the encoder can read. HEIC (iPhone default) is not among them: such files are left
/// out and named in the sync's log line.
fn is_image_name(name: &str) -> bool {
    let ext = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_public_folder_page() {
        let html = r#"<div class="flip-entries"><div class="flip-entry" id="entry-13BzKFX3pu1vPJzauT1IKFR332RPv8Qvd" tabindex="0" role="link"><div class="flip-entry-info"><a href="https://drive.google.com/file/d/13BzKFX3pu1vPJzauT1IKFR332RPv8Qvd/view?usp=drive_web" target="_blank"><div class="flip-entry-visual"></div><div class="flip-entry-title">20241012_125306.png</div></a></div><div class="flip-entry-last-modified"><div>4/15/25</div></div></div><div class="flip-entry" id="entry-1abcFOLDER" tabindex="0" role="link"><div class="flip-entry-info"><a href="https://drive.google.com/drive/folders/1abcFOLDER" target="_blank"><div class="flip-entry-title">Régi</div></a></div><div class="flip-entry-last-modified"><div>1/2/24</div></div></div><div class="flip-entry" id="entry-1_Kc-x" tabindex="0" role="link"><div class="flip-entry-info"><a href="https://drive.google.com/file/d/1_Kc-x/view?usp=drive_web" target="_blank"><div class="flip-entry-title">Orion-k&#246;d &amp; M43.jpg</div></a></div><div class="flip-entry-last-modified"><div>3:41 PM</div></div></div></div>"#;
        let files = parse_embedded_folder(html);
        assert_eq!(files.len(), 2, "the subfolder is skipped");
        assert_eq!(files[0].id, "13BzKFX3pu1vPJzauT1IKFR332RPv8Qvd");
        assert_eq!(files[0].name, "20241012_125306.png");
        assert_eq!(files[0].modified, "2025-04-15");
        assert_eq!(files[1].id, "1_Kc-x");
        assert_eq!(files[1].modified, "9999");
        assert_eq!(files[1].name, "Orion-köd & M43.jpg");
        assert_eq!(unescape("a &amp;b &#x151; &#39;c&#39; & d &bogus;"), "a &b ő 'c' & d &bogus;");
    }

    #[test]
    fn only_readable_formats_and_safe_ids() {
        assert!(is_image_name("IMG_1.JPG") && is_image_name("a.webp") && is_image_name("b.png"));
        assert!(!is_image_name("IMG_2.HEIC") && !is_image_name("notes.txt") && !is_image_name("raw"));
        assert!(encode_id("1zbg2j2_4B1a2xVOhQSp5fZDTBwbp0elS").is_ok());
        assert!(encode_id("../x").is_err() && encode_id("").is_err() && encode_id("a b").is_err());
    }
}
