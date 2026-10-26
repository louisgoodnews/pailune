//! How packed files are stored in Pylune's archive group.

/// The flags a packed file is stored with. Compression type 1 ("partial")
/// is written for textures only; other files using it in the game (`.pac`
/// meshes) are stored plain, as the game itself stores some meshes
/// (`docs/paz-format.md` §7.4). Encryption is kept.
pub fn storage_flags(vpath: &str, inherited: u16) -> u16 {
    match (vpath.ends_with(".dds"), inherited & 0x0F) {
        (false, 1) => inherited & !0x0F,
        _ => inherited,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_storage_is_for_textures_only() {
        assert_eq!(storage_flags("ui/a.dds", 0x0001), 0x0001);
        assert_eq!(storage_flags("character/model/x.pac", 0x0001), 0x0000);
        assert_eq!(storage_flags("character/model/x.pac", 0x0031), 0x0030);
        assert_eq!(storage_flags("ui/a.xml", 0x0032), 0x0032);
    }
}
