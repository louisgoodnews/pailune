//! `nxm://` registration on Windows (B-08): the per-user URL protocol key
//! `HKCU\Software\Classes\nxm`, written and read with `reg.exe`. Compiled on
//! every platform so the parsing is tested everywhere; only Windows calls it.

use crate::error::{PailuneError, Result};
use std::path::Path;
use std::process::Command;

/// The protocol key.
const KEY: &str = r"HKCU\Software\Classes\nxm";

/// The command Windows runs for a link: `"<exe>" "%1"`.
pub fn command_line(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

/// The default value from `reg query <key> /ve` output. The value name is
/// localised ("(Default)", "(Standard)" …), so the line is found by its type.
pub fn parse_default(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        ["REG_EXPAND_SZ", "REG_SZ"].iter().find_map(|kind| {
            line.split_once(kind)
                .map(|(_, value)| value.trim().to_string())
                .filter(|v| !v.is_empty())
        })
    })
}

/// Splits a Windows command line: whitespace separates, double quotes group.
pub fn split_command(command: &str) -> Vec<String> {
    let (mut args, current, _) = command.chars().fold(
        (Vec::new(), None::<String>, false),
        |(mut args, current, quoted), c| match (quoted, c) {
            (_, '"') => (args, Some(current.unwrap_or_default()), !quoted),
            (false, c) if c.is_whitespace() => {
                args.extend(current);
                (args, None, false)
            }
            (_, c) => (
                args,
                Some(current.unwrap_or_default() + &c.to_string()),
                quoted,
            ),
        },
    );
    args.extend(current);
    args
}

/// Arguments of `command` with `%1` replaced by the link.
pub fn expand(command: &str, url: &str) -> Vec<String> {
    split_command(command)
        .into_iter()
        .map(|arg| arg.replace("%1", url))
        .collect()
}

/// Runs `reg.exe` and fails with its output when it does.
fn reg(args: &[&str]) -> Result<String> {
    let out = crate::platform::background("reg")
        .args(args)
        .output()
        .map_err(|e| PailuneError::Invalid(format!("reg.exe failed: {e}")))?;
    match out.status.success() {
        true => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        false => Err(PailuneError::Invalid(format!(
            "reg.exe failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
    }
}

/// The command currently registered for `nxm://`, if any.
pub fn current_command() -> Option<String> {
    let key = format!(r"{KEY}\shell\open\command");
    reg(&["query", &key, "/ve"])
        .ok()
        .as_deref()
        .and_then(parse_default)
}

/// Makes `exe` the handler; returns the command it replaced.
pub fn register(exe: &Path) -> Result<Option<String>> {
    let ours = command_line(exe);
    let previous = current_command().filter(|c| *c != ours);
    let command_key = format!(r"{KEY}\shell\open\command");
    reg(&["add", KEY, "/ve", "/d", "URL:NXM Protocol", "/f"])?;
    reg(&["add", KEY, "/v", "URL Protocol", "/d", "", "/f"])?;
    reg(&["add", &command_key, "/ve", "/d", &ours, "/f"])?;
    Ok(previous)
}

/// Whether `exe` is the registered handler.
pub fn is_registered(exe: &Path) -> bool {
    current_command().is_some_and(|c| c == command_line(exe))
}

/// Hands `url` to a previously registered handler command.
pub fn forward(command: &str, url: &str) -> Result<()> {
    let argv = expand(command, url);
    let (program, rest) = argv
        .split_first()
        .ok_or_else(|| PailuneError::Invalid("empty handler command".into()))?;
    let mut spawn = Command::new(program);
    spawn.args(rest);
    crate::launch::run::spawn(spawn)
        .map_err(|e| PailuneError::Invalid(format!("could not start {program}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_reg_output_and_forwards_to_vortex() {
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Classes\\nxm\\shell\\open\\command\r\n    (Standard)    REG_SZ    \"C:\\Program Files\\Black Tree Gaming Ltd\\Vortex\\Vortex.exe\" -d \"%1\"\r\n\r\n";
        let command = parse_default(output).unwrap();
        assert_eq!(
            expand(&command, "nxm://skyrim/mods/1/files/2"),
            [
                r"C:\Program Files\Black Tree Gaming Ltd\Vortex\Vortex.exe",
                "-d",
                "nxm://skyrim/mods/1/files/2"
            ]
        );
        assert_eq!(parse_default("ERROR: nothing"), None);
        let ours = command_line(Path::new(r"C:\Games\Pailune\pailune.exe"));
        assert_eq!(ours, r#""C:\Games\Pailune\pailune.exe" "%1""#);
        assert_eq!(
            expand(&ours, "nxm://x"),
            [r"C:\Games\Pailune\pailune.exe", "nxm://x"]
        );
    }
}
