//! Photo galleries fed from the club's Google Drive folders, like the old site's "Integrate Google
//! Drive" plugin: the club uploads photos to a folder and they appear on the project page.
//!
//! The server lists every [`ALBUMS`] folder at startup and every 10 minutes ([`Library`], server
//! only), downloads new or changed photos, re-encodes them (AVIF + JPEG per width, no metadata, so
//! no GPS) into a cache directory served under `/drive/`, and publishes the result; pages read it
//! with [`photos`]. Browsers never talk to Google. The types compile in both builds; the hydrate
//! build never receives data, so there `photos()` is simply empty.

use std::{
    collections::HashMap,
    sync::{Arc, PoisonError, RwLock},
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
mod auth;
#[cfg(feature = "ssr")]
mod encode;
#[cfg(feature = "ssr")]
mod library;
#[cfg(feature = "ssr")]
mod source;
#[cfg(all(test, feature = "ssr"))]
mod tests;

#[cfg(feature = "ssr")]
pub use library::{Library, REFRESH_EVERY};

/// A Drive folder shown as the gallery of a project page.
pub struct Album {
    /// Content page (`content/oldalak/<page>.md`) whose gallery this is.
    pub page: &'static str,
    /// Directory under `/drive/` (and under the cache directory).
    pub dir: &'static str,
    /// Google Drive folder ID (the last part of the folder's URL).
    pub folder: &'static str,
    /// Widths generated per photo, never wider than the original.
    pub widths: &'static [u32],
}

/// Photos: from phone-sized tiles up to the full-screen photo page.
const PHOTO_WIDTHS: &[u32] = &[480, 800, 1200, 1600, 2048];
/// SSTV frames are 640 px wide.
const SSTV_WIDTHS: &[u32] = &[320, 480, 640];

/// The folders the old site's galleries showed. A folder must be readable by the server: shared
/// as "anyone with the link" (no setup), or shared with the service account (see README).
pub const ALBUMS: &[Album] = &[
    Album {
        page: "projektek-asztrofotok",
        dir: "asztrofotok",
        folder: "1Ye9x7I3ReobGIjRgYxbfMzQoY01bUz86",
        widths: PHOTO_WIDTHS,
    },
    Album {
        page: "projektek-isstv",
        dir: "isstv",
        folder: "1zbg2j2_4B1a2xVOhQSp5fZDTBwbp0elS",
        widths: SSTV_WIDTHS,
    },
    Album {
        page: "projektek-idojaras-muhold",
        dir: "idojaras-muhold",
        folder: "1_D-ujPnjDbNtgyvB_Z2yjJP7R0uPbIO_",
        widths: PHOTO_WIDTHS,
    },
];

/// The album shown on content page `page`, if any.
pub fn album(page: &str) -> Option<&'static Album> {
    ALBUMS.iter().find(|a| a.page == page)
}

/// One photo of an album, already encoded into the cache directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Photo {
    /// Google Drive file ID.
    pub id: String,
    /// File name in Drive.
    pub name: String,
    /// The listing's change marker (checksum or modification time); a new value means the file
    /// is downloaded again.
    pub version: String,
    /// Sort key: modification time (RFC 3339) or date, oldest first like the old site.
    pub modified: String,
    /// The file's Drive description, if the listing provides it ("" otherwise).
    pub description: String,
    /// URL-safe name, unique within the album: /projektek/<project>/kep/<slug>.
    pub slug: String,
    /// Short hash of the source bytes (and the encoder settings): part of every file name, so a
    /// changed photo gets new URLs and the files can be cached as immutable.
    pub hash: String,
    /// Encoder settings version the files were made with; older ones are encoded again (and
    /// shown meanwhile).
    #[serde(default)]
    pub pipeline: u32,
    pub width: u32,
    pub height: u32,
    pub widths: Vec<u32>,
}

impl Photo {
    /// URL prefix of the encoded files: `<base>-<width>.avif` / `.jpg`.
    pub fn base(&self, album: &Album) -> String {
        format!("/drive/{}/{}", album.dir, self.file_stem())
    }

    /// File name prefix inside the album directory.
    pub fn file_stem(&self) -> String {
        format!("{}-{}", self.slug, self.hash)
    }
}

/// Album directory → its photos.
type Albums = HashMap<String, Arc<[Photo]>>;

static CURRENT: RwLock<Option<Arc<Albums>>> = RwLock::new(None);

/// The photos of `album` as of the last sync, in display order (empty before the first sync, if
/// the folder is unreadable, and in the browser).
pub fn photos(album: &Album) -> Arc<[Photo]> {
    CURRENT
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|albums| albums.get(album.dir).cloned())
        .unwrap_or_else(|| Arc::from([]))
}

/// Replaces what [`photos`] returns.
#[cfg(feature = "ssr")]
fn publish(albums: Albums) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(albums));
}

/// URL-safe slug of a file name: Hungarian letters folded to ASCII, lowercase, runs of anything
/// else turned into one '-', extension dropped. "PD120_20241116_173959.png" → "pd120-20241116-173959".
pub fn slugify(name: &str) -> String {
    let stem = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) => stem,
        _ => name,
    };
    let mut slug = String::with_capacity(stem.len());
    for c in stem.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'ő' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ű' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// A readable caption from a file name: extension, "_" and a trailing " (1)" copy marker
/// dropped. "Jupiter_Szaturnusz (1).jpg" → "Jupiter Szaturnusz".
pub fn humanize(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| if stem.is_empty() { name } else { stem });
    let mut stem = stem.trim();
    if let Some(open) = stem.rfind(" (") {
        let inner = &stem[open + 2..];
        if inner.ends_with(')') && inner[..inner.len() - 1].chars().all(|c| c.is_ascii_digit()) {
            stem = stem[..open].trim_end();
        }
    }
    stem.replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}
