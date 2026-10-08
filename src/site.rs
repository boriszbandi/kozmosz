//! Site-wide facts shown in the header, footer and contact page.

pub const NAME: &str = "Egyetemi Kozmosz Szakkollégium";
pub const SHORT_NAME: &str = "Kozmosz";
/// Public origin, used for canonical URLs and Open Graph images.
pub const ORIGIN: &str = "https://kozmosz.bme.hu";

pub const ADDRESS: &str = "1111 Budapest, Műegyetem rkp. 3. K386.";
pub const EMAIL: &str = "bme.kozmosz@gmail.com";
pub const MAPS_URL: &str =
    "https://www.google.com/maps/search/?api=1&query=Budapest%2C%20M%C5%B1egyetem%20rkp.%203%2C%201111";
pub const FACEBOOK_URL: &str = "https://www.facebook.com/bme.kozmosz";
pub const INSTAGRAM_URL: &str = "https://www.instagram.com/bme.kozmosz/";

/// Public iCal feed of the club's Google Calendar. The server downloads it and lists the events
/// on the programs page (src/calendar.rs).
#[cfg(feature = "ssr")]
pub const CALENDAR_FEED_URL: &str =
    "https://calendar.google.com/calendar/ical/bme.kozmosz%40gmail.com/public/basic.ics";

/// Google Calendar of the club: the embeddable view and two ways to subscribe to the live feed
/// (a plain https .ics link would only download a one-off snapshot).
pub const CALENDAR_EMBED_URL: &str =
    "https://calendar.google.com/calendar/embed?src=bme.kozmosz%40gmail.com&ctz=Europe%2FBudapest";
/// Google Calendar's "add this calendar" flow; cid is the base64 of the calendar id.
pub const CALENDAR_SUBSCRIBE_GOOGLE_URL: &str =
    "https://calendar.google.com/calendar/u/0?cid=Ym1lLmtvem1vc3pAZ21haWwuY29t";
/// webcal:// makes Apple Calendar and Outlook subscribe instead of importing once.
pub const CALENDAR_SUBSCRIBE_WEBCAL_URL: &str =
    "webcal://calendar.google.com/calendar/ical/bme.kozmosz%40gmail.com/public/basic.ics";

/// Main navigation. `children` render as a submenu.
pub struct NavItem {
    pub label: &'static str,
    pub href: &'static str,
    pub children: &'static [(&'static str, &'static str)],
}

pub const NAV: &[NavItem] = &[
    NavItem { label: "Rólunk", href: "/rolunk", children: &[] },
    NavItem { label: "Elérhetőség", href: "/elerhetoseg", children: &[] },
    NavItem { label: "Programjaink", href: "/programjaink", children: &[] },
    NavItem { label: "Előadóink", href: "/eloadoink", children: &[] },
    NavItem {
        label: "Projektek",
        href: "/projektek",
        children: &[
            ("Időjárásműhold", "/projektek/idojaras-muhold"),
            ("ISSTV-felvételek", "/projektek/isstv"),
            ("Asztrofotóink", "/projektek/asztrofotok"),
        ],
    },
];

/// Old WordPress URLs that moved, answered with 301 so links and search results keep working.
#[cfg(feature = "ssr")]
pub const REDIRECTS: &[(&str, &str)] = &[
    ("/notlikeus", "/rolunk"),
    ("/contact", "/elerhetoseg"),
    ("/eloadok", "/eloadoink"),
    ("/archiv", "/eloadoink"),
    ("/idojaras-muhold", "/projektek/idojaras-muhold"),
    ("/isstv-felvetelek", "/projektek/isstv"),
    ("/asztrofotoink", "/projektek/asztrofotok"),
];
