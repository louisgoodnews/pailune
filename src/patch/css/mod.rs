//! CSS patch and merge files (`*.css.patch`, `*.css.merge`), §10.5.
//!
//! Directives in comments switch the mode for the rules that follow:
//! `/* @merge */` (default: property by property), `/* @replace */`
//! (whole rule), `/* @add */` (append) and `/* @remove ".sel" */`.
//! Untouched parts of the original stylesheet are kept byte for byte.

/// One top-level rule: `prelude { body }` with its exact source text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Rule {
    selector: String,
    body: String,
    raw: String,
}

/// Mode set by the latest directive.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Merge,
    Replace,
    Add,
}

/// What a patch file asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Op {
    Rule(Mode, Rule),
    Remove(String),
}

fn normalise(selector: &str) -> String {
    selector.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Splits text into top-level rules and the text between them.
fn split(text: &str) -> (Vec<String>, Vec<Rule>) {
    let mut gaps = vec![String::new()];
    let mut rules = Vec::new();
    let (mut depth, mut start, mut open) = (0usize, 0usize, 0usize);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = text[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |e| i + 2 + e + 2);
                if depth == 0 {
                    gaps.last_mut().unwrap().push_str(&text[i..end]);
                    start = end;
                }
                i = end;
                continue;
            }
            b'{' => {
                if depth == 0 {
                    gaps.last_mut()
                        .unwrap()
                        .push_str(&text[start..start + leading_ws(&text[start..i])]);
                    open = i;
                }
                depth += 1;
            }
            b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let prelude_start = start + leading_ws(&text[start..open]);
                    rules.push(Rule {
                        selector: normalise(&text[prelude_start..open]),
                        body: text[open + 1..i].to_string(),
                        raw: text[prelude_start..=i].to_string(),
                    });
                    gaps.push(String::new());
                    start = i + 1;
                }
            }
            _ if depth == 0 && start == i && bytes[i].is_ascii_whitespace() => {
                gaps.last_mut().unwrap().push(bytes[i] as char);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    gaps.last_mut()
        .unwrap()
        .push_str(&text[start.min(text.len())..]);
    (gaps, rules)
}

/// Bytes of leading whitespace (a BOM counts, so it stays outside selectors).
fn leading_ws(s: &str) -> usize {
    s.len()
        - s.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .len()
}

/// `name: value` declarations of a rule body, in order.
fn declarations(body: &str) -> Vec<(String, String)> {
    body.split(';')
        .filter_map(|d| d.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// Reads the operations of a patch file.
fn operations(patch: &str, default: Mode) -> Vec<Op> {
    let (gaps, rules) = split(patch);
    let mut mode = default;
    let mut ops = Vec::new();
    gaps.iter()
        .zip(rules.iter().map(Some).chain(std::iter::once(None)))
        .for_each(|(gap, rule)| {
            gap.split("/*")
                .skip(1)
                .filter_map(|c| c.split("*/").next())
                .map(str::trim)
                .for_each(|directive| match directive.split_whitespace().next() {
                    Some("@merge") => mode = Mode::Merge,
                    Some("@replace") => mode = Mode::Replace,
                    Some("@add") => mode = Mode::Add,
                    Some("@remove") => {
                        let target = directive
                            .trim_start_matches("@remove")
                            .trim()
                            .trim_matches(['"', '\'']);
                        ops.push(Op::Remove(normalise(target)));
                    }
                    _ => {}
                });
            if let Some(rule) = rule {
                ops.push(Op::Rule(mode.clone(), rule.clone()));
            }
        });
    ops
}

/// A merged rule: the original's properties (overridden in place) plus new ones.
fn merged(base: &Rule, patch: &Rule) -> String {
    let changes = declarations(&patch.body);
    let mut decls = declarations(&base.body);
    changes
        .iter()
        .for_each(|(k, v)| match decls.iter_mut().find(|(bk, _)| bk == k) {
            Some(existing) => existing.1 = v.clone(),
            None => decls.push((k.clone(), v.clone())),
        });
    let body: Vec<String> = decls.iter().map(|(k, v)| format!("{k}: {v};")).collect();
    format!("{} {{ {} }}", base.selector, body.join(" "))
}

/// Applies a patch to a stylesheet. Merging is the default mode, which is
/// also all a `.css.merge` file does.
pub fn apply(base: &str, patch: &str) -> String {
    let default = Mode::Merge;
    let (gaps, mut rules) = split(base);
    let mut texts: Vec<Option<String>> = rules.iter().map(|r| Some(r.raw.clone())).collect();
    let mut appended: Vec<String> = Vec::new();
    operations(patch, default)
        .into_iter()
        .for_each(|op| match op {
            Op::Remove(selector) => rules
                .iter()
                .enumerate()
                .filter(|(_, r)| r.selector == selector)
                .for_each(|(i, _)| texts[i] = None),
            Op::Rule(Mode::Add, rule) => appended.push(rule.raw),
            Op::Rule(mode, rule) => match rules.iter().position(|r| r.selector == rule.selector) {
                Some(i) => {
                    let text = match mode {
                        Mode::Replace => rule.raw.clone(),
                        _ => merged(&rules[i], &rule),
                    };
                    rules[i].body = text[text.find('{').map_or(0, |b| b + 1)
                        ..text.rfind('}').unwrap_or(text.len())]
                        .to_string();
                    texts[i] = Some(text);
                }
                None => appended.push(rule.raw),
            },
        });
    let newline = if base.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: String = gaps
        .iter()
        .zip(texts.iter().map(Some).chain(std::iter::once(None)))
        .fold(String::new(), |mut out, (gap, text)| {
            out.push_str(gap);
            if let Some(Some(text)) = text {
                out.push_str(text);
            }
            out
        });
    appended.iter().for_each(|rule| {
        if !out.ends_with(newline) {
            out.push_str(newline);
        }
        out.push_str(rule);
        out.push_str(newline);
    });
    out
}

#[cfg(test)]
mod tests;
