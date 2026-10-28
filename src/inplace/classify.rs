//! Classifies files that require the in-place route (D-14).
//!
//! Files that the engine reads outside overlays ("overlay-blind"):
//! - Dialogue and cutscene audio (`.wem`, `.bnk`, `.pck`, or under `sound/`/`audio/`).
//! - Weapon and socket placement XML (`gamedata/character/weapon/*.xml`, `gamedata/item/socket/*.xml`).
//!
//! Strict boundary: Data tables (`.pabgb`, `.pabgh`, `.staticinfobody`, `.staticinfoheader`,
//! `.paseq`, `.paseqc`) NEVER go in place and always use the overlay route.

/// Checks whether a normalized virtual path is an in-place deployment candidate.
pub fn is_inplace_candidate(vpath: &str) -> bool {
    let lower = vpath.to_lowercase().replace('\\', "/");

    // Scope boundary: Data tables ALWAYS use overlay route, NEVER in-place.
    if lower.ends_with(".pabgb")
        || lower.ends_with(".pabgh")
        || lower.ends_with(".staticinfobody")
        || lower.ends_with(".staticinfoheader")
        || lower.ends_with(".paseq")
        || lower.ends_with(".paseqc")
    {
        return false;
    }

    // Audio files: .wem, .bnk, .pck or sound/audio paths
    if lower.ends_with(".wem") || lower.ends_with(".bnk") || lower.ends_with(".pck") {
        return true;
    }
    if lower.starts_with("sound/")
        || lower.starts_with("audio/")
        || lower.contains("/sound/")
        || lower.contains("/audio/")
    {
        return true;
    }

    // Weapon / socket placement XML:
    if (lower.starts_with("gamedata/item/socket/")
        || lower.starts_with("gamedata/character/weapon/"))
        && lower.ends_with(".xml")
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_audio_files_as_inplace() {
        assert!(is_inplace_candidate("sound/dialogue/en/hero.wem"));
        assert!(is_inplace_candidate("audio/banks/general.bnk"));
        assert!(is_inplace_candidate("sound/pack01.pck"));
        assert!(is_inplace_candidate("cutscene/dialogue/vo_001.wem"));
    }

    #[test]
    fn classifies_weapon_and_socket_xml_as_inplace() {
        assert!(is_inplace_candidate(
            "gamedata/character/weapon/sword_01.xml"
        ));
        assert!(is_inplace_candidate("gamedata/item/socket/gem_slot.xml"));
    }

    #[test]
    fn excludes_data_tables_from_inplace() {
        assert!(!is_inplace_candidate(
            "gamedata/binarystaticinfo__/bin/iteminfo.pabgb"
        ));
        assert!(!is_inplace_candidate(
            "gamedata/binarystaticinfo__/bin/iteminfo.pabgh"
        ));
        assert!(!is_inplace_candidate("gamedata/sound/table.staticinfobody"));
        assert!(!is_inplace_candidate(
            "gamedata/character/weapon/data.pabgb"
        ));
    }

    #[test]
    fn excludes_regular_ui_and_mesh_files() {
        assert!(!is_inplace_candidate("ui/xml/gamemain/play/view.html"));
        assert!(!is_inplace_candidate("ui/theme.css"));
        assert!(!is_inplace_candidate("character/model.pac"));
        assert!(!is_inplace_candidate("character/texture.dds"));
        assert!(!is_inplace_candidate("ui/dialogue/menu.xml"));
    }
}
