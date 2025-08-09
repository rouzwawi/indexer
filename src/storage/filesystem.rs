//! Hash-based file system for organizing bitmap data.
//!
//! This module provides a file system abstraction using SHA1-based hashing
//! for efficient file organization and retrieval.

use crate::types::{Result, Error, fs::*};
use crate::hash::sha1::Sha1Hash;
use crate::storage::MemoryMappedFile;
use std::ops::{Index, IndexMut};

/// Hash entry stored in hash table pages (32 bytes total)
#[repr(C)]
#[derive(Debug)]
pub struct HashEntry {
    /// SHA1 hash as 5 32-bit words (20 bytes)
    pub sha1: [u32; 5],
    /// Page number where file data starts (4 bytes)
    pub file_page: u32,
    /// Next hash table page for collision resolution (4 bytes)
    pub next_table: u32,
    /// Flags including HE_FILLED (4 bytes)
    pub flags: u32,
}

impl HashEntry {
    /// Create a new empty hash entry
    pub fn new() -> Self {
        Self {
            sha1: [0; 5],
            file_page: 0,
            next_table: 0,
            flags: 0,
        }
    }

    /// Check if this entry is filled
    pub fn is_filled(&self) -> bool {
        (self.flags & HE_FILLED) != 0
    }

    /// Mark this entry as filled
    pub fn set_filled(&mut self) {
        self.flags |= HE_FILLED;
    }

    /// Check if SHA1 matches
    pub fn matches_sha1(&self, sha1: &[u32; 5]) -> bool {
        self.sha1 == *sha1
    }

    /// Copy SHA1 into this entry
    pub fn copy_sha1(&mut self, sha1: &[u32; 5]) {
        self.sha1.copy_from_slice(sha1);
    }
}

/// A hash table
pub struct HashTableRef<'a> {
    /// The page number of the hash table
    pub page: u32,
    /// The hash table data
    pub data: &'a [HashEntry],
}

/// A hash table
pub struct HashTableMut<'a> {
    /// The page number of the hash table
    pub page: u32,
    /// The hash table data
    pub data: &'a mut [HashEntry],
}

impl<'a> HashTableRef<'a> {
    /// Create a new hash table
    pub fn new(page: u32, data: &'a [HashEntry]) -> Self {
        Self {
            page,
            data,
        }
    }

