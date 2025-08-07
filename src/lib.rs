//! # Bitmap Indexer
//!
//! A WAH (Word-Aligned Hybrid) compressed bitmap indexing system designed for efficient
//! data indexing and retrieval.
//!
//! ## Features
//!
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
//! let mut index = BitmapIndex::open("./data")?;
//!
//! // Create a new bitmap
//! let mut writer = index.create_bitmap("example")?;
//! writer.append_bits(&[0b10101010], 8)?;
//! writer.fill(true, 1000)?;
//!
//! // Read the bitmap
//! let reader = index.open_bitmap("example")?;
//! for bit in reader.take(10) {
//!     println!("Bit: {}", bit);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use anyhow::Result;
use std::path::Path;

// Public modules
pub mod types;
pub mod wah;
pub mod storage;
pub mod bitmap;
pub mod hash;

// Re-export main types for convenience
pub use bitmap::{BitmapReader, BitmapWriter};
pub use storage::{MemoryMappedFile, FileSystem};
pub use types::{PageId, FileId, BitmapError};

/// Main entry point for the bitmap indexing system
pub struct BitmapIndex {
    storage: MemoryMappedFile,
    filesystem: FileSystem,
}

impl BitmapIndex {
    /// Open an existing bitmap index or create a new one at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let storage = MemoryMappedFile::new(&path)?;
        let filesystem = FileSystem::new(&storage)?;
        
        Ok(Self {
            storage,
            filesystem,
        })
    }

    /// Create a new bitmap writer for the specified name
    pub fn create_bitmap(&mut self, name: &str) -> Result<BitmapWriter> {
        let file_id = self.filesystem.create_file(name)?;
        BitmapWriter::new(&mut self.storage, file_id)
    }

    /// Open an existing bitmap for reading
    pub fn open_bitmap(&self, name: &str) -> Result<BitmapReader> {
        let file_id = self.filesystem.find_file(name)?;
        BitmapReader::new(&self.storage, file_id)
    }

    /// List all available bitmap names
    pub fn list_bitmaps(&self) -> Result<Vec<String>> {
        Ok(self.filesystem.list_files()?)
    }

    /// Delete a bitmap by name
    pub fn delete_bitmap(&mut self, name: &str) -> Result<()> {
        Ok(self.filesystem.delete_file(name)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bitmap_index_creation() {
        let temp_dir = TempDir::new().unwrap();
        let index = BitmapIndex::open(temp_dir.path());
        assert!(index.is_ok());
    }
}