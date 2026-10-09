//! The photo library: the encoded files in the cache directory, a manifest describing them, and
//! the sync that keeps both in step with the Drive folders.
//!
//! Layout of the cache directory: `files/<album>/<slug>-<hash>-<width>.{avif,jpg}` (served under
//! `/drive/`) and `manifest.json` next to `files/` (not served: it holds Drive IDs and names).

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use leptos::logging::{error, log};
use serde::{Deserialize, Serialize};

use super::{
    auth::ServiceAccount,
    encode::{self, PIPELINE_VERSION},
    publish, slugify,
    source::{self, Access, Listing, Remote},
    Album, Photo, ALBUMS,
};
use crate::site;

/// How often the folders are listed again.
pub const REFRESH_EVERY: Duration = Duration::from_secs(10 * 60);
/// How long the files of replaced or deleted photos stay on disk: a browser may still hold a
/// page that refers to them.
const KEEP_REPLACED: Duration = Duration::from_secs(24 * 60 * 60);
const MANIFEST: &str = "manifest.json";
/// Subdirectory of the cache directory served under `/drive/`.
const FILES: &str = "files";

/// Environment variables (all optional).
const ENV_DIR: &str = "KOZMOSZ_DRIVE_DIR";
const ENV_CREDENTIALS: &str = "KOZMOSZ_DRIVE_CREDENTIALS";
const ENV_API_KEY: &str = "KOZMOSZ_DRIVE_API_KEY";
const ENV_SWITCH: &str = "KOZMOSZ_DRIVE";

#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    albums: HashMap<String, Vec<Photo>>,
}

pub struct Library {
    client: Option<reqwest::Client>,
    access: Access,
    dir: PathBuf,
    enabled: bool,
    albums: HashMap<String, Vec<Photo>>,
    /// Photos replaced or deleted per album, with when: their files are kept for
    /// [`KEEP_REPLACED`].
    replaced: HashMap<String, Vec<(Photo, Instant)>>,
    /// Files that could not be fetched or encoded (ID → version): not tried again until they
    /// change or the server restarts.
    failed: HashMap<String, String>,
    /// Last logged state per album, so an unchanged folder is not logged every 10 minutes.
    status: HashMap<&'static str, String>,
}

impl Library {
    /// Reads the settings from the environment and the manifest from the cache directory, and
    /// publishes the photos already encoded there, so pages have them before the first sync.
    ///
    /// - `KOZMOSZ_DRIVE_DIR`: cache directory (default `drive-cache`).
    /// - `KOZMOSZ_DRIVE_CREDENTIALS`: path of a service-account JSON key (private folders shared
    ///   with that account), or `KOZMOSZ_DRIVE_API_KEY`: a Google API key (public folders).
    ///   Neither: public folders through their embed page.
    /// - `KOZMOSZ_DRIVE=off`: no syncing (the cached photos are still shown).
    pub fn open() -> Self {
        let dir = PathBuf::from(std::env::var(ENV_DIR).unwrap_or_else(|_| "drive-cache".into()));
        let enabled = !std::env::var(ENV_SWITCH).is_ok_and(|v| v.eq_ignore_ascii_case("off"));
        let access = access_from_env();
        let client = reqwest::Client::builder()
            .user_agent(format!("kozmosz-site/{} (+{})", env!("CARGO_PKG_VERSION"), site::ORIGIN))
            .connect_timeout(Duration::from_secs(5))
            .https_only(true)
            .build()
            .map_err(|err| error!("drive: cannot create the HTTP client: {err}"))
            .ok();

        let files = dir.join(FILES);
        let mut albums = read_manifest(&dir);
        // Entries whose files are gone (cache directory cleared by hand) are fetched again.
        for (name, photos) in &mut albums {
            photos.retain(|p| files_of(p).iter().all(|f| files.join(name).join(f).is_file()));
        }
        let library = Self {
            client,
            access,
            dir,
            enabled,
            albums,
            replaced: HashMap::new(),
            failed: HashMap::new(),
            status: HashMap::new(),
        };
        library.publish();
        if library.enabled {
            log!(
                "drive: reading {} folders via {}, cache in {}",
                ALBUMS.len(),
                library.access.describe(),
                library.dir.display()
            );
        }
        library
    }

