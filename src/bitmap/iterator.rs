//! Bitmap iterator implementation
//!
//! This module provides iteration over compressed bitmap data.

use crate::wah::WahIterator;

/// Iterator over bitmap bits
#[derive(Debug)]
pub struct BitmapIterator<'a> {
    wah_iter: WahIterator<'a>,
    position: u64,
    total_bits: u64,
}

impl<'a> BitmapIterator<'a> {
    /// Create a new bitmap iterator
    pub fn new(wah_iter: WahIterator<'a>, total_bits: u64) -> Self {
        Self {
            wah_iter,
            position: 0,
            total_bits,
        }
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get remaining bits
    pub fn remaining(&self) -> u64 {
        if self.total_bits > self.position {
            self.total_bits - self.position
        } else {
            0
        }
    }

    /// Skip ahead by n bits
    pub fn skip_bits(&mut self, n: u64) -> bool {
        for _ in 0..n {
            if self.next().is_none() {
                return false;
            }
        }
        true
    }
}

impl<'a> Iterator for BitmapIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.total_bits {
            return None;
        }

        let bit = self.wah_iter.next()?;
        self.position += 1;
        Some(bit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining() as usize;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BitmapIterator<'a> {
    fn len(&self) -> usize {
        self.remaining() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wah::{WahEncoder, WahDecoder};

    #[test]
    fn test_bitmap_iterator_basic() {
        // Create some test data
        let mut encoder = WahEncoder::new();
        encoder.append_bits(&[0b10101010], 8).unwrap();
        let words = encoder.finish().unwrap();
        
        let decoder = WahDecoder::new(words);
        let wah_iter = decoder.iter();
        let mut bitmap_iter = BitmapIterator::new(wah_iter, 8);
        
        assert_eq!(bitmap_iter.len(), 8);
        assert_eq!(bitmap_iter.position(), 0);
        
        // Should have some bits
        let first_bit = bitmap_iter.next();
        assert!(first_bit.is_some());
        assert_eq!(bitmap_iter.position(), 1);
        assert_eq!(bitmap_iter.len(), 7);
    }

    #[test]
    fn test_bitmap_iterator_skip() {
        let mut encoder = WahEncoder::new();
        encoder.fill(true, 100).unwrap();
        let words = encoder.finish().unwrap();
        
        let decoder = WahDecoder::new(words);
        let wah_iter = decoder.iter();
        let mut bitmap_iter = BitmapIterator::new(wah_iter, 100);
        
        assert_eq!(bitmap_iter.skip_bits(50), true);
        assert_eq!(bitmap_iter.position(), 50);
        assert_eq!(bitmap_iter.len(), 50);
    }
}