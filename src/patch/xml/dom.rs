//! A lenient XML reader that keeps byte positions, for editing the game's
//! files in place. Accepts what the game ships: fragments with several root
//! elements, a byte-order mark, comments, and the `</>` shorthand that
//! closes the innermost open element. Never re-serialises.

/// An attribute with its positions in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub name: String,
    pub value: String,
    /// From the whitespace before the name to after the closing quote.
    pub span: (usize, usize),
    /// The value between the quotes.
    pub value_span: (usize, usize),
}

/// An element with its positions in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<Attr>,
    /// Start of `<` of the start tag.
    pub start: usize,
    /// End of the start tag (after `>`).
    pub tag_end: usize,
    /// Start of the close tag; `None` for `<a/>`.
    pub close_start: Option<usize>,
    /// End of the whole element (after the close tag).
    pub end: usize,
    pub children: Vec<Element>,
}

impl Element {
    /// The value of an attribute, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// The text between the tags (empty for `<a/>`).
    pub fn inner<'t>(&self, text: &'t str) -> &'t str {
        self.close_start
            .map_or("", |close| &text[self.tag_end..close])
    }
}

/// Position of `needle` at or after `from`, or the end of `text`.
fn find(text: &str, from: usize, needle: &str) -> usize {
    text[from..].find(needle).map_or(text.len(), |i| from + i)
}

/// Parses the attributes inside a start tag (`inner` begins at `offset`).
fn attributes(inner: &str, offset: usize) -> Vec<Attr> {
    let bytes = inner.as_bytes();
    let mut attrs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let ws = i;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let name = &inner[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if name.is_empty() || i >= bytes.len() || bytes[i] != b'=' {
            i = (i + 1).max(name_start + name.len());
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let Some(&quote) = bytes.get(i).filter(|q| **q == b'"' || **q == b'\'') else {
            continue;
        };
        let value_start = i + 1;
        let value_end = inner[value_start..]
            .find(quote as char)
            .map_or(inner.len(), |e| value_start + e);
        attrs.push(Attr {
            name: name.to_string(),
            value: inner[value_start..value_end].to_string(),
            span: (offset + ws, offset + (value_end + 1).min(inner.len())),
            value_span: (offset + value_start, offset + value_end),
        });
        i = value_end + 1;
    }
    attrs
}

/// Closes the innermost element at `close_start..end` and attaches it.
fn close(stack: &mut Vec<Element>, roots: &mut Vec<Element>, close_start: usize, end: usize) {
    if let Some(mut element) = stack.pop() {
        element.close_start = Some(close_start);
        element.end = end;
        match stack.last_mut() {
            Some(parent) => parent.children.push(element),
            None => roots.push(element),
        }
    }
}

/// HTML elements that never have a close tag.
const VOID: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Reads the top-level elements of `text`.
pub fn parse(text: &str) -> Vec<Element> {
    parse_with(text, false)
}

/// Like [`parse`], treating HTML void elements as self-closing.
pub fn parse_html(text: &str) -> Vec<Element> {
    parse_with(text, true)
}

fn parse_with(text: &str, html: bool) -> Vec<Element> {
    let (mut roots, mut stack): (Vec<Element>, Vec<Element>) = (Vec::new(), Vec::new());
    let mut pos = 0;
    while let Some(i) = text[pos..].find('<').map(|i| pos + i) {
        let rest = &text[i..];
        let skip = [
            ("<!--", "-->"),
            ("<![CDATA[", "]]>"),
            ("<?", "?>"),
            ("<!", ">"),
        ]
        .iter()
        .find(|(open, _)| rest.starts_with(open));
        if let Some((_, end)) = skip {
            pos = (find(text, i + 2, end) + end.len()).min(text.len());
            continue;
        }
        let gt = find(text, i, ">");
        let end = (gt + 1).min(text.len());
        if let Some(name) = rest.strip_prefix("</") {
            let name = name[..gt - i - 2].trim();
            // `</>` closes the innermost element; a named close tag closes
            // up to its match (and is ignored if nothing matches).
            let depth = match name.is_empty() {
                true => stack.len().min(1),
                false => stack
                    .iter()
                    .rev()
                    .position(|e| e.name == name)
                    .map_or(0, |d| d + 1),
            };
            (0..depth).for_each(|_| close(&mut stack, &mut roots, i, end));
            pos = end;
            continue;
        }
        let body = &text[i + 1..gt];
        let explicit = body.ends_with('/');
        let body = body.trim_end_matches('/');
        let name_len = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        let void = html && VOID.contains(&body[..name_len].to_lowercase().as_str());
        let self_closing = explicit || void;
        let element = Element {
            name: body[..name_len].to_string(),
            attrs: attributes(&body[name_len..], i + 1 + name_len),
            start: i,
            tag_end: end,
            close_start: None,
            end,
            children: Vec::new(),
        };
        match (self_closing, stack.last_mut()) {
            (true, Some(parent)) => parent.children.push(element),
            (true, None) => roots.push(element),
            (false, _) => stack.push(element),
        }
        pos = end;
    }
    while !stack.is_empty() {
        close(&mut stack, &mut roots, text.len(), text.len());
    }
    roots
}