    /// The directory served under `/drive/`: `<album>/<file>` inside it.
    pub fn files_dir(&self) -> PathBuf {
        self.dir.join(FILES)
    }

    /// Lists every folder, fetches and encodes new or changed photos, forgets deleted ones.
    /// A folder that cannot be read keeps its last photos. Returns whether anything changed.
    pub async fn refresh(&mut self) -> bool {
        let Some(client) = self.client.clone().filter(|_| self.enabled) else { return false };
        let mut changed = false;
        for album in ALBUMS {
            match self.sync(&client, album).await {
                Ok(c) => changed |= c,
                Err(err) => self.report(album, format!("cannot list the folder: {err}"), true),
            }
        }
        if changed {
            self.publish();
        }
        changed
    }

    async fn sync(&mut self, client: &reqwest::Client, album: &'static Album) -> Result<bool, source::Error> {
        let Listing { files: remotes, unsupported } = source::list(client, &mut self.access, album.folder).await?;
        let current = self.albums.get(album.dir).cloned().unwrap_or_default();
        let by_id: HashMap<&str, &Photo> = current.iter().map(|p| (p.id.as_str(), p)).collect();

        // Every listed file seen before keeps its slug, changed or not (stable URLs; the first
        // wins if two ever shared one); only new files take a fresh slug.
        let mut slugs: HashSet<String> = HashSet::new();
        let mut owns_slug: HashSet<&str> = HashSet::new();
        for remote in &remotes {
            if let Some(old) = by_id.get(remote.id.as_str()) {
                if slugs.insert(old.slug.clone()) {
                    owns_slug.insert(old.id.as_str());
                }
            }
        }

        let (mut fetched, mut failed) = (0, Vec::new());
        let mut next = Vec::with_capacity(remotes.len());
        for remote in &remotes {
            let old = by_id.get(remote.id.as_str()).copied();
            let current_encoding = old.is_some_and(|p| p.pipeline == PIPELINE_VERSION);
            if let Some(photo) = old.filter(|p| p.version == remote.version && current_encoding) {
                next.push(Photo {
                    name: remote.name.clone(),
                    modified: remote.modified.clone(),
                    description: remote.description.clone(),
                    ..photo.clone()
                });
                continue;
            }
            if self.failed.get(&remote.id) == Some(&remote.version) {
                failed.push(format!("{}: failed before", remote.name));
                next.extend(old.cloned());
                continue;
            }
            let keep_slug = old.filter(|p| owns_slug.contains(p.id.as_str()));
            match self.fetch(client, album, remote, old, keep_slug, &mut slugs).await {
                Ok(photo) => {
                    fetched += 1;
                    self.failed.remove(&remote.id);
                    next.push(photo);
                }
                Err(err) => {
                    failed.push(format!("{}: {err}", remote.name));
                    self.failed.insert(remote.id.clone(), remote.version.clone());
                    // A changed file that fails keeps its previous version.
                    next.extend(old.cloned());
                }
            }
        }
        let changed = next != current;
        if changed {
            let kept: HashSet<String> = next.iter().flat_map(files_of).collect();
            let now = Instant::now();
            let replaced = self.replaced.entry(album.dir.to_owned()).or_default();
            replaced.extend(
                current.into_iter().filter(|p| !files_of(p).iter().all(|f| kept.contains(f))).map(|p| (p, now)),
            );
            self.albums.insert(album.dir.to_owned(), next);
            // The manifest must be on disk before any file goes.
            self.save();
        }
        self.collect_garbage(album);

        let count = self.albums.get(album.dir).map_or(0, Vec::len);
        let mut status = format!("{count} photos");
        if fetched > 0 {
            status.push_str(&format!(", {fetched} new or changed"));
        }
        if !unsupported.is_empty() {
            status.push_str(&format!(", not JPG/PNG/WebP (ignored): {}", unsupported.join(", ")));
        }
        let has_failures = !failed.is_empty();
        if has_failures {
            status.push_str(&format!(", {} skipped: {}", failed.len(), failed.join("; ")));
        }
        self.report(album, status, has_failures);
        Ok(changed)
    }

