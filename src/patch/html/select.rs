//! CSS-lite selectors for HTML patches: `tag`, `#id`, `.class`,
//! `[attr]`, `[attr=value]`, combined (`div.panel#main`) and separated by
//! spaces for descendants (`#menu li.active`).

use crate::patch::xml::dom::Element;

/// One compound selector (`div.a#b[x=y]`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Compound {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<(String, Option<String>)>,
}

/// Reads a compound selector.
fn compound(text: &str) -> Compound {
    let mut out = Compound::default();
    let mut rest = text;
    let token_end = |s: &str| s.find(['#', '.', '[']).unwrap_or(s.len());
    let tag_end = token_end(rest);
    if tag_end > 0 && &rest[..tag_end] != "*" {
        out.tag = Some(rest[..tag_end].to_lowercase());
    }
    rest = &rest[tag_end..];
    while let Some(c) = rest.chars().next() {
        match c {
            '#' | '.' => {
                let end = token_end(&rest[1..]) + 1;
                let value = rest[1..end].to_string();
                match c {
                    '#' => out.id = Some(value),
                    _ => out.classes.push(value),
                }
                rest = &rest[end..];
            }
            '[' => {
                let end = rest.find(']').unwrap_or(rest.len());
                let inner = &rest[1..end];
                let (name, value) = match inner.split_once('=') {
                    Some((n, v)) => (n, Some(v.trim().trim_matches(['"', '\'']).to_string())),
                    None => (inner, None),
                };
                out.attrs.push((name.trim().to_string(), value));
                rest = rest.get(end + 1..).unwrap_or("");
            }
            _ => break,
        }
    }
    out
}

/// Whether an element matches a compound selector.
fn matches(element: &Element, c: &Compound) -> bool {
    let attr = |name: &str| {
        element
            .attrs
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
            .map(|a| a.value.as_str())
    };
    let classes: Vec<&str> = attr("class")
        .map(|c| c.split_whitespace().collect())
        .unwrap_or_default();
    c.tag
        .as_ref()
        .is_none_or(|t| element.name.eq_ignore_ascii_case(t))
        && c.id.as_deref().is_none_or(|id| attr("id") == Some(id))
        && c.classes.iter().all(|k| classes.contains(&k.as_str()))
        && c.attrs.iter().all(|(name, value)| match value {
            Some(v) => attr(name) == Some(v.as_str()),
            None => attr(name).is_some(),
        })
}

/// Elements matching the last compound whose ancestors match the earlier
/// ones, in document order.
fn walk<'d>(elements: &'d [Element], chain: &[Compound], out: &mut Vec<&'d Element>) {
    elements.iter().for_each(|e| {
        match chain {
            [last] if matches(e, last) => out.push(e),
            [first, rest @ ..] if !rest.is_empty() && matches(e, first) => {
                walk(&e.children, rest, out)
            }
            _ => {}
        }
        walk(&e.children, chain, out);
    });
}

/// The elements a selector names.
pub fn select<'d>(roots: &'d [Element], selector: &str) -> Vec<&'d Element> {
    let chain: Vec<Compound> = selector.split_whitespace().map(compound).collect();
    let mut found = Vec::new();
    if !chain.is_empty() {
        walk(roots, &chain, &mut found);
    }
    found.sort_by_key(|e| e.start);
    found.dedup_by_key(|e| e.start);
    found
}
