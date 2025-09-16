//! # Bitmap Indexer
//!
//! A WAH (Word-Aligned Hybrid) compressed bitmap indexing system for efficient data indexing and retrieval.
//!
//! This library provides:
//! - Memory-mapped file management with automatic region allocation
//! - Hash-based file system for organizing indexed data
//! - WAH compression for space-efficient bitmap storage
//! - Bitmap operations (append, fill, iteration)
//! - SHA1-based file identification
//!
//! ## Example
//!
//! ```rust,no_run
//! use bitmap_indexer::BitmapIndex;
//!
//! // Open or create a bitmap index
//! let mut index = BitmapIndex::open("data/index").expect("Failed to open index");
//!
//! // Create a new bitmap
//! let mut writer = index.create_bitmap("my_bitmap").expect("Failed to create bitmap");
//!
//! // Write some data
//! writer.fill(true, 1000).expect("Failed to fill bitmap");
//! writer.append_bits(&[0xFF, 0x00], 16).expect("Failed to append bits");
//!
//! // Read the bitmap
//! let reader = index.open_bitmap("my_bitmap").expect("Failed to open bitmap");
//! for bit in reader.take(10) {
//!     println!("Bit: {}", bit);
//! }
//! ```

pub mod bitmap;
pub mod csv;
pub mod hash;
pub mod storage;
pub mod types;
pub mod wah;

pub use bitmap::{BitmapIndex, BitmapReader, BitmapWriter};
pub use types::{Error, Result};

/// Version information for the bitmap indexer
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
