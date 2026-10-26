//! Crimson Desert's archive formats (read side), per `docs/paz-format.md`.
//!
//! The game reads content only from PAZ archives indexed by per-group
//! `0.pamt` files, which `meta/0.papgt` registers. This module parses those
//! structures and extracts entries (ChaCha20 + LZ4). It never writes to the
//! game directory. The overlay writer (roadmap phase 3) builds on it.
#![allow(dead_code)]

pub mod crypto;
pub mod dds;
pub mod encode;
pub mod extract;
pub mod hash;
pub mod pamt;
pub mod pamt_check;
#[allow(dead_code)] // used by the in-place route (D-14), not wired yet
pub mod pamt_edit;
pub mod papgt;
pub mod pathc;
pub mod paver;
mod reader;
pub mod status;
pub mod vanilla;
pub mod write;

#[cfg(test)]
mod machine;
#[cfg(test)]
mod machine_overlay;
#[cfg(test)]
mod machine_tables;
#[cfg(test)]
mod machine_validate;

/// Compression of an archive entry (flags bits 0–3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    /// "Partial": DDS with per-mip LZ4 chunks; also used by `.pac` meshes
    /// (`PAR ` header), most of them stored uncompressed.
    DdsPartial,
    Lz4,
    /// A value the format documentation has never observed.
    Unknown(u8),
}

/// Encryption of an archive entry (flags bits 4–7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    None,
    ChaCha20,
    Unknown(u8),
}

impl Compression {
    /// Compression from an entry's flags (bits 0–3).
    pub fn from_flags(flags: u16) -> Self {
        match flags & 0x0F {
            0 => Compression::None,
            1 => Compression::DdsPartial,
            2 => Compression::Lz4,
            other => Compression::Unknown(other as u8),
        }
    }
}

impl Encryption {
    /// Encryption from an entry's flags (bits 4–7).
    pub fn from_flags(flags: u16) -> Self {
        match (flags >> 4) & 0x0F {
            0 => Encryption::None,
            3 => Encryption::ChaCha20,
            other => Encryption::Unknown(other as u8),
        }
    }
}
