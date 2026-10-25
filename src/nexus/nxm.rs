//! Parsing `nxm://` links handed over by the Nexus website.

use super::NxmLink;
use crate::error::{PyluneError, Result};

/// Parses `nxm://<domain>/mods/<mod_id>/files/<file_id>[?key=..&expires=..]`.
pub fn parse(url: &str) -> Result<NxmLink> {
    let invalid = || PyluneError::Invalid(format!("not a valid nxm link: {url}"));
    let rest = url.trim().strip_prefix("nxm://").ok_or_else(invalid)?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let segments: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let (domain, mod_id, file_id) = match segments.as_slice() {
        [domain, "mods", mod_id, "files", file_id] => (*domain, *mod_id, *file_id),
        _ => return Err(invalid()),
    };
    let param = |name: &str| {
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    };
    Ok(NxmLink {
        domain: domain.to_lowercase(),
        mod_id: mod_id.parse().map_err(|_| invalid())?,
        file_id: file_id.parse().map_err(|_| invalid())?,
        key: param("key"),
        expires: param("expires").and_then(|v| v.parse().ok()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_and_bare_links() {
        let link =
            parse("nxm://CrimsonDesert/mods/12/files/345?key=abc&expires=99&user_id=1").unwrap();
        assert_eq!(
            (link.domain.as_str(), link.mod_id, link.file_id),
            ("crimsondesert", 12, 345)
        );
        assert_eq!((link.key.as_deref(), link.expires), (Some("abc"), Some(99)));
        let bare = parse("nxm://crimsondesert/mods/1/files/2").unwrap();
        assert_eq!((bare.key, bare.expires), (None, None));
        assert!(parse("nxm://crimsondesert/mods/x/files/2").is_err());
        assert!(parse("https://nexusmods.com").is_err());
    }
}
