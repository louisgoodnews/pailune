//! A minimal parser for Valve's KeyValues text format (`.vdf`, `.acf`).

use std::iter::Peekable;
use std::str::Chars;

/// A parsed KeyValues node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vdf {
    Str(String),
    /// Children in file order (keys may repeat).
    Map(Vec<(String, Vdf)>),
}

/// Lexical tokens of the format.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Str(String),
    Open,
    Close,
}

impl Vdf {
    /// Child node by key (first match, case-insensitive like Steam).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Map(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    /// The string value, if this is a leaf.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Map(_) => None,
        }
    }

    /// Child maps/values in order.
    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Map(entries) => entries,
            Vdf::Str(_) => &[],
        }
    }
}

/// Reads a quoted string; the opening quote is already consumed.
fn quoted(chars: &mut Peekable<Chars>) -> String {
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => break,
            },
            c => out.push(c),
        }
    }
    out
}

/// Splits the text into tokens, skipping whitespace and `//` comments.
fn tokenize(text: &str) -> Vec<Token> {
    let mut chars = text.chars().peekable();
    std::iter::from_fn(move || {
        loop {
            match chars.next()? {
                '"' => return Some(Token::Str(quoted(&mut chars))),
                '{' => return Some(Token::Open),
                '}' => return Some(Token::Close),
                '/' if chars.peek() == Some(&'/') => {
                    chars.by_ref().find(|c| *c == '\n');
                }
                c if c.is_whitespace() => {}
                c => {
                    // Unquoted token (rare): read until whitespace or brace.
                    let mut word = c.to_string();
                    while let Some(&n) = chars
                        .peek()
                        .filter(|n| !n.is_whitespace() && **n != '{' && **n != '}')
                    {
                        word.push(n);
                        chars.next();
                    }
                    return Some(Token::Str(word));
                }
            }
        }
    })
    .collect()
}

/// Parses `key value` / `key { ... }` pairs until a closing brace or the end.
fn entries(tokens: &mut std::vec::IntoIter<Token>) -> Vec<(String, Vdf)> {
    std::iter::from_fn(|| match tokens.next()? {
        Token::Str(key) => match tokens.next()? {
            Token::Str(value) => Some((key, Vdf::Str(value))),
            Token::Open => Some((key, Vdf::Map(entries(tokens)))),
            Token::Close => None,
        },
        Token::Open | Token::Close => None,
    })
    .collect()
}

/// Parses a whole document into a root map.
pub fn parse(text: &str) -> Vdf {
    Vdf::Map(entries(&mut tokenize(text).into_iter()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_folders() {
        let text = r#"
            "libraryfolders"
            {
                // comment
                "0" { "path" "/home/u/.local/share/Steam" "apps" { "228980" "1" } }
                "1" { "path" "/mnt/games\\lib" "apps" { "3321460" "2" } }
            }"#;
        let root = parse(text);
        let folders = root.get("libraryfolders").unwrap();
        let paths: Vec<_> = folders
            .entries()
            .iter()
            .filter_map(|(_, f)| f.get("path")?.as_str())
            .collect();
        assert_eq!(paths, ["/home/u/.local/share/Steam", "/mnt/games\\lib"]);
        assert!(
            folders
                .get("1")
                .unwrap()
                .get("apps")
                .unwrap()
                .get("3321460")
                .is_some()
        );
    }
}
