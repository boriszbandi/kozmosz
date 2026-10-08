//! Turns calendar text fields into short plain text that is safe to show publicly.

use crate::site;

/// Google appends the join details of a Meet call (link, dial-in number, PIN) to the description
/// of events that have one, after a separator or straight after the organiser's text. They are
/// meant for invitees, not for a public page: everything from the first of these is dropped.
const CONFERENCE_STARTS: &[&str] = &[
    "-::~:~::~:",
    "Csatlakozás a Google Meet szolgáltatással",
    "Join with Google Meet",
    "Join Google Meet",
];

/// Plain text of a SUMMARY, LOCATION or DESCRIPTION: Google Meet details dropped, HTML tags
/// removed and entities decoded, e-mail addresses removed (except the club's own), whitespace
/// collapsed, and at most `max_chars` characters, cut at a word boundary with "…". None when
/// nothing is left.
pub fn plain(raw: &str, max_chars: usize) -> Option<String> {
    let raw = CONFERENCE_STARTS.iter().filter_map(|start| raw.find(start)).min().map_or(raw, |at| &raw[..at]);
    let text = decode_entities(&strip_tags(raw));
    let text = remove_emails(&text);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = truncate(&text, max_chars);
    (!text.is_empty()).then_some(text)
}

/// Removes HTML tags. Block-level tags become a space so words do not run together; inline
/// tags vanish. A '<' that does not start a tag ("1 < 2") is kept.
fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let tag = &rest[at + 1..];
        let starts_tag = tag.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!');
        match tag.find('>') {
            Some(end) if starts_tag => {
                let name: String = tag[..end]
                    .trim_start_matches(['/', '!'])
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_ascii_lowercase();
                if is_block(&name) {
                    out.push(' ');
                }
                rest = &tag[end + 1..];
            }
            _ => {
                out.push('<');
                rest = tag;
            }
        }
    }
    out.push_str(rest);
    out
}

fn is_block(tag: &str) -> bool {
    matches!(
        tag,
        "br" | "p" | "div" | "li" | "ul" | "ol" | "tr" | "td" | "th" | "table" | "blockquote" | "hr"
            | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre" | "section" | "article"
            // Google wraps rich descriptions in <html-blob>; the name is read up to the '-'.
            | "html"
    )
}

/// Decodes the entities Google Calendar writes, plus numeric ones.
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let decoded = after.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &after[..end];
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => ' ',
                _ => {
                    let number = name.strip_prefix('#')?;
                    let code = match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => number.parse().ok()?,
                    };
                    char::from_u32(code).filter(|c| !c.is_control() || c.is_whitespace())?
                }
            };
            Some((c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Removes e-mail addresses (with a "mailto:" in front), keeping the club's public address.
fn remove_emails(text: &str) -> String {
    let local = |c: char| c.is_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-');
    let domain = |c: char| c.is_alphanumeric() || matches!(c, '.' | '-');
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('@') {
        let before = &rest[..at];
        let after = &rest[at + 1..];
        let start = before.rfind(|c: char| !local(c)).map_or(0, |i| i + before[i..].chars().next().map_or(1, char::len_utf8));
        let end = after.find(|c: char| !domain(c)).unwrap_or(after.len());
        let host = after[..end].trim_end_matches(['.', '-']);
        let is_email = start < at
            && host.rsplit_once('.').is_some_and(|(name, tld)| {
                !name.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
            });
        if !is_email {
            out.push_str(&rest[..at + 1]);
            rest = after;
            continue;
        }
        let address = &rest[start..at + 1 + host.len()];
        let mut kept = &rest[..start];
        if address.eq_ignore_ascii_case(site::EMAIL) {
            out.push_str(kept);
            out.push_str(address);
        } else {
            // `get`, not slicing: the 7th byte from the end may fall inside an accented letter.
            let prefix = kept.len().checked_sub("mailto:".len());
            if let Some(at) = prefix.filter(|&at| kept.get(at..).is_some_and(|t| t.eq_ignore_ascii_case("mailto:"))) {
                kept = &kept[..at];
            }
            out.push_str(kept);
        }
        rest = &rest[at + 1 + host.len()..];
    }
    out.push_str(rest);
    out
}

/// At most `max_chars` characters; longer text is cut at a word boundary and ends in "…".
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let cut = text.char_indices().nth(max_chars.saturating_sub(1)).map_or(text.len(), |(i, _)| i);
    let head = &text[..cut];
    // Prefer the last space, unless that would throw away more than half of the text.
    let head = match head.rfind(' ') {
        Some(space) if space >= cut / 2 => &head[..space],
        _ => head,
    };
    let head = head.trim_end_matches(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ':' | '-' | '('));
    format!("{head}…")
}
