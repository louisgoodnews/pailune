//! Crimson Desert's archive formats (read side), per `docs/paz-format.md`.
//!
//! The game reads content only from PAZ archives indexed by per-group
//! `0.pamt` files, which `meta/0.papgt` registers. This module parses those
//! structures and extracts entries (ChaCha20 + LZ4). It never writes to the
//! game directory. The overlay writer (roadmap phase 3) builds on it.
#![allow(dead_code)]

pub mod crypto;
pub mod extract;
pub mod hash;
pub mod pamt;
pub mod papgt;
pub mod paver;
mod reader;
pub mod status;

#[cfg(test)]
mod machine;

/// Compression of an archive entry (flags bits 0–3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    /// DDS with per-mip LZ4 chunks ("partial").
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
    pub fn from_flags(flags: u16) -> Self {
        match (flags >> 4) & 0x0F {
            0 => Encryption::None,
            3 => Encryption::ChaCha20,
            other => Encryption::Unknown(other as u8),
        }
    }
}
