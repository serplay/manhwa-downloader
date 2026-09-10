//! Small helpers on top of `scraper` shared by the HTML-scraping sources.
//!
//! `scraper::Html` is not `Send`, so every source keeps parsing in synchronous
//! `parse_*` functions that take the body as a string. That also makes them
//! trivially testable against saved fixtures.

use scraper::{ElementRef, Selector};

use crate::model::parse_leading_number;

/// Parse a selector known at compile time. Panics on a typo, which the fixture
/// tests catch.
pub fn sel(css: &str) -> Selector {
    Selector::parse(css).unwrap_or_else(|e| panic!("invalid selector `{css}`: {e:?}"))
}

/// Visible text of an element with whitespace collapsed.
pub fn text(el: ElementRef<'_>) -> String {
    collapse_ws(el.text().collect::<Vec<_>>().join(" "))
}

pub fn collapse_ws(s: impl AsRef<str>) -> String {
    s.as_ref().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// First non-empty attribute among `names`, trimmed.
pub fn first_attr(el: ElementRef<'_>, names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|n| el.value().attr(n))
        .map(str::trim)
        .find(|v| !v.is_empty())
        .map(ToString::to_string)
}

/// Image URL of an `<img>`, preferring lazy-load attributes over the (often
/// placeholder) `src`. Falls back to the first candidate of a `srcset`.
pub fn image_src(img: ElementRef<'_>) -> Option<String> {
    first_attr(
        img,
        &[
            "data-src",
            "data-lazy-src",
            "data-original",
            "data-cfsrc",
            "src",
        ],
    )
    .filter(|u| !u.starts_with("data:"))
    .or_else(|| {
        first_attr(img, &["data-srcset", "srcset"]).and_then(|s| {
            s.split(',')
                .next()
                .and_then(|c| c.split_whitespace().next())
                .map(ToString::to_string)
        })
    })
}

/// Resolve a possibly relative URL against a site base.
pub fn absolutize(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Some(rest) = href.strip_prefix("//") {
        format!("https://{rest}")
    } else if href.starts_with('/') {
        format!("{}{href}", base.trim_end_matches('/'))
    } else {
        format!("{}/{href}", base.trim_end_matches('/'))
    }
}

/// Last non-empty path segment of a URL or path, ignoring query and fragment.
pub fn last_path_segment(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// Path segments after `marker` (e.g. `"/manga/"`), without query, fragment
/// or trailing slash. `path_after("https://x/manga/a/b/", "/manga/")` is `a/b`.
pub fn path_after(url: &str, marker: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let idx = path.find(marker)? + marker.len();
    let rest = path[idx..].trim_matches('/');
    (!rest.is_empty()).then(|| rest.to_string())
}

/// Format a chapter or volume number the way the old API did: integers without
/// a decimal point, fractions as written.
pub fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Chapter label to number. Looks for the number after a `Chapter`/`Ch.`
/// marker first (so `Vol.01 Ch.003` gives `3`), then the first number anywhere,
/// and finally falls back to the cleaned label itself for `Extra`, `Oneshot`...
pub fn chapter_number(label: &str) -> String {
    let label = collapse_ws(label);
    let lower = label.to_ascii_lowercase();
    let after_marker = ["chapter", "ch.", "ch ", "episode", "ep."]
        .iter()
        .find_map(|m| lower.find(m).map(|i| i + m.len()))
        .and_then(|i| parse_leading_number(&label[i..]));
    match after_marker.or_else(|| parse_leading_number(&label)) {
        Some(n) => fmt_number(n),
        None => label,
    }
}

/// Keep a label as a chapter title only when it says more than "Chapter N".
pub fn title_if_informative(label: &str, number: &str) -> Option<String> {
    let label = collapse_ws(label);
    let lower = label.to_ascii_lowercase();
    let bare = [
        format!("chapter {number}"),
        format!("ch.{number}"),
        format!("ch. {number}"),
        number.to_string(),
    ];
    if label.is_empty() || bare.contains(&lower) {
        None
    } else {
        Some(label)
    }
}

/// Extract the JSON object that starts at the first `{` at or after `from`,
/// honoring string escapes so braces inside strings do not confuse the scan.
pub fn json_object_at(src: &str, from: usize) -> Option<&str> {
    let start = from + src[from..].find('{')?;
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_numbers() {
        assert_eq!(chapter_number("Chapter 12"), "12");
        assert_eq!(chapter_number("Group 2 Chapter 203"), "203");
        assert_eq!(chapter_number("Vol.01 Ch.003"), "3");
        assert_eq!(chapter_number("Ch.202"), "202");
        assert_eq!(chapter_number("Chapter 7.5 - Title"), "7.5");
        assert_eq!(chapter_number("Volume 6"), "6");
        assert_eq!(chapter_number("  Extra  "), "Extra");
    }

    #[test]
    fn informative_titles() {
        assert_eq!(title_if_informative("Chapter 12", "12"), None);
        assert_eq!(title_if_informative("Ch.12", "12"), None);
        assert_eq!(
            title_if_informative("Chapter 12 - The End", "12").as_deref(),
            Some("Chapter 12 - The End")
        );
    }

    #[test]
    fn paths() {
        assert_eq!(
            last_path_segment("https://x/serie/abc/chapter-3/?x=1").as_deref(),
            Some("chapter-3")
        );
        assert_eq!(
            path_after("https://x/manga/a/b/", "/manga/").as_deref(),
            Some("a/b")
        );
        assert_eq!(absolutize("https://x", "//cdn/a.jpg"), "https://cdn/a.jpg");
        assert_eq!(absolutize("https://x/", "/a"), "https://x/a");
    }

    #[test]
    fn json_extraction() {
        let s = r#"ts_reader.run({"a":"}{","b":{"c":[1,2]}}); more"#;
        assert_eq!(json_object_at(s, 0), Some(r#"{"a":"}{","b":{"c":[1,2]}}"#));
    }

    #[test]
    fn numbers() {
        assert_eq!(fmt_number(12.0), "12");
        assert_eq!(fmt_number(12.5), "12.5");
    }
}
