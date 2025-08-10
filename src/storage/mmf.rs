//! Memory-mapped file implementation.
//!
//! This module replaces the C++ boost::interprocess memory mapping with a safe
//! Rust implementation using the memmap2 crate.
//!
//! ## Architecture Overview
//!
//! The memory-mapped file system uses a hierarchical structure to efficiently manage
//! large amounts of data with automatic expansion:
//!
//! ```text
//! File Series → Files → Regions → Pages
//! ```
//!
//! ## File Series Structure
//!
//! The system organizes data into a series of files with exponentially growing sizes:
//!
//! | File | Regions | Size (MiB) | Total Size (MiB) |
//! |------|---------|------------|------------------|
//! | 0    | 1       | 8          | 8                |
//! | 1    | 2       | 16         | 24               |
//! | 2    | 4       | 32         | 56               |
//! | 3    | 8       | 64         | 120              |
//! | 4    | 16      | 128        | 248              |
//! | 5    | 32      | 256        | 504              |
//! | 6    | 64      | 512        | 1,016            |
//! | 7+   | 128     | 1,024      | 1,016 + n*1,024  |
//!
//! ### Key Properties:
//!
//! - **Files 0-6**: Exponential growth (2^file regions)
//! - **Files 7+**: Fixed size (128 regions each)
//! - **Base region size**: 8 MiB (INITIAL_MAPPING_SIZE)
//! - **Pages per region**: 2,048 (8 MiB / 4 KiB)
//!
//! ## Region Addressing
//!
//! Regions are numbered sequentially across all files:
//!
//! ```text
//! File 0: Region 0
//! File 1: Regions 1-2
//! File 2: Regions 3-6
//! File 3: Regions 7-14
//! File 4: Regions 15-30
//! File 5: Regions 31-62
//! File 6: Regions 63-126
//! File 7: Regions 127-254
//! File 8: Regions 255-382
//! ...
//! ```
//!
//! ## Page Addressing
//!
//! Pages are numbered sequentially starting from 0:
//!
//! ```text
//! Region 0: Pages 0-2,047
//! Region 1: Pages 2,048-4,095
//! Region 2: Pages 4,096-6,143
//! ...
//! ```
//!
//! ## Address Translation Functions
//!
//! The system provides several key functions for address translation:
//!
//! - `file_regions(file)`: Number of regions in a file
//! - `regions_up_to(file)`: Total regions in files 0 through file-1
//! - `file_addr(region)`: Which file contains a given region
//! - `get_region_for_page(page)`: Which region contains a given page
//! - `get_page_offset_in_region(page)`: Offset within region for a page
//!
//! ## Memory Efficiency
//!
//! This design provides excellent memory efficiency:
//!
//! 1. **Lazy Loading**: Regions are only mapped when accessed
//! 2. **Exponential Growth**: Early files are small, later files are large
//! 3. **Page Granularity**: 4 KiB pages provide fine-grained access
//! 4. **Automatic Expansion**: New files/regions created as needed
//!
//! ## Persistence and Index File
//!
//! The system maintains persistent state through an index file (`.idx` extension):
//!
//! - **Automatic Creation**: Index file created on first use
//! - **State Persistence**: Next page counter saved across restarts
//! - **Crash Recovery**: State restored when reopening the same path
//! - **Auto-sync**: Index updated on every page allocation
//! - **Manual Sync**: `sync_index()` for explicit persistence
//! - **Direct Memory Access**: Counter accessed via raw pointers like C++ implementation
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! # use bitmap_indexer::storage::MemoryMappedFile;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut mmf = MemoryMappedFile::new("data/bitmap.mmf")?;
//!
//! // Allocate pages (automatically persisted)
//! let page1 = mmf.allocate_page()?; // Returns 0
//! let page2 = mmf.allocate_page()?; // Returns 1
//!
//! // Access page data
//! let data = mmf.get_page_mut(page1)?;
//! data[0] = 0x42;
//!
//! // Manual sync if needed
//! mmf.sync_index()?;
//!
//! // Get statistics
//! let stats = mmf.stats();
//! println!("Allocated pages: {}", stats.allocated_pages);
//! println!("Active regions: {}", stats.active_regions);
//! # Ok(())
//! # }
//! ```
//!
//! When reopened, the system will restore the previous state:
//!
//! ```rust,no_run
//! # use bitmap_indexer::storage::MemoryMappedFile;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Later, in a new process...
//! let mut mmf2 = MemoryMappedFile::new("data/bitmap.mmf")?;
//! let page3 = mmf2.allocate_page()?; // Returns 2 (continues from previous state)
//! # Ok(())
//! # }
//! ```
//!
//! ## Detailed Documentation
//!
//! For comprehensive documentation of the memory-mapped file system architecture,
//! including mathematical relationships, performance characteristics, and usage
//! patterns, see `doc/MEMORY_MAPPED_FILESYSTEM.md`.

