//! WAH (Word-Aligned Hybrid) compression implementation
//!
//! This module implements WAH compression for efficient bitmap storage.

use crate::types::{BitmapResult, BitmapError, WahWord as WahWordType, MAX_FILL_COUNT};

/// WAH compressed word wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WahWord(pub WahWordType);

impl WahWord {
    // WAH format constants
    const FILL_FLAG: u64 = 0x8000_0000_0000_0000;  // Bit 63: fill flag
    const FILL_VAL: u64 = 0x4000_0000_0000_0000;   // Bit 62: fill value (0 or 1)
    const COUNT_MASK: u64 = 0x3FFF_FFFF_0000_0000; // Bits 61-32: fill count
    const DATA_MASK: u64 = 0x0000_0000_FFFF_FFFF;  // Bits 31-0: literal data

    /// Create a new literal word
    pub fn literal(data: u32) -> Self {
        Self(data as u64)
    }

    /// Create a new fill word
    pub fn fill(value: bool, count: u32) -> BitmapResult<Self> {
        if count > MAX_FILL_COUNT {
            return Err(BitmapError::BitmapOperation {
                message: format!("Fill count {} exceeds maximum {}", count, MAX_FILL_COUNT),
            });
        }

        let mut word = Self::FILL_FLAG;
        if value {
            word |= Self::FILL_VAL;
        }
        word |= ((count as u64) << 32) & Self::COUNT_MASK;
        
        Ok(Self(word))
    }

    /// Check if this is a fill word
    pub fn is_fill(&self) -> bool {
        (self.0 & Self::FILL_FLAG) != 0
    }

    /// Get fill value (only valid for fill words)
    pub fn fill_value(&self) -> bool {
        (self.0 & Self::FILL_VAL) != 0
    }

    /// Get fill count (only valid for fill words)
    pub fn fill_count(&self) -> u32 {
        ((self.0 & Self::COUNT_MASK) >> 32) as u32
    }

    /// Get literal data (only valid for literal words)
    pub fn literal_data(&self) -> u32 {
        (self.0 & Self::DATA_MASK) as u32
    }

    /// Get raw word value
    pub fn raw(&self) -> WahWordType {
        self.0
    }
}

/// WAH encoder for compressing bitmap data
pub struct WahEncoder {
    words: Vec<WahWordType>,
    pending_literal: Option<u32>,
    bits_in_literal: usize,
}

impl WahEncoder {
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            pending_literal: None,
            bits_in_literal: 0,
        }
    }

    /// Add bits to the encoder
    pub fn append_bits(&mut self, bits: &[u8], bit_count: usize) -> BitmapResult<()> {
        // Simplified implementation - just process bits one by one
        for (i, &byte) in bits.iter().enumerate() {
            let bits_to_process = if i == bits.len() - 1 {
                bit_count - (i * 8).min(bit_count)
            } else {
                8
            };
            
            for bit_pos in 0..bits_to_process {
                let bit = (byte >> bit_pos) & 1 != 0;
                self.append_bit(bit)?;
            }
        }
        Ok(())
    }

    /// Add a single bit
    fn append_bit(&mut self, bit: bool) -> BitmapResult<()> {
        if let Some(ref mut literal) = self.pending_literal {
            *literal |= (bit as u32) << self.bits_in_literal;
            self.bits_in_literal += 1;
            
            if self.bits_in_literal >= 32 {
                self.words.push(WahWord::literal(*literal).raw());
                self.pending_literal = None;
                self.bits_in_literal = 0;
            }
        } else {
            self.pending_literal = Some(bit as u32);
            self.bits_in_literal = 1;
        }
        
        Ok(())
    }

    /// Fill with a value for count bits
    pub fn fill(&mut self, value: bool, count: usize) -> BitmapResult<()> {
        if count == 0 {
            return Ok(());
        }

        // Flush any pending literal
        self.flush_literal()?;

        // Create fill words
        let mut remaining = count;
        while remaining > 0 {
            let chunk = remaining.min(MAX_FILL_COUNT as usize);
            let fill_word = WahWord::fill(value, chunk as u32)?;
            self.words.push(fill_word.raw());
            remaining -= chunk;
        }

        Ok(())
    }

    /// Flush any pending literal word
    fn flush_literal(&mut self) -> BitmapResult<()> {
        if let Some(literal) = self.pending_literal.take() {
            self.words.push(WahWord::literal(literal).raw());
            self.bits_in_literal = 0;
        }
        Ok(())
    }

    /// Finalize encoding and return compressed words
    pub fn finish(mut self) -> BitmapResult<Vec<WahWordType>> {
        self.flush_literal()?;
        Ok(self.words)
    }
}

/// WAH decoder for decompressing bitmap data
pub struct WahDecoder {
    words: Vec<WahWordType>,
}

impl WahDecoder {
    pub fn new(words: Vec<WahWordType>) -> Self {
        Self { words }
    }

    /// Create an iterator over the bits
    pub fn iter(&self) -> WahIterator {
        WahIterator::new(&self.words)
    }
}

/// Iterator over WAH compressed bits
#[derive(Debug)]
pub struct WahIterator<'a> {
    words: &'a [WahWordType],
    word_index: usize,
    current_word: Option<WahWord>,
    bit_position: usize,
    fill_remaining: u32,
    fill_value: bool,
}

impl<'a> WahIterator<'a> {
    pub fn new(words: &'a [WahWordType]) -> Self {
        let mut iter = Self {
            words,
            word_index: 0,
            current_word: None,
            bit_position: 0,
            fill_remaining: 0,
            fill_value: false,
        };
        iter.load_next_word();
        iter
    }

    fn load_next_word(&mut self) {
        if self.word_index >= self.words.len() {
            self.current_word = None;
            return;
        }

        let word = WahWord(self.words[self.word_index]);
        self.word_index += 1;

        if word.is_fill() {
            self.fill_remaining = word.fill_count();
            self.fill_value = word.fill_value();
            self.current_word = None;
        } else {
            self.current_word = Some(word);
            self.bit_position = 0;
            self.fill_remaining = 0;
        }
    }
}

impl<'a> Iterator for WahIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        // Handle fill words
        if self.fill_remaining > 0 {
            self.fill_remaining -= 1;
            if self.fill_remaining == 0 {
                self.load_next_word();
            }
            return Some(self.fill_value);
        }

        // Handle literal words
        if let Some(word) = self.current_word {
            if self.bit_position < 32 {
                let bit = (word.literal_data() >> self.bit_position) & 1 != 0;
                self.bit_position += 1;
                
                if self.bit_position >= 32 {
                    self.load_next_word();
                }
                
                return Some(bit);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wah_word_literal() {
        let word = WahWord::literal(0x12345678);
        assert!(!word.is_fill());
        assert_eq!(word.literal_data(), 0x12345678);
    }

    #[test]
    fn test_wah_word_fill() {
        let word = WahWord::fill(true, 1000).unwrap();
        assert!(word.is_fill());
        assert!(word.fill_value());
        assert_eq!(word.fill_count(), 1000);
    }

    #[test]
    fn test_encoder_basic() {
        let mut encoder = WahEncoder::new();
        encoder.append_bits(&[0b10101010], 8).unwrap();
        let words = encoder.finish().unwrap();
        assert!(!words.is_empty());
    }
}