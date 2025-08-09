//! Storage layer for memory-mapped files and file system operations.
//!
//! This module provides the foundation for persistent storage using memory-mapped
//! files with automatic expansion and a hash-based file system for organization.

pub mod mmf;
pub mod filesystem;

pub use mmf::MemoryMappedFile;
pub use filesystem::FileSystem;
