//! Type definitions and error types for the bitmap indexer
//!
//! This module provides type aliases and error handling types that replace
//! the C++ typedefs.hpp functionality.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Type aliases (replacing C++ typedefs)
/// 32-bit unsigned integer (replaces u4 in C++)
pub type U32 = u32;

/// 64-bit unsigned integer (replaces u8 in C++)
pub type U64 = u64;

/// Page identifier type
pub type PageId = u32;

/// File identifier type
pub type FileId = u32;

/// Word type for WAH compression
pub type WahWord = u64;

// Constants (replacing C++ #defines)
/// Page size in bytes (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Initial file size (8MB)
pub const INITIAL_FILE_SIZE: u64 = 8 * 1024 * 1024;

/// Maximum fill count for WAH compression
pub const MAX_FILL_COUNT: u32 = 0x3FFF_FFFF;

/// WAH word size in bits
pub const WAH_WORD_BITS: usize = 64;

/// Bits per byte
pub const BITS_PER_BYTE: usize = 8;

// Error types
#[derive(Error, Debug)]
pub enum BitmapError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Memory mapping error: {message}")]
    MemoryMapping { message: String },

    #[error("Page not found: {page_id}")]
    PageNotFound { page_id: PageId },

    #[error("File not found: {name}")]
    FileNotFound { name: String },

    #[error("File already exists: {name}")]
    FileAlreadyExists { name: String },

    #[error("Invalid WAH word: {word:#018x}")]
    InvalidWahWord { word: WahWord },

    #[error("Bitmap operation failed: {message}")]
    BitmapOperation { message: String },

    #[error("Hash collision in file system")]
    HashCollision,

    #[error("Insufficient space: need {needed} bytes, have {available}")]
    InsufficientSpace { needed: usize, available: usize },

    #[error("Invalid bitmap header")]
    InvalidHeader,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("SHA1 computation error: {message}")]
    Sha1Error { message: String },
}

/// Result type alias for bitmap operations
pub type BitmapResult<T> = Result<T, BitmapError>;

// Utility traits and types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHeader {
    /// Magic number for file format validation
    pub magic: u32,
    /// File format version
    pub version: u32,
    /// Total number of pages
    pub page_count: u32,
    /// Next available page ID
    pub next_page: u32,
    /// Root hash table page
    pub root_hash_page: u32,
    /// Reserved for future use
    pub reserved: [u32; 3],
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            magic: 0x42495458, // "BITX"
            version: 1,
            page_count: 0,
            next_page: 1, // Page 0 is reserved for header
            root_hash_page: 0,
            reserved: [0; 3],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitmapHeader {
    /// Number of bits in the bitmap
    pub bit_count: u64,
    /// Number of WAH words used
    pub word_count: u32,
    /// First page containing bitmap data
    pub first_page: PageId,
    /// Last page containing bitmap data
    pub last_page: PageId,
    /// Checksum for integrity verification
    pub checksum: u32,
    /// Reserved for future use
    pub reserved: [u32; 3],
}

impl Default for BitmapHeader {
    fn default() -> Self {
        Self {
            bit_count: 0,
            word_count: 0,
            first_page: 0,
            last_page: 0,
            checksum: 0,
            reserved: [0; 3],
        }
    }
}

/// Hash table entry for file system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    /// SHA1 hash of the file name
    pub hash: [u8; 20],
    /// File name
    pub name: String,
    /// File ID
    pub file_id: FileId,
    /// Next entry in collision chain
    pub next: Option<PageId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_header_default() {
        let header = FileHeader::default();
        assert_eq!(header.magic, 0x42495458);
        assert_eq!(header.version, 1);
        assert_eq!(header.next_page, 1);
    }

    #[test]
    fn test_bitmap_header_default() {
        let header = BitmapHeader::default();
        assert_eq!(header.bit_count, 0);
        assert_eq!(header.word_count, 0);
    }

    #[test]
    fn test_error_display() {
        let error = BitmapError::PageNotFound { page_id: 42 };
        assert_eq!(error.to_string(), "Page not found: 42");
    }
}