use crate::types::{Error, Result, PAGES_PER_REGION, PAGE_SIZE, REGION_SIZE};
use memmap2::{MmapMut, MmapOptions};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

/// File mapping structure that holds a complete file mapping
struct FileMapping {
    /// The actual memory mapping covering the entire file
    mapping: MmapMut,
    /// File handle (kept alive for the mapping)
    _file: File,
    /// Size of this file in bytes
    #[allow(dead_code)]
    file_size: u64,
    /// Number of regions in this file
    region_count: u32,
}

/// Memory-mapped file manager with automatic expansion
///
/// This implementation is designed for single-threaded use and provides direct access
/// to memory-mapped regions without synchronization overhead.
pub struct MemoryMappedFile {
    /// Base path for the memory-mapped files
    base_path: PathBuf,
    /// Index file memory mapping for persistent metadata
    /// The first 4 bytes contain the next available page number
    /// MUST be declared before next_page_ptr to ensure proper drop order
    index_mapping: MmapMut,
    /// Raw pointer to the next page counter in the index mapping, viewed as an AtomicU32.
    /// Equivalent to C++'s: u32* next_empty_page
    ///
    /// SAFETY: This pointer is valid for the entire lifetime of this struct
    /// because index_mapping is owned by this struct and declared before this field.
    /// Rust's drop order guarantees the mapping stays valid.
    next_page_ptr: *mut AtomicU32,
    /// Whole-file mappings indexed by file number
    file_mappings: HashMap<u32, FileMapping>,

    /// Marker to make this type !Send and !Sync (single-threaded by design)
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl MemoryMappedFile {
    /// Create a new memory-mapped file manager
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base_path = path.as_ref().to_path_buf();

        // Ensure the directory exists
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create and initialize the index file mapping
        let mut index_mapping = Self::create_index_mapping(&base_path)?;

        // Get the raw pointer to the next page counter (first 4 bytes)
        // SAFETY: The pointer is valid as long as index_mapping is alive,
        // and we store both the mapping and pointer together in the struct.
        // The mmap is page-aligned, so the first 4 bytes are suitably aligned for AtomicU32.
        let next_page_ptr = index_mapping.as_mut_ptr() as *mut AtomicU32;

        Ok(Self {
            base_path,
            index_mapping,
            next_page_ptr,
            file_mappings: HashMap::new(),
            _not_send_or_sync: PhantomData,
        })
    }

    /// Allocate a new page and return its page number
    ///
    /// This method directly manipulates the memory-mapped counter using the stored raw pointer,
    /// exactly like the legacy C++ implementation where `next_empty_page` is a direct
    /// pointer to the memory-mapped location.
    pub fn allocate_page(&mut self) -> Result<u32> {
        let current_page = self.increment_next_page()?;

        // Ensure we have a mapping for this page
        let region_id = region_for_page(current_page);
        let file_num = file_addr(region_id);
        self.ensure_file_mapping(file_num)?;

        Ok(current_page)
    }

    /// Get a mutable slice for a specific page
    pub fn get_page_mut(&mut self, page_num: u32) -> Result<&mut [u8]> {
        // Bounds check to ensure the page has been allocated
        if page_num >= self.allocated_pages() {
            return Err(Error::InvalidPage(page_num));
        }

        // Calculate which file contains this page
        let region_id = region_for_page(page_num);
        let file_num = file_addr(region_id);

        // Ensure whole file is mapped
        let file_mapping = self.ensure_file_mapping(file_num)?;

        // Calculate offset within the entire file
        let regions_before_file = regions_up_to(file_num);
        let region_offset_in_file = region_id - regions_before_file;
        let page_offset_in_region = page_offset_in_region(page_num);

        let byte_offset = (region_offset_in_file * PAGES_PER_REGION as u32 + page_offset_in_region)
            as usize
            * PAGE_SIZE;
        let end_offset = byte_offset + PAGE_SIZE;

        // Return slice directly from file mapping
        file_mapping
            .mapping
            .get_mut(byte_offset..end_offset)
            .ok_or_else(|| Error::InvalidPage(page_num))
    }