    /// Downloads and encodes one file. `keep_slug`: the file's previous version, whose slug it
    /// keeps.
    async fn fetch(
        &mut self,
        client: &reqwest::Client,
        album: &'static Album,
        remote: &Remote,
        old: Option<&Photo>,
        keep_slug: Option<&Photo>,
        slugs: &mut HashSet<String>,
    ) -> Result<Photo, String> {
        let bytes = source::download(client, &mut self.access, &remote.id).await.map_err(|e| e.to_string())?;
        // The hash covers the encoder settings too: a new pipeline gives new file names.
        let hash = encode::content_hash(&bytes);
        let slug = match keep_slug {
            Some(old) => old.slug.clone(),
            None => {
                let slug = unique_slug(&remote.name, &remote.id, slugs);
                slugs.insert(slug.clone());
                slug
            }
        };
        let photo = |width, height, widths| Photo {
            id: remote.id.clone(),
            name: remote.name.clone(),
            version: remote.version.clone(),
            modified: remote.modified.clone(),
            description: remote.description.clone(),
            slug: slug.clone(),
            hash: hash.clone(),
            pipeline: PIPELINE_VERSION,
            width,
            height,
            widths,
        };
        // Same bytes and encoder under a new version marker (e.g. touched in Drive): reuse.
        if let Some(old) = old.filter(|p| p.hash == hash && p.slug == slug && p.pipeline == PIPELINE_VERSION) {
            return Ok(photo(old.width, old.height, old.widths.clone()));
        }
        let stem = format!("{slug}-{hash}");
        let target = self.files_dir().join(album.dir);
        let threads = std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).max(1));
        // Decoding and AVIF encoding take seconds per photo: off the async workers. Files left
        // complete by an interrupted sync (the names carry the content hash) are reused.
        let (width, height, widths) = tokio::task::spawn_blocking(move || -> Result<_, String> {
            if let Ok((width, height, widths)) = encode::probe(&bytes, album.widths) {
                let done = widths.iter().all(|w| {
                    ["avif", "jpg"].iter().all(|ext| target.join(format!("{stem}-{w}.{ext}")).is_file())
                });
                if done {
                    return Ok((width, height, widths));
                }
            }
            let encoded = encode::encode(&bytes, album.widths, threads)?;
            std::fs::create_dir_all(&target).map_err(|e| format!("cannot create {}: {e}", target.display()))?;
            for (suffix, data) in &encoded.files {
                write_atomic(&target.join(format!("{stem}{suffix}")), data)?;
            }
            Ok((encoded.width, encoded.height, encoded.widths))
        })
        .await
        .map_err(|e| format!("encoder crashed: {e}"))??;
        Ok(photo(width, height, widths))
    }

    /// Deletes files in the album directory that no current photo uses and that were not
    /// replaced within [`KEEP_REPLACED`] (and leftovers of interrupted writes).
    fn collect_garbage(&mut self, album: &Album) {
        if let Some(replaced) = self.replaced.get_mut(album.dir) {
            replaced.retain(|(_, since)| since.elapsed() < KEEP_REPLACED);
        }
        let keep: HashSet<String> = self
            .albums
            .get(album.dir)
            .into_iter()
            .flatten()
            .chain(self.replaced.get(album.dir).into_iter().flatten().map(|(p, _)| p))
            .flat_map(files_of)
            .collect();
        let Ok(entries) = std::fs::read_dir(self.files_dir().join(album.dir)) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_file() && !keep.contains(&name) {
                if let Err(err) = std::fs::remove_file(entry.path()) {
                    error!("drive: cannot delete {}: {err}", entry.path().display());
                }
            }
        }
    }

    fn save(&self) {
        let manifest = Manifest { albums: self.albums.clone() };
        let result = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string()).and_then(|json| {
            std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
            write_atomic(&self.dir.join(MANIFEST), &json)
        });
        if let Err(err) = result {
            error!("drive: cannot save the manifest: {err}");
        }
    }

    fn publish(&self) {
        publish(self.albums.iter().map(|(dir, photos)| (dir.clone(), Arc::from(photos.as_slice()))).collect());
    }

    fn report(&mut self, album: &'static Album, status: String, is_error: bool) {
        if self.status.get(album.dir) == Some(&status) {
            return;
        }
        if is_error {
            error!("drive: {}: {status}", album.dir);
        } else {
            log!("drive: {}: {status}", album.dir);
        }
        self.status.insert(album.dir, status);
    }
}

