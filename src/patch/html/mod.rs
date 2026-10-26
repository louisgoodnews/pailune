//! HTML patch files (`*.html.patch`), §10.5 of `docs/paz-format.md`. Like
//! XML patches, edits are spliced into the original text.
//!
//! Two notations, usable together:
//! * elements in an `<html-patch>` root: `<set at="#btn" class="+a -b" title="x"/>`,
//!   `<remove at>`, `<replace at>…`, `<inner at>…`, `<append at>…`,
//!   `<prepend at>…`, `<before at>…`, `<after at>…`;
//! * comment directives: `<!-- @replace selector="…" -->…<!-- @end -->`,
//!   `<!-- @set-attr selector="…" name="…" value="…" -->`,
//!   `<!-- @add-class selector="…" class="…" -->`.

pub mod select;

#[cfg(test)]
mod tests;

use crate::patch::xml::Outcome;
use crate::patch::xml::dom::{self, Element};
use crate::patch::xml::edit::{Action, edit, splice};

/// One operation: a selector and what to do there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Op {
    selector: String,
    action: HtmlAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HtmlAction {
    Plain(Action),
    /// `+add -remove` class tokens.
    Classes(String),
}

/// `key="value"` pairs of a comment directive.
fn directive_attrs(text: &str) -> Vec<(String, String)> {
    let roots = dom::parse(&format!("<d {text}/>"));
    roots.first().map_or_else(Vec::new, |d| {
        d.attrs
            .iter()
            .map(|a| (a.name.clone(), a.value.clone()))
            .collect()
    })
}

/// Operations written as comment directives.
fn comment_ops(patch: &str) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut rest = patch;
    while let Some(start) = rest.find("<!--") {
        let end = rest[start..].find("-->").map_or(rest.len(), |e| start + e);
        let body = rest[start + 4..end].trim();
        rest = rest.get(end + 3..).unwrap_or("");
        let Some((name, args)) = body
            .strip_prefix('@')
            .map(|b| b.split_once(char::is_whitespace).unwrap_or((b, "")))
        else {
            continue;
        };
        let attrs = directive_attrs(args);
        let get = |k: &str| {
            attrs
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let selector = get("selector");
        let action = match name {
            "replace" => {
                let close = rest
                    .find("<!-- @end -->")
                    .or_else(|| rest.find("<!--@end-->"))
                    .unwrap_or(rest.len());
                let content = rest[..close].trim().to_string();
                rest = rest.get(close..).unwrap_or("");
                HtmlAction::Plain(Action::Replace(content))
            }
            "set-attr" => HtmlAction::Plain(Action::SetAttr(get("name"), get("value"))),
            "add-class" => HtmlAction::Classes(
                get("class")
                    .split_whitespace()
                    .map(|c| format!("+{c}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => continue,
        };
        ops.push(Op { selector, action });
    }
    ops
}

/// Operations written as `<html-patch>` elements.
fn element_ops(patch: &str) -> Vec<Op> {
    let roots = dom::parse(patch);
    let ops = roots
        .iter()
        .filter(|e| e.name == "html-patch")
        .flat_map(|root| root.children.iter());
    ops.flat_map(|op: &Element| {
        let selector = op
            .attr("at")
            .or(op.attr("selector"))
            .unwrap_or_default()
            .to_string();
        let inner = op.inner(patch).trim().to_string();
        let actions: Vec<HtmlAction> = match op.name.as_str() {
            "set" => op
                .attrs
                .iter()
                .filter(|a| a.name != "at" && a.name != "selector")
                .map(|a| match a.name.as_str() {
                    "class"
                        if a.value
                            .split_whitespace()
                            .all(|t| t.starts_with(['+', '-'])) =>
                    {
                        HtmlAction::Classes(a.value.clone())
                    }
                    _ => HtmlAction::Plain(Action::SetAttr(a.name.clone(), a.value.clone())),
                })
                .collect(),
            "remove" => vec![HtmlAction::Plain(Action::Remove)],
            "replace" => vec![HtmlAction::Plain(Action::Replace(inner))],
            "inner" => vec![HtmlAction::Plain(Action::Inner(inner))],
            "append" => vec![HtmlAction::Plain(Action::Append(inner))],
            "prepend" => vec![HtmlAction::Plain(Action::Prepend(inner))],
            "before" => vec![HtmlAction::Plain(Action::Before(inner))],
            "after" => vec![HtmlAction::Plain(Action::After(inner))],
            _ => Vec::new(),
        };
        actions.into_iter().map(move |action| Op {
            selector: selector.clone(),
            action,
        })
    })
    .collect()
}

/// The class attribute after `+add -remove` changes.
fn classes(current: Option<&str>, changes: &str) -> String {
    let mut list: Vec<String> = current
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    changes
        .split_whitespace()
        .for_each(|t| match (t.strip_prefix('+'), t.strip_prefix('-')) {
            (Some(add), _) if !list.iter().any(|c| c == add) => list.push(add.to_string()),
            (_, Some(remove)) => list.retain(|c| c != remove),
            _ => {}
        });
    list.join(" ")
}

/// Applies an HTML patch to `base`.
pub fn apply(base: &str, patch: &str) -> Outcome {
    let ops: Vec<Op> = element_ops(patch)
        .into_iter()
        .chain(comment_ops(patch))
        .collect();
    ops.iter().fold(
        Outcome {
            text: base.to_string(),
            ..Outcome::default()
        },
        |mut out, op| {
            let roots = dom::parse_html(&out.text);
            let found = select::select(&roots, &op.selector);
            if found.is_empty() {
                out.missed.push(op.selector.clone());
                return out;
            }
            let edits = found
                .iter()
                .map(|e| match &op.action {
                    HtmlAction::Plain(action) => edit(&out.text, e, action),
                    HtmlAction::Classes(changes) => {
                        let value = classes(e.attr("class"), changes);
                        edit(&out.text, e, &Action::SetAttr("class".into(), value))
                    }
                })
                .collect();
            out.text = splice(&out.text, edits);
            out.applied += 1;
            out
        },
    )
}