    /// Get a read-only slice for a specific page
    pub fn get_page(&mut self, page_num: u32) -> Result<&[u8]> {
        // Bounds check to ensure the page has been allocated
        if page_num >= self.allocated_pages() {
            return Err(Error::InvalidPage(page_num));
        }

        // Calculate which file contains this page
        let region_id = region_for_page(page_num);
        let file_num = file_addr(region_id);

        // Ensure whole file is mapped
        let file_mapping = self.ensure_file_mapping(file_num)?;

        // Calculate offset within the entire file
        let regions_before_file = regions_up_to(file_num);
        let region_offset_in_file = region_id - regions_before_file;
        let page_offset_in_region = page_offset_in_region(page_num);

        let byte_offset = (region_offset_in_file * PAGES_PER_REGION as u32 + page_offset_in_region)
            as usize
            * PAGE_SIZE;
        let end_offset = byte_offset + PAGE_SIZE;

        // Return slice directly from file mapping
        file_mapping
            .mapping
            .get(byte_offset..end_offset)
            .ok_or_else(|| Error::InvalidPage(page_num))
    }

    /// Ensure a file is mapped, creating it if necessary
    fn ensure_file_mapping(&mut self, file_num: u32) -> Result<&mut FileMapping> {
        if !self.file_mappings.contains_key(&file_num) {
            let file_path = self.base_path.with_extension(format!("d{:04x}", file_num));
            let file_size_bytes = file_size(file_num);

            // Create/ensure file exists with correct size
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&file_path)?;

            if file.metadata()?.len() < file_size_bytes {
                file.set_len(file_size_bytes)?;
            }

            // Map entire file at once
            let mapping = unsafe {
                MmapOptions::new()
                    .len(file_size_bytes as usize)
                    .map_mut(&file)
                    .map_err(|e| {
                        Error::MemoryMapping(format!(
                            "Failed to create mapping for file {}: {}",
                            file_num, e
                        ))
                    })?
            };

            let file_mapping = FileMapping {
                mapping,
                _file: file,
                file_size: file_size_bytes,
                region_count: file_regions(file_num),
            };

            self.file_mappings.insert(file_num, file_mapping);
        }

        Ok(self.file_mappings.get_mut(&file_num).unwrap())
    }

    /// Get the total number of allocated pages
    ///
    /// Reads directly from the memory-mapped counter using the stored raw pointer,
    /// equivalent to dereferencing `next_empty_page` in the C++ version.
    pub fn allocated_pages(&self) -> u32 {
        self.get_next_page_atomic().load(Ordering::Relaxed)
    }

    fn increment_next_page(&mut self) -> Result<u32> {
        // Prevent overflow
        let current = self.allocated_pages();
        if current == u32::MAX {
            return Err(Error::InsufficientSpace(
                "Maximum number of pages reached".to_string(),
            ));
        }

        let prev = self.get_next_page_atomic().fetch_add(1, Ordering::Relaxed);
        self.sync_index()?;

        Ok(prev)
    }

    /// Safe wrapper to access the atomic next page counter
    ///
    /// SAFETY: This is safe because:
    /// - next_page_ptr is valid for the lifetime of this struct
    /// - index_mapping is owned by this struct and declared before next_page_ptr
    /// - The memory is properly aligned for AtomicU32 operations
    /// - We only access through this controlled interface
    fn get_next_page_atomic(&self) -> &AtomicU32 {
        unsafe { &*self.next_page_ptr }
    }

    /// Sync a specific page to disk
    pub fn sync_page(&mut self, page_num: u32) -> Result<()> {
        // Bounds check to ensure the page has been allocated
        if page_num >= self.allocated_pages() {
            return Err(Error::InvalidPage(page_num));
        }

        let region_id = region_for_page(page_num);
        let file_num = file_addr(region_id);

        // // If the file is mapped, flush it to disk
        if let Some(file_mapping) = self.file_mappings.get_mut(&file_num) {
            file_mapping.mapping.flush().map_err(|e| {
                Error::MemoryMapping(format!("Failed to sync page {}: {}", page_num, e))
            })?;
        }

        Ok(())
    }

    /// Sync all mappings to disk
    pub fn sync_all(&self) -> Result<()> {
        for file_mapping in self.file_mappings.values() {
            file_mapping
                .mapping
                .flush()
                .map_err(|e| Error::MemoryMapping(format!("Failed to sync: {}", e)))?;
        }

        self.sync_index()?;

        Ok(())
    }

    /// Manually sync the index file to disk.
    ///
    /// This is automatically called on page allocation, but can be called
    /// manually for explicit persistence.
    pub fn sync_index(&self) -> Result<()> {
        self.index_mapping
            .flush()
            .map_err(|e| Error::MemoryMapping(format!("Failed to flush index: {}", e)))?;
        Ok(())
    }

    /// Get statistics about memory usage
    pub fn stats(&self) -> MemoryStats {
        let total_mapped = self
            .file_mappings
            .values()
            .map(|fm| fm.mapping.len())
            .sum::<usize>();

        // Calculate active regions from active files
        let active_regions = self
            .file_mappings
            .values()
            .map(|fm| fm.region_count as usize)
            .sum::<usize>();

        MemoryStats {
            active_regions,
            allocated_pages: self.allocated_pages(),
            total_mapped_bytes: total_mapped,
            current_region_size: REGION_SIZE,
        }
    }

    /// Create and initialize the index file memory mapping.
    ///
    /// The index file stores persistent metadata including the next available
    /// page number. This ensures state is preserved across restarts.
    fn create_index_mapping(base_path: &Path) -> Result<MmapMut> {
        let index_path = base_path.with_extension("idx");

        // Create index file if it doesn't exist (64 bytes, matching legacy implementation)
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&index_path)?;

        // Set file size to 64 bytes
        file.set_len(64)?;

        // Create memory mapping
        let mmap = unsafe {
            MmapOptions::new()
                .len(64)
                .map_mut(&file)
                .map_err(|e| Error::MemoryMapping(format!("Failed to map index file: {}", e)))?
        };

        Ok(mmap)
    }
}

