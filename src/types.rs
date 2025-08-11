//! Type definitions and error handling for the bitmap indexer.
//!
//! This module provides constants, error types, and other type definitions
//! for the bitmap indexer, replacing the C++ typedefs.hpp.

use thiserror::Error;

/// Page size constant (4 KiB pages)
pub const PAGE_SIZE: usize = 4096;

/// Initial memory mapping size (8 MiB)
pub const REGION_SIZE: usize = 8 * 1024 * 1024;

/// Pages per region (2048 pages)
pub const PAGES_PER_REGION: usize = REGION_SIZE / PAGE_SIZE;

/// Maximum file name length for the hash-based file system
pub const MAX_FILENAME_LENGTH: usize = 255;

/// WAH compression constants
pub mod wah {
    /// Number of data bits in a literal word (63)
    pub const WORD_LENGTH: u32 = 63;

    /// Fill flag bit mask (bit 63)
    pub const FILL_FLAG: u64 = 0x8000_0000_0000_0000;

    /// Fill value bit mask (bit 62)
    pub const FILL_VAL: u64 = 0x4000_0000_0000_0000;

    /// Combined mask for fill flag and value (bits 63..62)
    pub const FILL_FV: u64 = 0xC000_0000_0000_0000;

    /// Canonical fill words for value 0 and 1 (with zero counts)
    pub const FILL_0: u64 = 0x8000_0000_0000_0000;
    pub const FILL_1: u64 = 0xC000_0000_0000_0000;

    /// Fill count bits (low 31 bits)
    pub const FILL_BITS: u64 = 0x0000_0000_7FFF_FFFF;
    /// Backwards-compat alias for maximum fill count
    pub const MAX_FILL_COUNT: u64 = FILL_BITS;

    /// Literal-count bits (bits 61..31)
    pub const LTRL_BITS: u64 = 0x3FFF_FFFF_8000_0000;

    /// Data bits for literal words (low 63 bits)
    pub const DATA_BITS: u64 = 0x7FFF_FFFF_FFFF_FFFF;
}

/// File system constants
pub mod fs {
    /// Magic word for filesystem identification
    pub const MAGIC_WORD: u32 = u32::from_le_bytes(*b"idxr");

    /// Offset within page where hash table starts
    pub const TABLE_OFFSET: usize = 64;

    /// Number of entries per hash table page
    pub const TABLE_ENTRIES: usize = 125;

    /// Parent pointer location in page header
    pub const HEAD_PARENT: usize = 1;

    /// Flag indicating filled hash entry
    pub const HE_FILLED: u32 = 0x00000001;

    /// Maximum collision resolution attempts
    pub const MAX_COLLISION_ATTEMPTS: u32 = 16;

    /// Number of in-page probes to attempt before chaining
    pub const PROBES_PER_PAGE: u32 = 16;

    /// Invalid page marker
    pub const INVALID_PAGE: u32 = 0xFFFF_FFFF;

    /// Size of hash entry in bytes
    pub const HASH_ENTRY_SIZE: usize = std::mem::size_of::<crate::storage::filesystem::HashEntry>();
}

// ===== Compile-time layout guarantees =====
// Ensure hash-table data starts at an offset aligned for `HashEntry`
const _: [(); 1] = [(); (fs::TABLE_OFFSET
    % core::mem::align_of::<crate::storage::filesystem::HashEntry>()
    == 0) as usize];
// Ensure the `HashEntry` layout size matches expectations (32 bytes)
const _: [(); 1] = [(); (fs::HASH_ENTRY_SIZE == 32) as usize];
// Ensure the table fits within a single page
const _: [(); 1] =
    [(); (fs::TABLE_OFFSET + fs::TABLE_ENTRIES * fs::HASH_ENTRY_SIZE <= PAGE_SIZE) as usize];

/// Result type alias for bitmap indexer operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the bitmap indexer
#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Memory mapping error: {0}")]
    MemoryMapping(String),

    #[error("Invalid page number: {0}")]
    InvalidPage(u32),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Hash collision: unable to resolve after {0} levels")]
    HashCollision(u32),

    #[error("Invalid bitmap operation: {0}")]
    InvalidOperation(String),

    #[error("Insufficient space: {0}")]
    InsufficientSpace(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(PAGE_SIZE, 4096);
        assert_eq!(REGION_SIZE, 8 * 1024 * 1024);
        assert_eq!(PAGES_PER_REGION, 2048);
        assert!(MAX_FILENAME_LENGTH > 0);
    }

    #[test]
    fn test_wah_constants() {
        use wah::*;
        assert_eq!(FILL_FLAG, 0x8000_0000_0000_0000);
        assert_eq!(FILL_VAL, 0x4000_0000_0000_0000);
        // In the C++ layout we follow, fill count uses 31 bits
        assert_eq!(MAX_FILL_COUNT, 0x7FFF_FFFF);
    }

    #[test]
    fn test_error_types() {
        let error = Error::InvalidPage(42);
        assert!(error.to_string().contains("42"));
    }

    #[test]
    fn test_fs_constants() {
        assert_eq!(fs::MAGIC_WORD, 0x72786469);
        assert_eq!(fs::TABLE_OFFSET, 64);
        assert_eq!(fs::TABLE_ENTRIES, 125);
        assert_eq!(fs::HEAD_PARENT, 1);
        assert_eq!(fs::HASH_ENTRY_SIZE, 32);
    }
}
