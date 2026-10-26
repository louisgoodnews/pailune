//! Reading `ModuleConfig.xml` (FOMOD schema 5.0, tolerant of older files).

use super::{
    Condition, ConditionalInstall, Config, FileInstall, Group, GroupKind, Plugin, PluginKind, Step,
};
use crate::error::{PyluneError, Result};
use roxmltree::{Document, Node};

/// Child elements named `name` (namespace-free, case-insensitive).
fn children<'a, 'i>(node: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> {
    node.children()
        .filter(move |n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case(name))
}

/// The first child element named `name`.
fn child<'a, 'i>(node: Node<'a, 'i>, name: &'a str) -> Option<Node<'a, 'i>> {
    children(node, name).next()
}

/// An attribute, empty when missing.
fn attr(node: Node, name: &str) -> String {
    node.attributes()
        .find(|a| a.name().eq_ignore_ascii_case(name))
        .map(|a| a.value().to_string())
        .unwrap_or_default()
}

/// Sorts by name unless the container says `order="Explicit"` (the
/// schema's default order is ascending).
fn ordered<T>(container: Node, mut items: Vec<T>, name: impl Fn(&T) -> String) -> Vec<T> {
    match attr(container, "order").as_str() {
        "Explicit" => items,
        "Descending" => {
            items.sort_by_key(|i| std::cmp::Reverse(name(i).to_lowercase()));
            items
        }
        _ => {
            items.sort_by_key(|i| name(i).to_lowercase());
            items
        }
    }
}

/// `<file>` and `<folder>` children.
fn files(node: Option<Node>) -> Vec<FileInstall> {
    node.into_iter()
        .flat_map(|n| n.children().filter(|c| c.is_element()))
        .filter_map(|n| {
            let folder = match n.tag_name().name().to_lowercase().as_str() {
                "file" => false,
                "folder" => true,
                _ => return None,
            };
            Some(FileInstall {
                source: attr(n, "source"),
                destination: attr(n, "destination"),
                folder,
                priority: attr(n, "priority").parse().unwrap_or(0),
            })
        })
        .collect()
}

/// Deepest nesting of `<dependencies>` followed; real installers use two or
/// three levels, a hostile one could exhaust the stack.
const MAX_DEPTH: usize = 32;

/// A dependency element or a `<dependencies>`/`<visible>` container.
fn condition(node: Node) -> Condition {
    nested(node, 0)
}

/// [`condition`] at nesting `depth`; anything deeper than [`MAX_DEPTH`]
/// counts as met, like other conditions Pylune cannot judge.
fn nested(node: Node, depth: usize) -> Condition {
    match node.tag_name().name().to_lowercase().as_str() {
        "flagdependency" => Condition::Flag(attr(node, "flag"), attr(node, "value")),
        "filedependency" => Condition::File(attr(node, "file"), attr(node, "state")),
        "dependencies" | "visible" | "moduledependencies" if depth >= MAX_DEPTH => {
            tracing::warn!("FOMOD conditions nested too deeply; treating them as met");
            Condition::Always
        }
        "dependencies" | "visible" | "moduledependencies" => {
            let items: Vec<Condition> = node
                .children()
                .filter(|c| c.is_element())
                .map(|c| nested(c, depth + 1))
                .collect();
            match attr(node, "operator").as_str() {
                "Or" => Condition::Any(items),
                _ => Condition::All(items),
            }
        }
        _ => Condition::Always,
    }
}

/// A `<type name>` value.
fn plugin_kind(name: &str) -> PluginKind {
    match name {
        "Required" => PluginKind::Required,
        "Recommended" => PluginKind::Recommended,
        "NotUsable" => PluginKind::NotUsable,
        "CouldBeUsable" => PluginKind::CouldBeUsable,
        _ => PluginKind::Optional,
    }
}

fn plugin(node: Node) -> Plugin {
    let descriptor = child(node, "typeDescriptor");
    let simple = descriptor
        .and_then(|d| child(d, "type"))
        .map(|t| attr(t, "name"));
    let dependency = descriptor.and_then(|d| child(d, "dependencyType"));
    let default = dependency
        .and_then(|d| child(d, "defaultType"))
        .map(|t| attr(t, "name"));
    let kind_rules = dependency
        .and_then(|d| child(d, "patterns"))
        .into_iter()
        .flat_map(|p| children(p, "pattern"))
        .filter_map(|p| {
            let when = child(p, "dependencies").map(condition)?;
            let kind = child(p, "type").map(|t| plugin_kind(&attr(t, "name")))?;
            Some((when, kind))
        })
        .collect();
    Plugin {
        name: attr(node, "name"),
        description: child(node, "description")
            .and_then(|d| d.text())
            .unwrap_or_default()
            .trim()
            .to_string(),
        image: child(node, "image")
            .map(|i| attr(i, "path"))
            .filter(|p| !p.is_empty()),
        files: files(child(node, "files")),
        flags: child(node, "conditionFlags")
            .into_iter()
            .flat_map(|f| children(f, "flag"))
            .map(|f| {
                (
                    attr(f, "name"),
                    f.text().unwrap_or_default().trim().to_string(),
                )
            })
            .collect(),
        kind: plugin_kind(&simple.or(default).unwrap_or_default()),
        kind_rules,
    }
}

fn group(node: Node) -> Group {
    let kind = match attr(node, "type").as_str() {
        "SelectExactlyOne" => GroupKind::ExactlyOne,
        "SelectAtMostOne" => GroupKind::AtMostOne,
        "SelectAtLeastOne" => GroupKind::AtLeastOne,
        "SelectAll" => GroupKind::All,
        _ => GroupKind::Any,
    };
    let list = child(node, "plugins");
    let plugins = list.map_or_else(Vec::new, |l| {
        ordered(l, children(l, "plugin").map(plugin).collect(), |p| {
            p.name.clone()
        })
    });
    Group {
        name: attr(node, "name"),
        kind,
        plugins,
    }
}

fn step(node: Node) -> Step {
    let list = child(node, "optionalFileGroups");
    Step {
        name: attr(node, "name"),
        visible: child(node, "visible").map(condition),
        groups: list.map_or_else(Vec::new, |l| {
            ordered(l, children(l, "group").map(group).collect(), |g| {
                g.name.clone()
            })
        }),
    }
}

/// Parses the text of a `ModuleConfig.xml`.
pub fn parse(text: &str) -> Result<Config> {
    let doc = Document::parse(text)
        .map_err(|e| PyluneError::Invalid(format!("FOMOD ModuleConfig.xml is invalid: {e}")))?;
    let root = doc.root_element();
    let steps = child(root, "installSteps").map_or_else(Vec::new, |s| {
        ordered(s, children(s, "installStep").map(step).collect(), |s| {
            s.name.clone()
        })
    });
    let conditional = child(root, "conditionalFileInstalls")
        .and_then(|c| child(c, "patterns"))
        .into_iter()
        .flat_map(|p| children(p, "pattern"))
        .map(|p| ConditionalInstall {
            condition: child(p, "dependencies").map_or(Condition::Always, condition),
            files: files(child(p, "files")),
        })
        .collect();
    Ok(Config {
        name: child(root, "moduleName")
            .and_then(|n| n.text())
            .unwrap_or_default()
            .trim()
            .to_string(),
        required: files(child(root, "requiredInstallFiles")),
        steps,
        conditional,
    })
}
