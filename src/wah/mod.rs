//! WAH (Word-Aligned Hybrid) compression implementation.
//!
//! This module provides efficient bitmap compression using the WAH algorithm,
//! which combines run-length encoding with literal words for optimal compression
//! of sparse bitmaps.

pub mod compression;

pub use compression::{WahWord, WahEncoder, WahDecoder};

use crate::types::Result;

/// WAH compression utilities and operations
pub struct WahCompressor;

impl WahCompressor {
    /// Compress a bitmap using WAH compression
    pub fn compress(bits: &[u8], bit_count: usize) -> Result<Vec<WahWord>> {
        let mut encoder = WahEncoder::new();
        encoder.encode_bits(bits, bit_count)
    }

    /// Decompress a WAH-compressed bitmap
    pub fn decompress(compressed: &[WahWord]) -> Result<Vec<u8>> {
        let mut decoder = WahDecoder::new();
        decoder.decode_words(compressed)
    }

    /// Calculate the compression ratio for a given bitmap
    pub fn compression_ratio(original_bits: usize, compressed_words: usize) -> f64 {
        if compressed_words == 0 {
            return 0.0;
        }

        let original_words = (original_bits + 63) / 64;
        original_words as f64 / compressed_words as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_ratio() {
        let ratio = WahCompressor::compression_ratio(1000, 10);
        assert!(ratio > 1.0); // Should have some compression
    }

    #[test]
    fn test_empty_compression() {
        let ratio = WahCompressor::compression_ratio(0, 0);
        assert_eq!(ratio, 0.0);
    }
}
