//! Finding elements: an XPath subset (`/a/b`, `//a`, `*`, `[@k='v']`,
//! `[@k]`, `[n]`), `#id`, `.class`, and `find`/`key` identity lookups.

use super::dom::Element;

/// How a patch names its target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    XPath(String),
    /// `#value`: an `id`/`Id`/`ID` attribute.
    Id(String),
    /// `.value`: one of the words of a `class` attribute.
    Class(String),
    /// An element with this tag whose identity attribute equals `key`.
    Find {
        tag: String,
        key: String,
    },
}

impl Target {
    /// Reads `at="#id"`, `at=".cls"` or an XPath.
    pub fn parse(at: &str) -> Self {
        match (at.strip_prefix('#'), at.strip_prefix('.')) {
            (Some(id), _) => Target::Id(id.to_string()),
            (_, Some(class)) if !class.starts_with('/') && !class.starts_with('.') => {
                Target::Class(class.to_string())
            }
            _ => Target::XPath(at.to_string()),
        }
    }
}

/// Identity attributes, strongest first (merge and `find`/`key`).
pub const IDENTITY: [&str; 6] = ["Key", "Name", "Id", "key", "name", "id"];

/// The identity attribute of an element: the first of [`IDENTITY`], else
/// any `*_key`/`*_id`/`*_name` attribute.
pub fn identity(element: &Element) -> Option<(&str, &str)> {
    let suffixed = |a: &&super::dom::Attr| {
        let lower = a.name.to_lowercase();
        ["_key", "_id", "_name"].iter().any(|s| lower.ends_with(s))
    };
    IDENTITY
        .iter()
        .find_map(|name| element.attrs.iter().find(|a| a.name == *name))
        .or_else(|| element.attrs.iter().find(suffixed))
        .map(|a| (a.name.as_str(), a.value.as_str()))
}

/// Every element below `roots`, depth first in document order.
fn all(roots: &[Element]) -> Vec<&Element> {
    roots
        .iter()
        .flat_map(|e| std::iter::once(e).chain(all(&e.children)))
        .collect()
}

/// One XPath step: descendant axis, name test, predicates.
struct Step {
    descendant: bool,
    name: String,
    predicates: Vec<String>,
}

fn steps(path: &str) -> Vec<Step> {
    let mut out = Vec::new();
    let mut descendant = !path.starts_with('/');
    let mut rest = path.trim_start_matches('/');
    if path.starts_with("//") {
        descendant = true;
    }
    while !rest.is_empty() {
        let (part, next) = split_step(rest);
        let name_end = part.find('[').unwrap_or(part.len());
        let predicates = part[name_end..]
            .split(']')
            .filter_map(|p| p.trim().strip_prefix('[').map(str::to_string))
            .collect();
        out.push(Step {
            descendant,
            name: part[..name_end].trim().to_string(),
            predicates,
        });
        descendant = next.starts_with("//");
        rest = next.trim_start_matches('/');
    }
    out
}

/// Splits off the first step, ignoring `/` inside predicates.
fn split_step(path: &str) -> (&str, &str) {
    let mut depth = 0;
    for (i, c) in path.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            '/' if depth == 0 => return (&path[..i], &path[i..]),
            _ => {}
        }
    }
    (path, "")
}

/// Whether an element passes a non-positional predicate.
fn passes(element: &Element, predicate: &str) -> bool {
    let unquote = |v: &str| v.trim().trim_matches(['\'', '"']).to_string();
    match predicate.strip_prefix('@').map(|p| p.split_once('=')) {
        Some(Some((name, value))) => element.attr(name.trim()) == Some(unquote(value).as_str()),
        Some(None) => element.attr(predicate[1..].trim()).is_some(),
        None => true,
    }
}

/// Applies one step to a set of context elements (`None`: the document).
fn apply<'d>(
    roots: &'d [Element],
    context: &[Option<&'d Element>],
    step: &Step,
) -> Vec<&'d Element> {
    context
        .iter()
        .flat_map(|ctx| {
            let children = ctx.map_or(roots, |e| e.children.as_slice());
            let pool: Vec<&Element> = match step.descendant {
                true => all(children),
                false => children.iter().collect(),
            };
            let mut found: Vec<&Element> = pool
                .into_iter()
                .filter(|e| step.name == "*" || e.name == step.name)
                .collect();
            for p in &step.predicates {
                found = match p.trim().parse::<usize>() {
                    Ok(n) => found.get(n.wrapping_sub(1)).copied().into_iter().collect(),
                    Err(_) => found.into_iter().filter(|e| passes(e, p)).collect(),
                };
            }
            found
        })
        .collect()
}

/// The elements a target names, in document order, without duplicates.
pub fn select<'d>(roots: &'d [Element], target: &Target) -> Vec<&'d Element> {
    let mut found: Vec<&Element> = match target {
        Target::Id(id) => all(roots)
            .into_iter()
            .filter(|e| ["id", "Id", "ID"].iter().any(|k| e.attr(k) == Some(id)))
            .collect(),
        Target::Class(class) => all(roots)
            .into_iter()
            .filter(|e| {
                e.attr("class")
                    .is_some_and(|c| c.split_whitespace().any(|w| w == class))
            })
            .collect(),
        Target::Find { tag, key } => all(roots)
            .into_iter()
            .filter(|e| e.name == *tag && identity(e).is_some_and(|(_, v)| v == key))
            .collect(),
        Target::XPath(path) => steps(path)
            .iter()
            .fold(vec![None], |ctx, step| {
                apply(roots, &ctx, step).into_iter().map(Some).collect()
            })
            .into_iter()
            .flatten()
            .collect(),
    };
    found.sort_by_key(|e| e.start);
    found.dedup_by_key(|e| e.start);
    found
}
