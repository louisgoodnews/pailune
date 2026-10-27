//! Parsing `nxm://` links handed over by the Nexus website.

use super::NxmLink;
use crate::error::{PailuneError, Result};

/// Parses `nxm://<domain>/mods/<mod_id>/files/<file_id>[?key=..&expires=..]`.
pub fn parse(url: &str) -> Result<NxmLink> {
    // Never echo the query: it carries the per-download key.
    let shown = url.split('?').next().unwrap_or_default().to_string();
    let invalid = || PailuneError::Invalid(format!("not a valid nxm link: {shown}"));
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

/// Accepts only links as the website's "Mod Manager Download" button makes
/// them: with a download key and expiry. A page that merely crafts an
/// `nxm://` URL cannot produce a valid key, so it cannot trigger downloads.
pub fn validate(link: &NxmLink, domain: &str) -> Result<()> {
    match (link.domain == domain, link.key.as_deref(), link.expires) {
        (false, _, _) => Err(PailuneError::Invalid(format!("nxm link is for '{}', not this game", link.domain))),
        (true, Some(key), Some(_)) if !key.is_empty() => Ok(()),
        _ => Err(PailuneError::Invalid(
            "ignored an nxm link without a download key — use the website's \"Mod Manager Download\" button".into(),
        )),
    }
}

/// A collection link: `nxm://<domain>/collections/<slug>/revisions/<n>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionLink {
    pub domain: String,
    pub slug: String,
    pub revision: Option<u32>,
}

/// Parses a collection link, if `url` is one.
pub fn parse_collection(url: &str) -> Option<CollectionLink> {
    let rest = url.trim().strip_prefix("nxm://")?;
    let path = rest.split('?').next()?.trim_end_matches('/');
    match path.split('/').collect::<Vec<_>>().as_slice() {
        [domain, "collections", slug] => Some(CollectionLink {
            domain: domain.to_lowercase(),
            slug: slug.to_lowercase(),
            revision: None,
        }),
        [domain, "collections", slug, "revisions", rev] => Some(CollectionLink {
            domain: domain.to_lowercase(),
            slug: slug.to_lowercase(),
            revision: rev.parse().ok(),
        }),
        _ => None,
    }
    .filter(|link| link.slug.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Accepts a bare id or a `nexusmods.com/<game>/mods/<id>` URL.
pub fn parse_mod_id(input: &str) -> Result<u64> {
    let trimmed = input.trim();
    let tail = trimmed.split("/mods/").nth(1).unwrap_or(trimmed);
    tail.split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|digits| digits.parse().ok())
        .ok_or_else(|| PailuneError::Invalid(format!("'{trimmed}' is not a Nexus mod id or URL")))
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
        assert_eq!((bare.key.as_deref(), bare.expires), (None, None));
        assert!(parse("nxm://crimsondesert/mods/x/files/2").is_err());
        assert!(parse("https://nexusmods.com").is_err());
        assert!(validate(&link, "crimsondesert").is_ok());
        assert!(
            validate(&bare, "crimsondesert").is_err(),
            "no key, no download"
        );
        assert!(validate(&link, "skyrim").is_err());
        let err = parse("nxm://x/mods/a/files/1?key=SECRET")
            .unwrap_err()
            .to_string();
        assert!(!err.contains("SECRET"));
    }

    #[test]
    fn mod_ids_from_urls() {
        assert_eq!(parse_mod_id(" 42 ").unwrap(), 42);
        assert_eq!(
            parse_mod_id("https://www.nexusmods.com/crimsondesert/mods/77?tab=files").unwrap(),
            77
        );
        assert!(parse_mod_id("abc").is_err());
    }
}
