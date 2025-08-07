//! Bitmap operations and iteration
//!
//! This module provides:
//! - Bitmap writing with WAH compression
//! - Bitmap reading and iteration
//! - Efficient append and fill operations

pub mod bitmap;
pub mod iterator;

pub use bitmap::{BitmapWriter, BitmapReader};
pub use iterator::BitmapIterator;