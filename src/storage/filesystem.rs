//! Hash-based file system for bitmap organization
//!
//! This module provides file system functionality using SHA1 hashes for file identification.

use crate::hash::{sha1_string, Sha1Hash};
use crate::storage::MemoryMappedFile;
use crate::types::{BitmapResult, BitmapError, FileId, HashEntry};
use std::collections::HashMap;

/// Hash-based file system
pub struct FileSystem {
    hash_table: HashMap<Sha1Hash, HashEntry>,
    next_file_id: FileId,
}

impl FileSystem {
    /// Create a new file system
    pub fn new(_storage: &MemoryMappedFile) -> BitmapResult<Self> {
        // TODO: Load existing hash table from storage
        Ok(Self {
            hash_table: HashMap::new(),
            next_file_id: 1,
        })
    }

    /// Create a new file with the given name
    pub fn create_file(&mut self, name: &str) -> BitmapResult<FileId> {
        let hash = sha1_string(name);
        
        // Check if file already exists
        if self.hash_table.contains_key(&hash) {
            return Err(BitmapError::FileAlreadyExists {
                name: name.to_string(),
            });
        }

        let file_id = self.next_file_id;
        self.next_file_id += 1;

        let entry = HashEntry {
            hash,
            name: name.to_string(),
            file_id,
            next: None,
        };

        self.hash_table.insert(hash, entry);
        Ok(file_id)
    }

    /// Find a file by name
    pub fn find_file(&self, name: &str) -> BitmapResult<FileId> {
        let hash = sha1_string(name);
        
        self.hash_table
            .get(&hash)
            .map(|entry| entry.file_id)
            .ok_or_else(|| BitmapError::FileNotFound {
                name: name.to_string(),
            })
    }

    /// List all files
    pub fn list_files(&self) -> BitmapResult<Vec<String>> {
        Ok(self.hash_table.values().map(|entry| entry.name.clone()).collect())
    }

    /// Delete a file by name
    pub fn delete_file(&mut self, name: &str) -> BitmapResult<()> {
        let hash = sha1_string(name);
        
        self.hash_table
            .remove(&hash)
            .ok_or_else(|| BitmapError::FileNotFound {
                name: name.to_string(),
            })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_filesystem() -> (FileSystem, MemoryMappedFile) {
        let temp_dir = TempDir::new().unwrap();
        let storage = MemoryMappedFile::new(temp_dir.path()).unwrap();
        let fs = FileSystem::new(&storage).unwrap();
        (fs, storage)
    }

    #[test]
    fn test_create_file() {
        let (mut fs, _storage) = create_test_filesystem();
        
        let file_id = fs.create_file("test.bitmap").unwrap();
        assert!(file_id > 0);
        
        // Should fail to create duplicate
        let result = fs.create_file("test.bitmap");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_file() {
        let (mut fs, _storage) = create_test_filesystem();
        
        let file_id = fs.create_file("test.bitmap").unwrap();
        let found_id = fs.find_file("test.bitmap").unwrap();
        assert_eq!(file_id, found_id);
        
        // Should fail to find non-existent file
        let result = fs.find_file("nonexistent.bitmap");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_files() {
        let (mut fs, _storage) = create_test_filesystem();
        
        fs.create_file("file1.bitmap").unwrap();
        fs.create_file("file2.bitmap").unwrap();
        
        let files = fs.list_files().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"file1.bitmap".to_string()));
        assert!(files.contains(&"file2.bitmap".to_string()));
    }

    #[test]
    fn test_delete_file() {
        let (mut fs, _storage) = create_test_filesystem();
        
        fs.create_file("test.bitmap").unwrap();
        assert!(fs.find_file("test.bitmap").is_ok());
        
        fs.delete_file("test.bitmap").unwrap();
        assert!(fs.find_file("test.bitmap").is_err());
    }
}