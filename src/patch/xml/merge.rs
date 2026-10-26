//! `*.xml.merge`: elements are matched by tag and identity attribute
//! (`Key > Name > Id > key > name > id`, then `*_key`/`*_id`/`*_name`);
//! attributes are copied onto the match, children merge recursively,
//! `__delete="true"` removes the match, unmatched elements are appended.

use super::Outcome;
use super::dom::{self, Element};
use super::edit::{Action, Edit, edit};
use super::select::identity;

/// Marks an element for deletion.
const DELETE: &str = "__delete";

/// The base element `wanted` refers to among `candidates`.
fn matching<'d>(candidates: &'d [Element], wanted: &Element) -> Option<&'d Element> {
    let same_tag = candidates.iter().filter(|c| c.name == wanted.name);
    match identity(wanted) {
        Some((attr, value)) => same_tag.clone().find(|c| c.attr(attr) == Some(value)),
        // A container without identity (e.g. the document root) merges into
        // the first element with its tag.
        None if !wanted.children.is_empty() => same_tag.clone().next(),
        None => None,
    }
}

/// Edits that merge `wanted` (from `patch`) into `candidates` of `base`;
/// `append_at` is where unmatched elements go.
fn merge_into(
    base: &str,
    patch: &str,
    candidates: &[Element],
    wanted: &[Element],
    append_at: usize,
    edits: &mut Vec<Edit>,
    out: &mut Outcome,
) {
    wanted.iter().for_each(|w| match matching(candidates, w) {
        Some(found) if w.attr(DELETE) == Some("true") => {
            edits.push(edit(base, found, &Action::Remove));
            out.applied += 1;
        }
        // `<Item …/>` gaining children: rewrite the element as a whole, so the
        // attribute changes and the new content cannot collide.
        Some(found) if found.close_start.is_none() && !w.children.is_empty() => {
            edits.push(open_up(base, patch, found, w));
            out.applied += 1;
        }
        Some(found) => {
            w.attrs.iter().filter(|a| a.name != DELETE).for_each(|a| {
                edits.push(edit(
                    base,
                    found,
                    &Action::SetAttr(a.name.clone(), a.value.clone()),
                ));
            });
            out.applied += 1;
            let inner_end = found.close_start.unwrap_or(found.end);
            merge_into(
                base,
                patch,
                &found.children,
                &w.children,
                inner_end,
                edits,
                out,
            );
        }
        None if w.attr(DELETE) == Some("true") => {
            out.missed.push(format!("<{}> to delete", w.name))
        }
        None => {
            let raw = &patch[w.start..w.end];
            let nl = super::edit::eol(base);
            edits.push(Edit {
                start: append_at,
                end: append_at,
                text: format!("{raw}{nl}"),
            });
            out.applied += 1;
        }
    });
}

/// A self-closing element with merged attributes and `w`'s children inside.
fn open_up(base: &str, patch: &str, found: &Element, w: &Element) -> Edit {
    let tag = &base[found.start..found.tag_end];
    let local = dom::parse(tag);
    let attrs: Vec<Edit> = local
        .first()
        .map(|e| {
            w.attrs
                .iter()
                .filter(|a| a.name != DELETE)
                .map(|a| edit(tag, e, &Action::SetAttr(a.name.clone(), a.value.clone())))
                .collect()
        })
        .unwrap_or_default();
    let tag = super::edit::splice(tag, attrs);
    let open = tag.trim_end_matches('>').trim_end_matches('/').trim_end();
    let inner: String = w.children.iter().map(|c| &patch[c.start..c.end]).collect();
    Edit {
        start: found.start,
        end: found.end,
        text: format!("{open}>{inner}</{}>", found.name),
    }
}

/// Merges `patch` into `base`.
pub fn merge(base: &str, patch: &str) -> Outcome {
    let roots = dom::parse(base);
    let patch_roots = dom::parse(patch);
    let wanted: &[Element] = match patch_roots.as_slice() {
        [wrapper] if wrapper.name == "xml-merge" => &wrapper.children,
        all => all,
    };
    let mut out = Outcome::default();
    let mut edits = Vec::new();
    let end = base.len();
    if !base.ends_with('\n') && !base.is_empty() {
        let nl = super::edit::eol(base);
        edits.push(Edit {
            start: end,
            end,
            text: nl.into(),
        });
    }
    merge_into(base, patch, &roots, wanted, end, &mut edits, &mut out);
    out.text = super::edit::splice(base, edits);
    out
}
