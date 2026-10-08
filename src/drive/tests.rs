use super::*;

#[test]
fn slugs_from_drive_file_names() {
    assert_eq!(slugify("PD120_20241116_173959.png"), "pd120-20241116-173959");
    assert_eq!(slugify("20241113_202308.png"), "20241113-202308");
    assert_eq!(slugify("IMG_3217.png"), "img-3217");
    assert_eq!(slugify("Orion-köd.jpg"), "orion-kod");
    assert_eq!(slugify("Jupiter Szaturnusz 12. 23..jpg"), "jupiter-szaturnusz-12-23");
    assert_eq!(slugify("Hold1 (1).jpg"), "hold1-1");
    assert_eq!(slugify("Űrállomás ÉJJEL"), "urallomas-ejjel");
    assert_eq!(slugify("2023.05.15_08.12_color.png"), "2023-05-15-08-12-color");
    assert_eq!(slugify(".jpg"), "jpg");
    assert_eq!(slugify("___"), "");
}

#[test]
fn captions_from_drive_file_names() {
    assert_eq!(humanize("Vénusz.jpg"), "Vénusz");
    assert_eq!(humanize("Jupiter_Szaturnusz (1).jpg"), "Jupiter Szaturnusz");
    assert_eq!(humanize("Hold1 (1).jpg"), "Hold1");
    assert_eq!(humanize("Jupiter Szaturnusz 12. 23..jpg"), "Jupiter Szaturnusz 12. 23.");
    assert_eq!(humanize("M42 (Orion)"), "M42 (Orion)");
}

#[test]
fn albums_point_at_project_pages() {
    for album in ALBUMS {
        assert!(crate::content::PROJECTS.contains(&album.page), "{} is not a project page", album.page);
        assert_eq!(album.page, format!("projektek-{}", album.dir));
        assert!(album.widths.windows(2).all(|w| w[0] < w[1]));
    }
    assert!(album("projektek-isstv").is_some());
    assert!(album("rolunk").is_none());
}
