//! Text edits on the original XML: splices by byte position, so untouched
//! parts stay byte for byte as the game shipped them.

use super::dom::Element;

/// Replace `start..end` with `text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// What to do with a selected element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Replace(String),
    Append(String),
    /// Insert as the first child.
    Prepend(String),
    /// Replace the content between the tags.
    Inner(String),
    Before(String),
    After(String),
    Remove,
    SetAttr(String, String),
    RemoveAttr(String),
}

/// Leading whitespace of the line `pos` is on.
fn indent(text: &str, pos: usize) -> &str {
    let line = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    let rest = &text[line..pos];
    &rest[..rest.len() - rest.trim_start().len()]
}

/// Start of the line `pos` is on, after a byte-order mark (which stays).
fn line_start(text: &str, pos: usize) -> usize {
    let start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    match text[start..].starts_with('\u{feff}') {
        true => start + '\u{feff}'.len_utf8(),
        false => start,
    }
}

/// Widens `start..end` to whole lines when nothing else is on them.
fn whole_lines(text: &str, start: usize, end: usize) -> (usize, usize) {
    let first = line_start(text, start);
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
    let alone = text[first..start].trim().is_empty() && text[end..line_end].trim().is_empty();
    match alone {
        true => (first, line_end),
        false => (start, end),
    }
}

/// The file's line ending (the game's XML mostly uses CRLF).
pub fn eol(text: &str) -> &'static str {
    match text.contains("\r\n") {
        true => "\r\n",
        false => "\n",
    }
}

/// Escapes a value for a double-quoted attribute.
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// The edit that performs `action` on `element`.
pub fn edit(text: &str, element: &Element, action: &Action) -> Edit {
    let at = |pos: usize, text: String| Edit {
        start: pos,
        end: pos,
        text,
    };
    let pad = indent(text, element.start);
    let nl = eol(text);
    match action {
        Action::Replace(xml) => Edit {
            start: element.start,
            end: element.end,
            text: xml.clone(),
        },
        Action::Remove => {
            let (start, end) = whole_lines(text, element.start, element.end);
            Edit {
                start,
                end,
                text: String::new(),
            }
        }
        Action::Prepend(xml) => match element.close_start {
            Some(_) => at(element.tag_end, xml.clone()),
            None => edit(text, element, &Action::Append(xml.clone())),
        },
        Action::Inner(xml) => match element.close_start {
            Some(close) => Edit {
                start: element.tag_end,
                end: close,
                text: xml.clone(),
            },
            None => edit(text, element, &Action::Append(xml.clone())),
        },
        Action::Before(xml) => at(element.start, format!("{xml}{nl}{pad}")),
        Action::After(xml) => at(element.end, format!("{nl}{pad}{xml}")),
        Action::Append(xml) => match element.close_start {
            // A close tag on its own line: a new line, indented like the
            // existing children.
            Some(close) if text[line_start(text, close)..close].trim().is_empty() => {
                let child_pad = element
                    .children
                    .first()
                    .map_or_else(|| format!("{pad}\t"), |c| indent(text, c.start).to_string());
                at(line_start(text, close), format!("{child_pad}{xml}{nl}"))
            }
            Some(close) => at(close, xml.clone()),
            // `<a/>` becomes `<a>…</a>`.
            None => {
                let open = text[element.start..element.tag_end]
                    .trim_end_matches('>')
                    .trim_end_matches('/')
                    .trim_end();
                let name = &element.name;
                Edit {
                    start: element.start,
                    end: element.tag_end,
                    text: format!("{open}>{xml}</{name}>"),
                }
            }
        },
        Action::SetAttr(name, value) => match element.attrs.iter().find(|a| a.name == *name) {
            Some(attr) => Edit {
                start: attr.value_span.0,
                end: attr.value_span.1,
                text: escape(value),
            },
            None => {
                let close = text[element.start..element.tag_end].trim_end_matches('>');
                let insert = element.start + close.trim_end_matches('/').trim_end().len();
                at(insert, format!(" {name}=\"{}\"", escape(value)))
            }
        },
        Action::RemoveAttr(name) => match element.attrs.iter().find(|a| a.name == *name) {
            Some(attr) => Edit {
                start: attr.span.0,
                end: attr.span.1,
                text: String::new(),
            },
            None => at(element.start, String::new()),
        },
    }
}

/// Applies edits (non-overlapping; insertions at one position keep their
/// order) and returns the new text.
pub fn splice(text: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|e| (e.start, e.end));
    let (mut out, last) = edits
        .iter()
        .fold((String::new(), 0usize), |(mut out, pos), e| {
            if e.start < pos {
                return (out, pos); // overlaps an earlier edit (e.g. inside a removed element)
            }
            out.push_str(&text[pos..e.start]);
            out.push_str(&e.text);
            (out, e.end)
        });
    out.push_str(&text[last..]);
    out
}
