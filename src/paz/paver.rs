//! `meta/0.paver`: game version plus checksum (read only, never written).

use super::hash::pa_hash;
use super::reader::Reader;
use crate::error::{PailuneError, Result};

/// The installed game build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub checksum: u32,
}

/// Parses and verifies the 10-byte version file.
pub fn parse(data: &[u8]) -> Result<GameVersion> {
    let mut r = Reader::new(data);
    let version = GameVersion {
        major: r.u16()?,
        minor: r.u16()?,
        patch: r.u16()?,
        checksum: r.u32()?,
    };
    match pa_hash(&data[..6]) == version.checksum {
        true => Ok(version),
        false => Err(PailuneError::Archive("0.paver checksum mismatch".into())),
    }
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_version_2_3_2() {
        let data = [2, 0, 3, 0, 2, 0, 0xE3, 0xCC, 0x19, 0xCB];
        let v = super::parse(&data).unwrap();
        assert_eq!(v.to_string(), "2.3.2");
        assert!(super::parse(&[2, 0, 3, 0, 2, 0, 0, 0, 0, 0]).is_err());
    }
}
