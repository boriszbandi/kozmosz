//! Project galleries. The photos come from the project's Drive album once it has been synced;
//! until then (or if the folder cannot be read) from the images listed in the page's Markdown.
//!
//! The Markdown alt texts are also used for Drive photos with the same file name
//! ("20241113_202308.png" and `kep:sstv/20241113-202308` match), so a hand-written description
//! survives the move to Drive. Priority: the Drive description, the Markdown alt, then text made
//! from the file name.

use crate::{
    content::doc,
    drive,
    media::{Caption, Img, Tile},
};

/// One photo of a project gallery.
#[derive(Clone, Debug)]
pub struct Shot {
    /// Unique within the gallery: /projektek/<project>/kep/<slug>.
    pub slug: String,
    pub img: Img,
    pub alt: String,
    pub caption: Option<Caption>,
    /// Short name for the photo page title.
    pub title: String,
}

/// The gallery of content page `page`, in display order.
pub fn shots(page: &str) -> Vec<Shot> {
    let doc = doc(page);
    let curated: Vec<_> = doc.images().collect();
    let curated_alt = |slug: &str| curated.iter().find(|i| last_segment(&i.key) == slug).map(|i| i.alt.clone());

    if let Some(album) = drive::album(page) {
        let photos = drive::photos(album);
        if !photos.is_empty() {
            return photos
                .iter()
                .map(|p| {
                    let given = Some(p.description.clone()).filter(|d| !d.is_empty()).or_else(|| curated_alt(&p.slug));
                    let img = Img { base: p.base(album), width: p.width, height: p.height, widths: p.widths.clone() };
                    shot(page, p.slug.clone(), img, &p.name, given)
                })
                .collect();
        }
    }
    curated
        .iter()
        .filter_map(|i| {
            let slug = last_segment(&i.key).to_owned();
            Some(shot(page, slug.clone(), Img::by_key(&i.key)?, &slug, Some(i.alt.clone())))
        })
        .collect()
}

/// Tiles of the gallery, each linking to its photo page.
pub fn tiles(page: &str) -> Vec<Tile> {
    let route = &doc(page).route;
    shots(page)
        .into_iter()
        .map(|s| Tile {
            href: photo_href(route, &s.slug),
            id: Some(tile_id(&s.slug)),
            img: s.img,
            alt: s.alt,
            caption: s.caption,
        })
        .collect()
}

pub fn photo_href(route: &str, slug: &str) -> String {
    format!("{route}/kep/{slug}")
}

/// `id` of a photo's tile on the project page: the photo page's back link returns there.
pub fn tile_id(slug: &str) -> String {
    format!("kep-{slug}")
}

/// Radio images are captioned with their reception time (from the file name); photos with their
/// description.
fn shot(page: &str, slug: String, img: Img, name: &str, given: Option<String>) -> Shot {
    let time = received_at(name);
    match page {
        "projektek-isstv" | "projektek-idojaras-muhold" => {
            let kind = if page == "projektek-isstv" { "SSTV-kép az ISS-ről" } else { "Időjárásműhold-felvétel" };
            let alt = given.unwrap_or_else(|| match &time {
                Some((_, label)) => format!("{kind}, vétel: {label}"),
                None => kind.to_owned(),
            });
            let title = time.as_ref().map_or_else(|| kind.to_owned(), |(_, label)| format!("Vétel: {label}"));
            Shot { slug, img, alt, caption: time.map(|(iso, label)| Caption::Time(iso, label)), title }
        }
        _ => {
            let text = given.unwrap_or_else(|| drive::humanize(name));
            Shot { slug, img, alt: text.clone(), caption: Some(Caption::Text(text.clone())), title: first_clause(&text) }
        }
    }
}

