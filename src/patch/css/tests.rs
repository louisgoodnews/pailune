//! Tests of the CSS patch engine.

use super::apply;

const BASE: &str = "\u{feff}#Title { color: #acacac; font-size: 15px; }\r\n.btn{ width: 10px }\r\n@keyframes spin { 0% { opacity: 0; } 100% { opacity: 1; } }\r\n";

#[test]
fn merges_replaces_adds_and_removes() {
    let patch = "#Title { color: #ff3030; margin: 2px }\n/* @replace */\n.btn { height: 3px }\n/* @add */\n.new { x: 1 }\n/* @remove \".gone\" */";
    let out = apply(BASE, patch);
    assert!(out.starts_with('\u{feff}'), "BOM kept");
    assert!(
        out.contains("#Title { color: #ff3030; font-size: 15px; margin: 2px; }"),
        "{out}"
    );
    assert!(
        out.contains(".btn { height: 3px }") && !out.contains("width: 10px"),
        "{out}"
    );
    assert!(
        out.contains("@keyframes spin { 0% { opacity: 0; } 100% { opacity: 1; } }\r\n"),
        "nested blocks kept"
    );
    assert!(out.ends_with(".new { x: 1 }\r\n"), "{out:?}");
    let removed = apply(BASE, "/* @remove \".btn\" */");
    assert!(!removed.contains(".btn"), "{removed}");
    assert_eq!(apply(BASE, ""), BASE, "an empty patch changes nothing");
}

#[cfg(test)]
mod real {
    /// `PYLUNE_CSS=<extracted titleview.css> cargo test -- --ignored`
    #[test]
    #[ignore = "needs a stylesheet extracted from the game"]
    fn patches_the_real_title_stylesheet() {
        let Ok(path) = std::env::var("PYLUNE_CSS") else {
            return;
        };
        let base = std::fs::read_to_string(path).unwrap();
        let out = super::apply(
            &base,
            "#TitleViewCopyrightWrap { color: #ff3030; font-size: 30px }",
        );
        let changed: Vec<(&str, &str)> = base
            .lines()
            .zip(out.lines())
            .filter(|(a, b)| a != b)
            .collect();
        println!("{changed:#?}");
        assert_eq!(changed.len(), 1);
        assert_eq!(base.lines().count(), out.lines().count());
        assert!(
            changed[0].1.contains("color: #ff3030") && changed[0].1.contains("font-size: 30px")
        );
    }
}
