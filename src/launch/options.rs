//! Parsing Steam-style launch options: `ENV=1 OTHER="a b" %command% -arg`.

use super::LaunchSpec;

/// Splits on whitespace while honouring double and single quotes.
fn tokenize(text: &str) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Quote {
        None,
        Single,
        Double,
    }
    let (mut tokens, current, _) = text.chars().fold(
        (Vec::new(), None::<String>, Quote::None),
        |(mut tokens, current, quote), c| match (quote, c) {
            (Quote::None, c) if c.is_whitespace() => {
                tokens.extend(current);
                (tokens, None, quote)
            }
            (Quote::None, '"') => (tokens, Some(current.unwrap_or_default()), Quote::Double),
            (Quote::None, '\'') => (tokens, Some(current.unwrap_or_default()), Quote::Single),
            (Quote::Double, '"') | (Quote::Single, '\'') => (tokens, current, Quote::None),
            (_, c) => {
                let mut token = current.unwrap_or_default();
                token.push(c);
                (tokens, Some(token), quote)
            }
        },
    );
    tokens.extend(current);
    tokens
}

/// Whether a token is an environment assignment like `KEY=value`.
fn as_env(token: &str) -> Option<(String, String)> {
    let (key, value) = token.split_once('=')?;
    let valid = !key.is_empty()
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !key.starts_with(|c: char| c.is_ascii_digit());
    valid.then(|| (key.to_string(), value.to_string()))
}

/// Parses launch options. Without `%command%` every non-assignment token
/// is treated as a game argument.
pub fn parse(text: &str) -> LaunchSpec {
    let tokens = tokenize(text);
    let split = tokens.iter().position(|t| t == "%command%");
    let (before, after): (&[String], &[String]) = match split {
        Some(i) => (&tokens[..i], &tokens[i + 1..]),
        None => (&tokens, &[]),
    };
    let env = before.iter().filter_map(|t| as_env(t)).collect();
    let loose = before.iter().filter(|t| as_env(t).is_none()).cloned();
    LaunchSpec {
        env,
        args: loose.chain(after.iter().cloned()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_command_and_args() {
        let spec = parse(r#"WINEDLLOVERRIDES="winmm=n,b" DXVK_HUD=1 %command% -dx12 "--name=a b""#);
        assert_eq!(
            spec.env,
            [
                ("WINEDLLOVERRIDES".into(), "winmm=n,b".into()),
                ("DXVK_HUD".into(), "1".into())
            ]
        );
        assert_eq!(spec.args, ["-dx12", "--name=a b"]);
        assert_eq!(parse("-windowed").args, ["-windowed"]);
        assert_eq!(parse("   "), LaunchSpec::default());
    }
}