    /// Get the length of the hash table
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<'a> HashTableMut<'a> {
    /// Create a new hash table
    pub fn new(page: u32, data: &'a mut [HashEntry]) -> Self {
        Self {
            page,
            data,
        }
    }

    /// Get the length of the hash table
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<'a> Index<usize> for HashTableRef<'a> {
    type Output = HashEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<'a> Index<usize> for HashTableMut<'a> {
    type Output = HashEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<'a> IndexMut<usize> for HashTableMut<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

/// Hash-based file system for bitmap storage
pub struct FileSystem {
    /// Memory-mapped file for storage
    storage: MemoryMappedFile,
}

impl FileSystem {
    /// Create a new file system from existing storage
    pub fn new(mut storage: MemoryMappedFile) -> Result<Self> {
        // Verify the file is initialized
        let page_0 = storage.get_page(0)?;
        let magic_word = u32::from_le_bytes([page_0[0], page_0[1], page_0[2], page_0[3]]);
        if magic_word != MAGIC_WORD {
            return Err(Error::InvalidFormat("File not initialized".to_string()));
        }

        Ok(Self { storage })
    }

    /// Initialize a new file system
    pub fn init(mut storage: MemoryMappedFile) -> Result<Self> {
        // Make sure file is empty
        if storage.allocated_pages() != 0 {
            return Err(Error::InvalidOperation("File not empty".to_string()));
        }

        // Allocate the 0th page for origin table
        let page_0 = storage.allocate_page()?;
        if page_0 != 0 {
            return Err(Error::InvalidOperation("Expected page 0".to_string()));
        }

        let page_ptr = storage.get_page_mut(page_0)?;
        Self::init_table(page_ptr, u32::MAX)?;
        storage.sync_page(page_0)?;

        Self::new(storage)
    }

    /// Get the page number for a file
    pub fn get_file_page(&mut self, filename: &str) -> Result<u32> {
        let sha1_hash = Sha1Hash::from_string(filename);
        let sha1 = sha1_hash.as_words();

        match self.find_table_entry(sha1, false) {
            Ok((page, index)) => {
                let hash_table = self.get_hash_table(page)?;
                let entry = &hash_table[index];
                if entry.is_filled() {
                    Ok(entry.file_page)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Ok(0),
        }
    }

    /// Create a new file
    pub fn create_file(&mut self, filename: &str) -> Result<u32> {
        let sha1_hash = Sha1Hash::from_string(filename);
        let sha1 = sha1_hash.as_words();

        // This will effectively delete any old file with this name by rewriting its page address
        let (page, index) = self.find_table_entry(sha1, true)?;
        let file_page = self.storage.allocate_page()?;

        {
            let mut hash_table_mut = self.get_hash_table_mut(page)?;
            let entry = &mut hash_table_mut[index];
            entry.file_page = file_page;
            entry.set_filled();
            entry.copy_sha1(sha1);
        }

        self.storage.sync_page(page)?;
        Ok(file_page)
    }

    /// Check if a file exists
    pub fn has_file(&mut self, filename: &str) -> Result<bool> {
        Ok(self.get_file_page(filename)? != 0)
    }

    /// Get page data for reading
    pub fn get_page_data(&mut self, page_num: u32) -> Result<&[u8]> {
        self.storage.get_page(page_num)
    }

    /// Get mutable page data for writing
    pub fn get_page_data_mut(&mut self, page_num: u32) -> Result<&mut [u8]> {
        self.storage.get_page_mut(page_num)
    }

    /// Sync a page to ensure it's written to disk
    pub fn sync_page(&mut self, page_num: u32) -> Result<()> {
        self.storage.sync_page(page_num)
    }

    /// Get hash table as a slice from a page
    fn get_hash_table(&mut self, page: u32) -> Result<HashTableRef> {
        let page_data = self.storage.get_page(page)?;
        Self::verify_page_initialized(page_data)?;

        let table_data = &page_data[TABLE_OFFSET..];

        // Ensure we have enough data for the hash table
        if table_data.len() < TABLE_ENTRIES * HASH_ENTRY_SIZE {
            return Err(Error::InvalidFormat("Page too small for hash table".to_string()));
        }

        // SAFETY: We've verified the slice is large enough and HashEntry has repr(C)
        // The memory layout is compatible with the C++ struct
        unsafe {
            let ptr = table_data.as_ptr() as *const HashEntry;
            let data = std::slice::from_raw_parts(ptr, TABLE_ENTRIES);
            Ok(HashTableRef::new(page, data))
        }
    }

    /// Get mutable hash table as a slice from a page
    fn get_hash_table_mut(&mut self, page: u32) -> Result<HashTableMut> {
        let page_data = self.storage.get_page_mut(page)?;
        Self::verify_page_initialized(page_data)?;

        let table_data = &mut page_data[TABLE_OFFSET..];

        // Ensure we have enough data for the hash table
        if table_data.len() < TABLE_ENTRIES * HASH_ENTRY_SIZE {
            return Err(Error::InvalidFormat("Page too small for hash table".to_string()));
        }

        // SAFETY: We've verified the slice is large enough and HashEntry has repr(C)
        // The memory layout is compatible with the C++ struct
        unsafe {
            let ptr = table_data.as_mut_ptr() as *mut HashEntry;
            let data = std::slice::from_raw_parts_mut(ptr, TABLE_ENTRIES);
            Ok(HashTableMut::new(page, data))
        }
    }

    /// Find a table entry, optionally allocating new tables for collisions
    fn find_table_entry(&mut self, sha1: &[u32; 5], allocate: bool) -> Result<(u32, usize)> {
        let mut current_page = 0;
        let mut jumps = 0;

        loop {
            let h1 = Self::table_hash_h1(sha1);
            let h2 = Self::table_hash_h2(sha1);
            let hash_table = self.get_hash_table(current_page)?;

            // Probe multiple slots within this page first
            for i in 0..PROBES_PER_PAGE {
                let idx = ((h1.wrapping_add(h2.wrapping_mul(i as u32))) as usize) % TABLE_ENTRIES;
                let e = &hash_table[idx];
                if e.is_filled() {
                    if e.matches_sha1(sha1) {
                        return Ok((current_page, idx));
                    }
                } else {
                    return Ok((current_page, idx));
                }
            }

            // No in-page slot; follow or create chain via a stable anchor
            let chain_idx = (h1 as usize) % TABLE_ENTRIES;
            let entry = &hash_table[chain_idx];
            if entry.next_table == 0 {
                if !allocate {
                    return Err(Error::FileNotFound("File not found".to_string()));
                }

                // Crash-consistent order: (1) create and initialize new page, (2) link from current page
                let next_table_page = self.storage.allocate_page()?;

                // Initialize and persist the new table page first
                let new_page_ptr = self.storage.get_page_mut(next_table_page)?;
                Self::init_table(new_page_ptr, current_page)?;
                self.storage.sync_page(next_table_page)?;

                // Now publish the link from the current page and persist it
                {
                    let mut hash_table = self.get_hash_table_mut(current_page)?;
                    hash_table[chain_idx].next_table = next_table_page;
                    self.storage.sync_page(current_page)?;
                }

                current_page = next_table_page;
            } else {
                current_page = entry.next_table;
            }

            jumps += 1;
            if jumps >= MAX_COLLISION_ATTEMPTS {
                return Err(Error::HashCollision);
            }
        }
    }

    /// Initialize a hash table page
    fn init_table(page_ptr: &mut [u8], parent: u32) -> Result<()> {
        // Make sure page starts empty (allow for reinitialization)
        if page_ptr.len() < 4 {
            return Err(Error::InvalidOperation("Page too small".to_string()));
        }

        // Write magic word
        page_ptr[0..4].copy_from_slice(&MAGIC_WORD.to_le_bytes());

        // Write parent pointer
        let parent_offset = HEAD_PARENT * 4;
        if parent_offset + 4 <= page_ptr.len() {
            page_ptr[parent_offset..parent_offset + 4].copy_from_slice(&parent.to_le_bytes());
        }

        // Clear hash table area
        if TABLE_OFFSET + TABLE_ENTRIES * HASH_ENTRY_SIZE <= page_ptr.len() {
            let table_area = &mut page_ptr[TABLE_OFFSET..TABLE_OFFSET + TABLE_ENTRIES * HASH_ENTRY_SIZE];
            table_area.fill(0);
        }

        Ok(())
    }

    /// Verify that a page is initialized
    fn verify_page_initialized(page_data: &[u8]) -> Result<()> {
        if page_data.len() < 4 {
            return Err(Error::InvalidFormat("Page too small for hash table".to_string()));
        }
        let magic_word = u32::from_le_bytes([page_data[0], page_data[1], page_data[2], page_data[3]]);
        if magic_word != MAGIC_WORD {
            return Err(Error::InvalidFormat("Page not initialized".to_string()));
        }
        Ok(())
    }

    /// Primary mixed hash (h1) for double hashing within a page
    #[inline]
    fn table_hash_h1(sha1: &[u32; 5]) -> u32 {
        let mut h = sha1[0]
            ^ sha1[1].rotate_left(5)
            ^ sha1[2].rotate_left(11)
            ^ sha1[3].rotate_left(17)
            ^ sha1[4].rotate_left(23);
        h ^= h >> 16;
        h = h.wrapping_mul(0x7FEB_352D);
        h ^= h >> 15;
        h = h.wrapping_mul(0x846C_A68B);
        h ^= h >> 16;
        h
    }

    /// Secondary hash (h2) for double hashing; ensure odd to walk table
    #[inline]
    fn table_hash_h2(sha1: &[u32; 5]) -> u32 {
        let mut h = sha1[0].wrapping_mul(0x9E37_79B9)
            ^ sha1[2].rotate_left(9)
            ^ sha1[4].wrapping_mul(0x85EB_CA6B);
        h ^= h >> 16;
        h |= 1;
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::mem;

    #[test]
    fn test_hash_entry_size() {
        assert_eq!(mem::size_of::<HashEntry>(), HASH_ENTRY_SIZE);
    }

    #[test]
    fn test_init_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let storage = MemoryMappedFile::new(&mmf_path).unwrap();
        let _fs = FileSystem::init(storage).unwrap();
    }

    #[test]
    fn test_create_and_find_file() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let storage = MemoryMappedFile::new(&mmf_path).unwrap();
        let mut fs = FileSystem::init(storage).unwrap();

        // Create a file
        let file_page = fs.create_file("test.bitmap").unwrap();
        assert!(file_page > 0);

        // Find the file
        let found_page = fs.get_file_page("test.bitmap").unwrap();
        assert_eq!(found_page, file_page);

        // Check if file exists
        assert!(fs.has_file("test.bitmap").unwrap());
        assert!(!fs.has_file("nonexistent.bitmap").unwrap());
    }

    #[test]
    fn test_file_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let storage = MemoryMappedFile::new(&mmf_path).unwrap();
        let mut fs = FileSystem::init(storage).unwrap();

        // Create a file
        let file_page1 = fs.create_file("test.bitmap").unwrap();

        // Create the same file again (should replace)
        let file_page2 = fs.create_file("test.bitmap").unwrap();
        assert_ne!(file_page1, file_page2);

        // Should find the new page
        let found_page = fs.get_file_page("test.bitmap").unwrap();
        assert_eq!(found_page, file_page2);
    }

    #[test]
    fn test_collision_handling() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let storage = MemoryMappedFile::new(&mmf_path).unwrap();
        let mut fs = FileSystem::init(storage).unwrap();

        // Create multiple files to force collisions
        let mut file_pages = Vec::new();
        for i in 0..200 {
            let filename = format!("file_{:03}.bitmap", i);
            let page = fs.create_file(&filename).unwrap();
            file_pages.push((filename, page));
        }

        // Verify all files can be found
        for (filename, expected_page) in file_pages {
            let found_page = fs.get_file_page(&filename).unwrap();
            assert_eq!(found_page, expected_page, "File {} not found correctly", filename);
        }
    }

    #[test]
    fn test_filesystem_reopen() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        // Create filesystem and add files
        {
            let storage = MemoryMappedFile::new(&mmf_path).unwrap();
            let mut fs = FileSystem::init(storage).unwrap();

            fs.create_file("persistent.bitmap").unwrap();
            fs.create_file("another.bitmap").unwrap();
        }

        // Reopen filesystem and verify files exist
        {
            let storage = MemoryMappedFile::new(&mmf_path).unwrap();
            let mut fs = FileSystem::new(storage).unwrap();

            assert!(fs.has_file("persistent.bitmap").unwrap());
            assert!(fs.has_file("another.bitmap").unwrap());
            assert!(!fs.has_file("nonexistent.bitmap").unwrap());
        }
    }

    #[test]
    fn test_binary_format_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let storage = MemoryMappedFile::new(&mmf_path).unwrap();
        let mut fs = FileSystem::init(storage).unwrap();

        // Verify magic word is written correctly
        let page_0 = fs.storage.get_page(0).unwrap();
        let magic_word = u32::from_le_bytes([page_0[0], page_0[1], page_0[2], page_0[3]]);
        assert_eq!(magic_word, MAGIC_WORD);

        // Verify parent pointer location
        let parent_offset = HEAD_PARENT * 4;
        let parent = u32::from_le_bytes([
            page_0[parent_offset],
            page_0[parent_offset + 1],
            page_0[parent_offset + 2],
            page_0[parent_offset + 3],
        ]);
        assert_eq!(parent, u32::MAX); // Root table has no parent

        // Create a file and verify hash entry format
        let filename = "test.bitmap";
        let file_page = fs.create_file(filename).unwrap();

        // Get the hash table and verify entry
        let sha1_hash = Sha1Hash::from_string(filename);
        let sha1 = sha1_hash.as_words();
        let h1 = FileSystem::table_hash_h1(sha1);
        let h2 = FileSystem::table_hash_h2(sha1);
        let index = ((h1.wrapping_add(h2.wrapping_mul(0))) as usize) % TABLE_ENTRIES;

        let hash_table = fs.get_hash_table(0).unwrap();
        let entry = &hash_table[index];
        assert!(entry.is_filled());
        assert_eq!(entry.file_page, file_page);
        assert_eq!(entry.sha1, *sha1);
        assert_eq!(entry.flags & HE_FILLED, HE_FILLED);
    }

    #[test]
    fn test_sha1_word_format() {
        let test_string = "hello";
        let hash = Sha1Hash::from_string(test_string);
        let words = hash.as_words();
        let bytes = hash.as_bytes();

        // Verify that words and bytes represent the same hash
        for i in 0..5 {
            let expected_word = u32::from_be_bytes([
                bytes[i * 4],
                bytes[i * 4 + 1],
                bytes[i * 4 + 2],
                bytes[i * 4 + 3],
            ]);
            assert_eq!(words[i], expected_word);
        }
    }

    #[test]
    fn test_direct_memory_mapping_performance() {
        let temp_dir = TempDir::new().unwrap();
        let mmf_path = temp_dir.path().join("test.mmf");

        let storage = MemoryMappedFile::new(&mmf_path).unwrap();
        let mut fs = FileSystem::init(storage).unwrap();

        // Create many files to test performance
        let file_count = 1000;
        let mut file_names = Vec::new();

        for i in 0..file_count {
            let filename = format!("file_{:04}.bitmap", i);
            fs.create_file(&filename).unwrap();
            file_names.push(filename);
        }

        // Verify all files can be found quickly
        for filename in &file_names {
            let page = fs.get_file_page(filename).unwrap();
            assert!(page > 0, "File {} not found", filename);
        }

        // Test that we can access hash tables directly
        let hash_table = fs.get_hash_table(0).unwrap();
        assert_eq!(hash_table.len(), TABLE_ENTRIES);

        // Verify we can get mutable access
        let hash_table_mut = fs.get_hash_table_mut(0).unwrap();
        assert_eq!(hash_table_mut.len(), TABLE_ENTRIES);
    }
}
