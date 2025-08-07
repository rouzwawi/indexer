//! WAH (Word-Aligned Hybrid) compression module
//!
//! This module implements WAH compression for efficient bitmap storage.
//! WAH compression uses 64-bit words with a special format:
//! - Fill words: compress runs of identical bits
//! - Literal words: store uncompressible data

pub mod compression;

pub use compression::{WahWord, WahEncoder, WahDecoder, WahIterator};