impl Drop for MemoryMappedFile {
    /// Ensure index is saved when the MMF is dropped
    fn drop(&mut self) {
        // Best effort to sync index on drop - ignore errors since we can't propagate them
        let _ = self.sync_all();
    }
}

/// Address translation functions for the memory-mapped file system.
/// These functions implement the mathematical relationships between files,
/// regions, and pages as described in the module documentation.

/// Calculate the total size of a file in bytes.
///
/// Files grow exponentially from 0-6, then have fixed size:
/// - File 0: 1 region × 8 MiB = 8 MiB
/// - File 1: 2 regions × 8 MiB = 16 MiB
/// - File 2: 4 regions × 8 MiB = 32 MiB
/// - ...
/// - File 6: 64 regions × 8 MiB = 512 MiB
/// - File 7+: 128 regions × 8 MiB = 1024 MiB
pub fn file_size(file: u32) -> u64 {
    let regions = file_regions(file);
    regions as u64 * REGION_SIZE as u64
}

/// Calculate the number of regions in a given file.
///
/// Uses exponential growth for files 0-6, then fixed size:
/// - Files 0-6: 2^file regions (1, 2, 4, 8, 16, 32, 64)
/// - Files 7+: 128 regions each
pub fn file_regions(file: u32) -> u32 {
    if file >= 7 {
        128 // Fixed size for files 7 and above
    } else {
        1 << file // Exponential growth: 2^file
    }
}

/// Calculate the total number of regions in all files before the given file.
///
/// This is used to determine the starting region number for a file:
/// - File 0: starts at region 0 (0 regions before)
/// - File 1: starts at region 1 (1 region before)
/// - File 2: starts at region 3 (1+2 = 3 regions before)
/// - File 3: starts at region 7 (1+2+4 = 7 regions before)
/// - etc.
pub fn regions_up_to(file: u32) -> u32 {
    if file == 0 {
        0 // No regions before file 0
    } else if file <= 7 {
        // Sum of geometric series: 1 + 2 + 4 + ... + 2^(file-1) = 2^file - 1
        (1 << file) - 1
    } else {
        // Regions in files 0-6: 2^7 - 1 = 127
        // Plus regions in files 7 through (file-1): 128 * (file - 7)
        127 + 128 * (file - 7)
    }
}

/// Get the region ID for a given page number.
///
/// Pages are numbered starting from 1, with each region containing
/// PAGES_PER_REGION pages (typically 2,048 pages = 8 MiB / 4 KiB).
///
/// Examples:
/// - Page 0 → Region 0
/// - Page 2,047 → Region 0
/// - Page 2,048 → Region 1
/// - Page 4,095 → Region 1
/// - Page 4,096 → Region 2
pub fn region_for_page(page_num: u32) -> u32 {
    page_num / PAGES_PER_REGION as u32
}

