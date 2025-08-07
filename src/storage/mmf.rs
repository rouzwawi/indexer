//! Memory-mapped file management
//!
//! This module provides memory-mapped file functionality replacing boost::interprocess.

use crate::types::{BitmapResult, BitmapError, PageId, PAGE_SIZE, INITIAL_FILE_SIZE};
use memmap2::{MmapMut, MmapOptions};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Memory-mapped file manager
pub struct MemoryMappedFile {
    base_path: PathBuf,
    file: File,
    mappings: HashMap<PageId, MmapMut>,
    next_page: AtomicU32,
    file_size: u64,
}

impl MemoryMappedFile {
    /// Create or open a memory-mapped file
    pub fn new<P: AsRef<Path>>(path: P) -> BitmapResult<Self> {
        let base_path = path.as_ref().to_path_buf();
        let data_file = base_path.join("bitmap_data.bin");
        
        // Create directory if it doesn't exist
        if let Some(parent) = data_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open or create the file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&data_file)?;

        // Get current file size or initialize
        let metadata = file.metadata()?;
        let file_size = if metadata.len() == 0 {
            file.set_len(INITIAL_FILE_SIZE)?;
            INITIAL_FILE_SIZE
        } else {
            metadata.len()
        };

        Ok(Self {
            base_path,
            file,
            mappings: HashMap::new(),
            next_page: AtomicU32::new(1), // Page 0 reserved for header
            file_size,
        })
    }

    /// Allocate a new page and return its ID
    pub fn allocate_page(&mut self) -> BitmapResult<PageId> {
        let page_id = self.next_page.fetch_add(1, Ordering::SeqCst);
        let required_size = (page_id as u64 + 1) * PAGE_SIZE as u64;
        
        // Expand file if necessary
        if required_size > self.file_size {
            let new_size = self.file_size * 2; // Exponential growth
            self.file.set_len(new_size)?;
            self.file_size = new_size;
        }

        Ok(page_id)
    }

    /// Get a mutable reference to a page
    pub fn get_page_mut(&mut self, page_id: PageId) -> BitmapResult<&mut [u8]> {
        if !self.mappings.contains_key(&page_id) {
            let offset = page_id as u64 * PAGE_SIZE as u64;
            let mmap = unsafe {
                MmapOptions::new()
                    .offset(offset)
                    .len(PAGE_SIZE)
                    .map_mut(&self.file)
                    .map_err(|e| BitmapError::MemoryMapping {
                        message: format!("Failed to map page {}: {}", page_id, e),
                    })?
            };
            self.mappings.insert(page_id, mmap);
        }

        self.mappings
            .get_mut(&page_id)
            .map(|mmap| mmap.as_mut())
            .ok_or(BitmapError::PageNotFound { page_id })
    }

    /// Sync all mappings to disk
    pub fn sync(&self) -> BitmapResult<()> {
        for mmap in self.mappings.values() {
            mmap.flush().map_err(|e| BitmapError::MemoryMapping {
                message: format!("Failed to sync mapping: {}", e),
            })?;
        }
        Ok(())
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get current file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get next page ID that would be allocated
    pub fn next_page_id(&self) -> PageId {
        self.next_page.load(Ordering::SeqCst)
    }
}

impl Drop for MemoryMappedFile {
    fn drop(&mut self) {
        let _ = self.sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mmf_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mmf = MemoryMappedFile::new(temp_dir.path());
        assert!(mmf.is_ok());
    }

    #[test]
    fn test_page_allocation() {
        let temp_dir = TempDir::new().unwrap();
        let mut mmf = MemoryMappedFile::new(temp_dir.path()).unwrap();
        
        let page1 = mmf.allocate_page().unwrap();
        let page2 = mmf.allocate_page().unwrap();
        
        assert_ne!(page1, page2);
        assert!(page1 > 0); // Page 0 is reserved
        assert!(page2 > 0);
    }
}