//! Bitmap operations and management.
//!
//! This module provides the main bitmap indexing functionality including
//! bitmap creation, writing, reading, and iteration.

pub mod bitmap;
pub mod iterator;

pub use bitmap::{
    header_from_page_bytes, BitmapHeaderView, BitmapIndex, BitmapReader, BitmapWriter,
};