fn access_from_env() -> Access {
    if let Ok(path) = std::env::var(ENV_CREDENTIALS) {
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|json| ServiceAccount::from_json(&json)) {
            Ok(account) => return Access::ServiceAccount(Box::new(account)),
            Err(err) => error!("drive: {ENV_CREDENTIALS}={path}: {err}; falling back to public access"),
        }
    }
    match std::env::var(ENV_API_KEY) {
        Ok(key) if !key.trim().is_empty() => Access::ApiKey(key.trim().to_owned()),
        _ => Access::Public,
    }
}

/// The photos of the last sync. Photos encoded with older settings stay listed (and shown) until
/// the next sync has encoded them again.
fn read_manifest(dir: &Path) -> HashMap<String, Vec<Photo>> {
    let Ok(json) = std::fs::read(dir.join(MANIFEST)) else { return HashMap::new() };
    match serde_json::from_slice::<Manifest>(&json) {
        Ok(manifest) => manifest.albums,
        Err(err) => {
            error!("drive: ignoring an unreadable manifest: {err}");
            HashMap::new()
        }
    }
}

/// The file names of a photo inside its album directory.
fn files_of(photo: &Photo) -> Vec<String> {
    let stem = photo.file_stem();
    photo.widths.iter().flat_map(|w| [format!("{stem}-{w}.avif"), format!("{stem}-{w}.jpg")]).collect()
}

/// A free slug from the file name: taken slugs get "-2", "-3"…; a name without letters or
/// digits falls back to the start of the file ID.
fn unique_slug(name: &str, id: &str, taken: &HashSet<String>) -> String {
    let mut base = slugify(name);
    if base.is_empty() {
        base = id.chars().filter(char::is_ascii_alphanumeric).take(12).collect::<String>().to_ascii_lowercase();
    }
    if base.len() > 80 {
        base.truncate(80);
        while base.ends_with('-') {
            base.pop();
        }
    }
    if !taken.contains(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|s| !taken.contains(s)).unwrap_or(base)
}

/// Writes through a temporary file and a rename, so a crash never leaves half a file behind
/// under the real name.
fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, data).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique() {
        let mut taken = HashSet::new();
        assert_eq!(unique_slug("Orion-köd.jpg", "id1", &taken), "orion-kod");
        taken.insert("orion-kod".to_owned());
        assert_eq!(unique_slug("Orion köd.png", "id2", &taken), "orion-kod-2");
        assert_eq!(unique_slug("___.jpg", "1AbC_dEf-123456789", &taken), "1abcdef12345");
        let photo = Photo {
            id: "id3".into(),
            name: "a.jpg".into(),
            version: "v1".into(),
            modified: String::new(),
            description: String::new(),
            slug: "elso-nev".into(),
            hash: "00000000".into(),
            pipeline: PIPELINE_VERSION,
            width: 1,
            height: 1,
            widths: vec![1],
        };
        assert_eq!(files_of(&photo), ["elso-nev-00000000-1.avif", "elso-nev-00000000-1.jpg"]);
    }

    #[test]
    fn temp_files_do_not_collide() {
        let dir = std::env::temp_dir().join(format!("kozmosz-drive-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_atomic(&dir.join("a-1.avif"), b"x").unwrap();
        write_atomic(&dir.join("a-1.jpg"), b"y").unwrap();
        assert_eq!(std::fs::read(dir.join("a-1.avif")).unwrap(), b"x");
        assert_eq!(std::fs::read(dir.join("a-1.jpg")).unwrap(), b"y");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