/// Get the page index within its containing region.
///
/// Returns the page index (0-based) within the region.
/// Each region contains PAGES_PER_REGION pages.
pub fn page_offset_in_region(page_num: u32) -> u32 {
    page_num % PAGES_PER_REGION as u32
}

/// Determine which file contains the given region.
pub fn file_addr(region: u32) -> u32 {
    if region == 0 {
        return 0; // Region 0 is always in file 0
    }

    let mut file = 0;
    // Find the largest file number where regions_up_to(file+1) <= region
    while regions_up_to(file + 1) <= region {
        file += 1;
    }
    file
}

/// Statistics about memory-mapped file usage
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub active_regions: usize,
    pub allocated_pages: u32,
    pub total_mapped_bytes: usize,
    pub current_region_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_mmf() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let mmf = MemoryMappedFile::new(&mmf_path).unwrap();
        assert_eq!(mmf.allocated_pages(), 0);
    }

    #[test]
    fn test_allocate_page() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let mut mmf = MemoryMappedFile::new(&mmf_path).unwrap();
        let page0 = mmf.allocate_page().unwrap();
        let page1 = mmf.allocate_page().unwrap();

        assert_eq!(page0, 0);
        assert_eq!(page1, 1);
        assert_eq!(mmf.allocated_pages(), 2);
    }

    #[test]
    fn test_page_access() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let mut mmf = MemoryMappedFile::new(&mmf_path).unwrap();
        let page_0 = mmf.allocate_page().unwrap();

        // Write to page
        {
            let page = mmf.get_page_mut(page_0).unwrap();
            page[0] = 0x42;
            page[1] = 0x24;
        }

        // Read from page
        {
            let page = mmf.get_page(page_0).unwrap();
            assert_eq!(page[0], 0x42);
            assert_eq!(page[1], 0x24);
        }

        let page_1 = mmf.allocate_page().unwrap();
        {
            let page = mmf.get_page_mut(page_1).unwrap();
            assert_eq!(page[0], 0x0);
            assert_eq!(page[1], 0x0);
            page[0] = 0x56;
            page[1] = 0x65;
        }

        {
            let page = mmf.get_page(page_1).unwrap();
            assert_eq!(page[0], 0x56);
            assert_eq!(page[1], 0x65);
        }
    }

    #[test]
    fn test_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let mut mmf = MemoryMappedFile::new(&mmf_path).unwrap();
        let _page0 = mmf.allocate_page().unwrap();
        let _page1 = mmf.allocate_page().unwrap();

        let stats = mmf.stats();
        assert_eq!(stats.allocated_pages, 2);
        assert!(stats.total_mapped_bytes > 0);
    }

    #[test]
    fn test_index_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        // Create MMF and allocate some pages
        {
            let mut mmf = MemoryMappedFile::new(&mmf_path).unwrap();
            assert_eq!(mmf.allocated_pages(), 0);

            let page0 = mmf.allocate_page().unwrap();
            let page1 = mmf.allocate_page().unwrap();
            let page2 = mmf.allocate_page().unwrap();

            assert_eq!(page0, 0);
            assert_eq!(page1, 1);
            assert_eq!(page2, 2);
            assert_eq!(mmf.stats().allocated_pages, 3);
            assert_eq!(mmf.allocated_pages(), 3);

            // Manually sync to ensure data is written
            mmf.sync_index().unwrap();
        } // MMF is dropped here, should auto-save

        // Verify index file exists
        let index_path = mmf_path.with_extension("idx");
        assert!(index_path.exists());

        // Create new MMF with same path - should restore state
        {
            let mut mmf2 = MemoryMappedFile::new(&mmf_path).unwrap();
            assert_eq!(mmf2.allocated_pages(), 3); // Should restore previous count

            // Next allocation should continue from where we left off
            let page3 = mmf2.allocate_page().unwrap();
            assert_eq!(page3, 3);
            assert_eq!(mmf2.allocated_pages(), 4);
        }
    }

    #[test]
    fn test_new_index_file() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("new_test.mmf");

        // Ensure index file doesn't exist initially
        let index_path = mmf_path.with_extension("idx");
        assert!(!index_path.exists());

        let mut mmf = MemoryMappedFile::new(&mmf_path).unwrap();

        // Should start from page 0 for new index
        assert_eq!(mmf.allocated_pages(), 0);
        let page0 = mmf.allocate_page().unwrap();
        assert_eq!(page0, 0);

        // Index file should now exist
        assert!(index_path.exists());
    }

    #[test]
    fn test_addr_calcs() {
        // Port the address calculation tests from legacy C++ code
        assert_eq!(PAGE_SIZE, 0x1000); // 4 KiB
        assert_eq!(REGION_SIZE, 0x800000); // 8 MiB
        assert_eq!(PAGES_PER_REGION, 2048);

        // Test file size calculations
        assert_eq!(file_size(0), 0x800000);
        assert_eq!(file_size(1), 0x1000000);
        assert_eq!(file_size(2), 0x2000000);
        assert_eq!(file_size(3), 0x4000000);
        assert_eq!(file_size(4), 0x8000000);
        assert_eq!(file_size(5), 0x10000000);
        assert_eq!(file_size(6), 0x20000000);
        assert_eq!(file_size(7), 0x40000000);
        assert_eq!(file_size(8), 0x40000000);

        // Test region calculations
        assert_eq!(file_regions(0), 1);
        assert_eq!(file_regions(1), 2);
        assert_eq!(file_regions(2), 4);
        assert_eq!(file_regions(3), 8);
        assert_eq!(file_regions(4), 16);
        assert_eq!(file_regions(5), 32);
        assert_eq!(file_regions(6), 64);
        assert_eq!(file_regions(7), 128);
        assert_eq!(file_regions(8), 128);

        // Test regions up to calculations
        assert_eq!(regions_up_to(0), 0);
        assert_eq!(regions_up_to(1), 1);
        assert_eq!(regions_up_to(2), 3);
        assert_eq!(regions_up_to(3), 7);
        assert_eq!(regions_up_to(4), 15);
        assert_eq!(regions_up_to(5), 31);
        assert_eq!(regions_up_to(6), 63);
        assert_eq!(regions_up_to(7), 127);
        assert_eq!(regions_up_to(8), 255);

        // Test file address calculations
        assert_eq!(file_addr(0), 0);
        assert_eq!(file_addr(1), 1);
        assert_eq!(file_addr(2), 1);
        assert_eq!(file_addr(3), 2);
        assert_eq!(file_addr(6), 2);
        assert_eq!(file_addr(7), 3);
        assert_eq!(file_addr(14), 3);
        assert_eq!(file_addr(15), 4);
        assert_eq!(file_addr(30), 4);
        assert_eq!(file_addr(31), 5);
        assert_eq!(file_addr(62), 5);
        assert_eq!(file_addr(63), 6);
        assert_eq!(file_addr(126), 6);
        assert_eq!(file_addr(127), 7);
        assert_eq!(file_addr(254), 7);
        assert_eq!(file_addr(255), 8);
        assert_eq!(file_addr(382), 8);
        assert_eq!(file_addr(383), 9);

        // Test get_region_for_page
        assert_eq!(region_for_page(0), 0);
        assert_eq!(region_for_page(1), 0);
        assert_eq!(region_for_page(2047), 0);
        assert_eq!(region_for_page(2048), 1);
        assert_eq!(region_for_page(4095), 1);
        assert_eq!(region_for_page(4096), 2);
        assert_eq!(region_for_page(6143), 2);
        assert_eq!(region_for_page(6144), 3);
        assert_eq!(region_for_page(8191), 3);
        assert_eq!(region_for_page(8192), 4);
        assert_eq!(region_for_page(10239), 4);
        assert_eq!(region_for_page(10240), 5);
        assert_eq!(region_for_page(12287), 5);
        assert_eq!(region_for_page(12288), 6);
        assert_eq!(region_for_page(14335), 6);
        assert_eq!(region_for_page(14336), 7);
        assert_eq!(region_for_page(16383), 7);
        assert_eq!(region_for_page(16384), 8);

        // Test get_page_offset_in_region
        assert_eq!(page_offset_in_region(0), 0);
        assert_eq!(page_offset_in_region(1), 1);
        assert_eq!(page_offset_in_region(2), 2);
        assert_eq!(page_offset_in_region(2048), 0);
        assert_eq!(page_offset_in_region(2049), 1);
        assert_eq!(page_offset_in_region(4096), 0);
        assert_eq!(page_offset_in_region(4097), 1);
        assert_eq!(page_offset_in_region(6144), 0);
        assert_eq!(page_offset_in_region(6145), 1);
        assert_eq!(page_offset_in_region(8192), 0);
        assert_eq!(page_offset_in_region(8193), 1);
        assert_eq!(page_offset_in_region(10240), 0);
        assert_eq!(page_offset_in_region(10241), 1);
    }
}