fn last_segment(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

/// Up to the first comma: "Az Orion-köd (M42), Nagy Botond Sebestyén felvétele" → "Az Orion-köd (M42)".
fn first_clause(text: &str) -> String {
    text.split(", ").next().unwrap_or(text).trim().to_owned()
}

/// Reception time encoded in a file name, as (ISO datetime, label): YYYYMMDD HHMMSS
/// ("20241113_202308", "PD120_20241116_173959") or YYYY.MM.DD HH.MM ("2023.05.15_08.12_color").
pub fn received_at(name: &str) -> Option<(String, String)> {
    // A copy marker ("20241113_202308 (1).png") is not part of the time.
    let name = drive::humanize(name);
    let digits: String = name.chars().filter(char::is_ascii_digit).collect();
    let valid_date = |d: &str| {
        (1..=12).contains(&d[4..6].parse::<u8>().unwrap_or(0)) && (1..=31).contains(&d[6..8].parse::<u8>().unwrap_or(0))
    };
    let valid_time = |t: &str| t[..2].parse::<u8>().is_ok_and(|h| h < 24) && t[2..4].parse::<u8>().is_ok_and(|m| m < 60);
    if digits.len() >= 14 {
        let tail = &digits[digits.len() - 14..];
        let (d, t) = tail.split_at(8);
        if valid_date(d) && valid_time(t) && t[4..].parse::<u8>().is_ok_and(|s| s < 60) {
            let (y, mo, da, h, mi, se) = (&d[..4], &d[4..6], &d[6..], &t[..2], &t[2..4], &t[4..]);
            return Some((format!("{y}-{mo}-{da}T{h}:{mi}:{se}"), format!("{y}.{mo}.{da}. {h}:{mi}:{se}")));
        }
    }
    if digits.len() == 12 {
        let (d, t) = digits.split_at(8);
        if valid_date(d) && valid_time(t) {
            let (y, mo, da, h, mi) = (&d[..4], &d[4..6], &d[6..], &t[..2], &t[2..]);
            return Some((format!("{y}-{mo}-{da}T{h}:{mi}"), format!("{y}.{mo}.{da}. {h}:{mi}")));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reception_times_from_file_names() {
        assert_eq!(received_at("20241113-202308").unwrap().1, "2024.11.13. 20:23:08");
        assert_eq!(received_at("20241113_202308.png").unwrap().0, "2024-11-13T20:23:08");
        assert_eq!(received_at("pd120-20241116-174408").unwrap().0, "2024-11-16T17:44:08");
        assert_eq!(received_at("2023.05.15_08.12_color.png").unwrap(), ("2023-05-15T08:12".into(), "2023.05.15. 08:12".into()));
        assert_eq!(received_at("img-3217"), None);
        assert_eq!(received_at("20241113_202308 (1).png").unwrap().0, "2024-11-13T20:23:08");
        assert_eq!(received_at("20241399_202308"), None);
        assert_eq!(received_at("20241113_256308"), None);
    }

    #[test]
    fn markdown_galleries_until_drive_is_synced() {
        // No sync has run in tests: every project shows its Markdown images.
        for &page in crate::content::PROJECTS {
            let shots = shots(page);
            let images = doc(page).images().count();
            assert_eq!(shots.len(), images, "{page}");
            for s in &shots {
                assert!(!s.alt.is_empty() && !s.title.is_empty() && !s.slug.is_empty(), "{page}: {}", s.slug);
            }
        }
        let sstv = shots("projektek-isstv");
        assert_eq!(sstv[2].slug, "20241113-202308");
        assert!(matches!(&sstv[2].caption, Some(Caption::Time(_, label)) if label == "2024.11.13. 20:23:08"));
        assert_eq!(sstv[2].title, "Vétel: 2024.11.13. 20:23:08");
        let astro = shots("projektek-asztrofotok");
        assert_eq!(astro[1].title, "Az Orion-köd (M42)");
        assert_eq!(tiles("projektek-isstv")[0].href, "/projektek/isstv/kep/20241012-125306");
    }
}
