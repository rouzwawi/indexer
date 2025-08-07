//! Storage layer for memory-mapped files and file system management
//!
//! This module provides:
//! - Memory-mapped file management with automatic expansion
//! - Hash-based file system for organizing bitmap files
//! - Page-based allocation and management

pub mod mmf;
pub mod filesystem;

pub use mmf::MemoryMappedFile;
pub use filesystem::FileSystem;