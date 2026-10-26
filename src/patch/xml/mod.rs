//! XML patch and merge files (`*.xml.patch`, `*.xml.merge`), §10.5 of
//! `docs/paz-format.md`. The game's XML is edited in place: only the
//! changed spans are rewritten, everything else stays byte for byte.
//!
//! Patch files come in two forms:
//! * JSON: `{"operations": [{"op", "xpath", "value", "attribute"}]}` with
//!   `replace`, `add`, `add-before`, `add-after`, `remove`, `set-attr`,
//!   `remove-attr`;
//! * XML: `<xml-patch>` with `<set>`, `<unset>`, `<insert>`/`<add>`,
//!   `<insert-before>`, `<insert-after>`, `<replace>`, `<delete>`/`<remove>`,
//!   targeted by `target="xpath"`, `at="#id"`/`".cls"`, or `find="tag" key="K"`.

pub mod dom;
pub mod edit;
pub mod merge;
pub mod select;

use edit::{Action, edit, splice};
use select::{Target, select};
use serde_json::Value;

/// One operation of a patch file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub target: Target,
    pub action: Action,
}

/// Outcome of applying a patch or merge file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub text: String,
    pub applied: usize,
    /// Operations whose target matched nothing.
    pub missed: Vec<String>,
}

/// Operations of a JSON patch file.
fn json_ops(value: &Value) -> Result<Vec<Op>, String> {
    let ops = value
        .get("operations")
        .and_then(Value::as_array)
        .ok_or("a JSON XML patch needs an \"operations\" list")?;
    ops.iter()
        .map(|op| {
            let text = |k: &str| {
                op.get(k)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            let at = ["xpath", "target", "at"]
                .iter()
                .map(|k| text(k))
                .find(|t| !t.is_empty())
                .ok_or("an operation has no xpath")?;
            let action = match text("op").as_str() {
                "replace" => Action::Replace(text("value")),
                "add" | "append" => Action::Append(text("value")),
                "add-before" | "insert-before" => Action::Before(text("value")),
                "add-after" | "insert-after" => Action::After(text("value")),
                "remove" | "delete" => Action::Remove,
                "set-attr" | "remove-attr" if text("attribute").is_empty() => {
                    return Err("set-attr/remove-attr needs an \"attribute\"".into());
                }
                "set-attr" => Action::SetAttr(text("attribute"), text("value")),
                "remove-attr" => Action::RemoveAttr(text("attribute")),
                other => return Err(format!("unknown XML patch operation \"{other}\"")),
            };
            Ok(Op {
                target: Target::parse(&at),
                action,
            })
        })
        .collect()
}

/// The target named by an `<xml-patch>` operation element.
fn xml_target(op: &dom::Element) -> Option<Target> {
    let get = |k: &str| op.attr(k).map(str::to_string);
    match (get("target"), get("at"), get("find"), get("key")) {
        (Some(path), _, _, _) => Some(Target::XPath(path)),
        (_, Some(at), _, _) => Some(Target::parse(&at)),
        (_, _, Some(tag), Some(key)) => Some(Target::Find { tag, key }),
        _ => None,
    }
}

/// Operations of an `<xml-patch>` file.
fn xml_ops(text: &str) -> Result<Vec<Op>, String> {
    let roots = dom::parse(text);
    let root = roots
        .iter()
        .find(|e| e.name == "xml-patch")
        .ok_or("no <xml-patch> element")?;
    let reserved = [
        "target",
        "at",
        "find",
        "key",
        "attribute",
        "attr",
        "value",
        "name",
    ];
    root.children
        .iter()
        .map(|op| {
            let target = xml_target(op).ok_or_else(|| format!("<{}> has no target", op.name))?;
            let inner = op.inner(text).trim().to_string();
            let content = op.attr("value").map(str::to_string).unwrap_or(inner);
            let attr_name = op
                .attr("attribute")
                .or(op.attr("attr"))
                .or(op.attr("name"))
                .unwrap_or_default()
                .to_string();
            let actions: Vec<Action> = match op.name.as_str() {
                "set" if !attr_name.is_empty() => vec![Action::SetAttr(
                    attr_name,
                    op.attr("value").unwrap_or_default().to_string(),
                )],
                "set" => op
                    .attrs
                    .iter()
                    .filter(|a| !reserved.contains(&a.name.as_str()))
                    .map(|a| Action::SetAttr(a.name.clone(), a.value.clone()))
                    .collect(),
                "unset" if attr_name.is_empty() => {
                    return Err("<unset> needs an attribute".to_string());
                }
                "unset" => vec![Action::RemoveAttr(attr_name)],
                "insert" | "add" | "append" => vec![Action::Append(content)],
                "insert-before" => vec![Action::Before(content)],
                "insert-after" => vec![Action::After(content)],
                "replace" => vec![Action::Replace(content)],
                "delete" | "remove" => vec![Action::Remove],
                other => return Err(format!("unknown XML patch operation <{other}>")),
            };
            Ok(actions
                .into_iter()
                .map(|action| Op {
                    target: target.clone(),
                    action,
                })
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|ops| ops.into_iter().flatten().collect())
}

/// Reads a patch file in either form.
pub fn parse_patch(patch: &str) -> Result<Vec<Op>, String> {
    let body = patch.trim_start_matches('\u{feff}').trim_start();
    match body.starts_with('{') {
        true => serde_json::from_str(body)
            .map_err(|e| e.to_string())
            .and_then(|v| json_ops(&v)),
        false => xml_ops(body),
    }
}

/// Applies operations one after another (later ones see earlier results).
pub fn apply_ops(base: &str, ops: &[Op]) -> Outcome {
    ops.iter().fold(
        Outcome {
            text: base.to_string(),
            ..Outcome::default()
        },
        |mut out, op| {
            let roots = dom::parse(&out.text);
            let found = select(&roots, &op.target);
            match found.is_empty() {
                true => out.missed.push(format!("{:?}", op.target)),
                false => {
                    let edits = found
                        .iter()
                        .map(|e| edit(&out.text, e, &op.action))
                        .collect();
                    out.text = splice(&out.text, edits);
                    out.applied += 1;
                }
            }
            out
        },
    )
}

/// Applies a `*.xml.patch` or `*.xml.merge` file (by its name) to `base`.
pub fn apply(base: &str, file_name: &str, patch: &str) -> Result<Outcome, String> {
    match file_name.to_lowercase().ends_with(".merge") {
        true => Ok(merge::merge(base, patch)),
        false => parse_patch(patch).map(|ops| apply_ops(base, &ops)),
    }
}

#[cfg(test)]
mod tests;
