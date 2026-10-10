//! Per-page CSS: the stylesheet inlined into a page's `<style>` is trimmed to the rules that can
//! match that page. Every page used to carry the whole site's CSS (~33 KB), most of it for other
//! pages.
//!
//! The pages have no JavaScript, so the classes and ids in the HTML are all there will ever be. A
//! style rule is kept when every class and id in one of its selectors occurs in the page. That
//! check is conservative: anything inside a functional pseudo-class (`:not()`, `:is()`, `:has()`,
//! `:where()`) is not required, element and attribute parts are assumed to match, and at-rules
//! other than conditional groups (`@font-face`, `@view-transition`, ...) are always kept.
//! `@keyframes` are kept when a kept rule names them.
//!
//! The input is Lightning CSS output (minified, no nesting), but the scanner also copes with
//! whitespace, comments and strings.

use std::collections::HashSet;

/// Returns `html` with its first `<style>` element trimmed to the rules the page can use, or
/// `None` when there is no `<style>` element or nothing to remove.
pub fn purge_inline_css(html: &str) -> Option<String> {
    let open = html.find("<style>")?;
    let start = open + "<style>".len();
    let end = start + html[start..].find("</style>")?;
    let css = &html[start..end];

    // Class and id names of everything outside the stylesheet itself.
    let mut used = Used::default();
    used.collect(&html[..open]);
    used.collect(&html[end..]);

    let purged = purge(css, &used);
    (purged.len() < css.len()).then(|| [&html[..start], purged.as_str(), &html[end..]].concat())
}

/// The class and id names a document uses.
#[derive(Default)]
struct Used<'a> {
    classes: HashSet<&'a str>,
    ids: HashSet<&'a str>,
}

impl<'a> Used<'a> {
    /// Collects the values of every `class="…"` and `id="…"` attribute (the renderer quotes all
    /// attributes with `"`).
    fn collect(&mut self, html: &'a str) {
        for (attr, is_class) in [(" class=\"", true), (" id=\"", false)] {
            let mut rest = html;
            while let Some(at) = rest.find(attr) {
                rest = &rest[at + attr.len()..];
                let Some(close) = rest.find('"') else { break };
                for name in rest[..close].split_ascii_whitespace() {
                    if is_class {
                        self.classes.insert(name);
                    } else {
                        self.ids.insert(name);
                    }
                }
                rest = &rest[close..];
            }
        }
    }
}

