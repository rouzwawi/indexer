//! Storage layer for memory-mapped files and file system operations.
//!
//! This module provides the foundation for persistent storage using memory-mapped
//! files with automatic expansion and a hash-based file system for organization.

pub mod filesystem;
pub mod mmf;

pub use filesystem::FileSystem;
pub use mmf::MemoryMappedFile;