/// One item of a rule list: `prelude { body }` (with its whole text) or a statement ending in
/// `;`.
enum Item<'a> {
    Block { prelude: &'a str, body: &'a str, whole: &'a str },
    Statement(&'a str),
}

/// Splits a rule list (a stylesheet or an at-rule's body) into its top-level items.
fn items(css: &str) -> Vec<Item<'_>> {
    let bytes = css.as_bytes();
    let mut out = Vec::new();
    let (mut i, mut start) = (0, 0);
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = skip_string(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_comment(bytes, i),
            b'{' => {
                let close = matching_brace(bytes, i);
                out.push(Item::Block {
                    prelude: &css[start..i],
                    body: &css[i + 1..close.min(bytes.len())],
                    whole: css[start..(close + 1).min(bytes.len())].trim_start(),
                });
                i = close + 1;
                start = i;
            }
            b';' => {
                out.push(Item::Statement(&css[start..=i]));
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    out
}

/// Index just past the string starting at `i` (which holds the quote).
fn skip_string(bytes: &[u8], i: usize) -> usize {
    let quote = bytes[i];
    let mut j = i + 1;
    while j < bytes.len() && bytes[j] != quote {
        j += if bytes[j] == b'\\' { 2 } else { 1 };
    }
    j + 1
}

/// Index just past the comment starting at `i`.
fn skip_comment(bytes: &[u8], i: usize) -> usize {
    let mut j = i + 2;
    while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
        j += 1;
    }
    j + 2
}

/// Index of the `}` matching the `{` at `open` (or the end of the input if unbalanced).
fn matching_brace(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                i = skip_string(bytes, i);
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = skip_comment(bytes, i);
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

fn purge(css: &str, used: &Used) -> String {
    let mut keyframes = Vec::new();
    let mut out = String::with_capacity(css.len());
    purge_list(css, used, &mut out, Some(&mut keyframes));
    // Animations are named in declarations: keep the keyframes something still refers to.
    for (name, rule) in keyframes {
        if mentions(&out, name) {
            out.push_str(rule);
        }
    }
    out
}

/// Appends the kept items of a rule list to `out`. Top-level `@keyframes` go to `keyframes`
/// (name, whole rule) for the reference check; nested ones are kept as they are.
fn purge_list<'a>(css: &'a str, used: &Used, out: &mut String, mut keyframes: Option<&mut Vec<(&'a str, &'a str)>>) {
    for item in items(css) {
        match item {
            Item::Statement(text) => out.push_str(text.trim_start()),
            Item::Block { prelude, body, whole } => {
                let head = prelude.trim();
                if let Some(at) = head.strip_prefix('@') {
                    let name = at.split(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("");
                    match name.to_ascii_lowercase().as_str() {
                        "media" | "supports" | "container" | "layer" | "document" | "scope" | "starting-style" => {
                            let mut inner = String::new();
                            purge_list(body, used, &mut inner, None);
                            if !inner.is_empty() {
                                out.push_str(head);
                                out.push('{');
                                out.push_str(&inner);
                                out.push('}');
                            }
                        }
                        "keyframes" | "-webkit-keyframes" if keyframes.is_some() => {
                            let animation = at[name.len()..].trim().trim_matches(|c| c == '"' || c == '\'');
                            if let Some(list) = keyframes.as_deref_mut() {
                                list.push((animation, whole));
                            }
                        }
                        // @font-face, @view-transition, @property, @page, @counter-style...
                        _ => {
                            out.push_str(head);
                            out.push('{');
                            out.push_str(body);
                            out.push('}');
                        }
                    }
                } else {
                    let kept: Vec<&str> =
                        split_selectors(head).into_iter().map(str::trim).filter(|s| may_match(s, used)).collect();
                    if !kept.is_empty() {
                        out.push_str(&kept.join(","));
                        out.push('{');
                        out.push_str(body);
                        out.push('}');
                    }
                }
            }
        }
    }
}

/// The selectors of a selector list (commas inside parentheses, brackets and strings do not
/// split).
fn split_selectors(list: &str) -> Vec<&str> {
    let bytes = list.as_bytes();
    let mut out = Vec::new();
    let (mut depth, mut start, mut i) = (0usize, 0, 0);
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                i = skip_string(bytes, i);
                continue;
            }
            b'\\' => i += 1,
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                out.push(&list[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&list[start..]);
    out
}

/// Whether the page has every class and id the selector names outside parentheses and
/// attribute brackets.
fn may_match(selector: &str, used: &Used) -> bool {
    let bytes = selector.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                i = skip_string(bytes, i);
                continue;
            }
            b'\\' => i += 2,
            b'(' | b'[' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            c @ (b'.' | b'#') if depth == 0 => {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && is_ident_byte(bytes[end]) {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                let end = end.min(bytes.len());
                let name = &selector[start..end];
                // A name with escapes is compared as written: unlikely to match the HTML,
                // so it is treated as present (keep the rule).
                if !name.is_empty() && !name.contains('\\') {
                    let present = if c == b'.' { used.classes.contains(name) } else { used.ids.contains(name) };
                    if !present {
                        return false;
                    }
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    true
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'\\' || b >= 0x80
}

/// Whether `name` occurs in `css` as a whole identifier.
fn mentions(css: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let bytes = css.as_bytes();
    let mut from = 0;
    while let Some(at) = css[from..].find(name) {
        let start = from + at;
        let end = start + name.len();
        let before = start.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(end).copied();
        let boundary = |b: Option<u8>| b.is_none_or(|b| !(b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        if boundary(before) && boundary(after) {
            return true;
        }
        from = end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(css: &str, body: &str) -> String {
        format!("<!DOCTYPE html><html><head><style>{css}</style></head><body>{body}</body></html>")
    }

    fn purged(css: &str, body: &str) -> String {
        let html = page(css, body);
        let out = purge_inline_css(&html).unwrap_or(html);
        let start = out.find("<style>").unwrap() + 7;
        out[start..out.find("</style>").unwrap()].to_owned()
    }

    #[test]
    fn keeps_only_rules_the_page_uses() {
        let css = ":root{--a:1}body{margin:0}.hero{color:red}.gallery li{gap:0}.hero .lead,.footer{x:1}#tartalom{y:2}#nincs{z:3}";
        let body = r#"<main id="tartalom"><section class="hero big"><p class="lead">x</p></section></main>"#;
        assert_eq!(purged(css, body), ":root{--a:1}body{margin:0}.hero{color:red}.hero .lead{x:1}#tartalom{y:2}");
    }

    #[test]
    fn functional_pseudo_classes_and_attributes_are_not_required() {
        let css = ".a:not(.b){k:1}:is(.x,.y)::before{k:2}body:has(.z) .a{k:3}a[href$=\".c\"]{k:4}.q:hover{k:5}";
        assert_eq!(purged(css, r#"<p class="a">x</p>"#), ".a:not(.b){k:1}:is(.x,.y)::before{k:2}body:has(.z) .a{k:3}a[href$=\".c\"]{k:4}");
    }

    #[test]
    fn conditional_groups_are_trimmed_and_dropped_when_empty() {
        let css = "@media (width<768px){.a{k:1}.b{k:2}}@supports (x:y){.b{k:3}}@font-face{font-family:U}@view-transition{navigation:auto}";
        assert_eq!(purged(css, r#"<i class="a"></i>"#), "@media (width<768px){.a{k:1}}@font-face{font-family:U}@view-transition{navigation:auto}");
    }

    #[test]
    fn keyframes_survive_only_when_named() {
        let css = ".a{animation:spin 1s}.b{animation:pass 4s}@keyframes spin{to{rotate:1turn}}@keyframes pass{from{opacity:0}}@keyframes spinner{to{x:1}}";
        assert_eq!(purged(css, r#"<i class="a"></i>"#), ".a{animation:spin 1s}@keyframes spin{to{rotate:1turn}}");
    }

    #[test]
    fn strings_and_comments_do_not_confuse_the_scanner() {
        let css = ".a::after{content:\"}{,.\"}/* .b{} */.b{k:1}.c{content:'a;b'}";
        assert_eq!(purged(css, r#"<i class="a c"></i>"#), ".a::after{content:\"}{,.\"}.c{content:'a;b'}");
    }

    #[test]
    fn the_stylesheet_itself_does_not_count_as_usage() {
        // A class that only appears in the CSS (here in a string) is still unused.
        assert_eq!(purged(".a{k:1}.b{content:\" class=\\\"a\\\"\"}", "<p>x</p>"), "");
        assert!(purge_inline_css("<p>no style</p>").is_none());
    }

    #[test]
    fn whole_identifier_check() {
        assert!(mentions("animation:pass 4s", "pass"));
        assert!(!mentions("animation:passive 4s", "pass"));
        assert!(!mentions("animation:by-pass 4s", "pass"));
    }
}

/// Size check on a real page: `KOZMOSZ_PURGE_SAMPLE=page.html cargo test --features ssr
/// purge_sample -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore]
fn purge_sample() {
    let path = std::env::var("KOZMOSZ_PURGE_SAMPLE").expect("KOZMOSZ_PURGE_SAMPLE");
    let html = std::fs::read_to_string(path).unwrap();
    let out = purge_inline_css(&html).unwrap();
    let css = |h: &str| h[h.find("<style>").unwrap() + 7..h.find("</style>").unwrap()].len();
    println!("html {} -> {} bytes, css {} -> {} bytes", html.len(), out.len(), css(&html), css(&out));
}
