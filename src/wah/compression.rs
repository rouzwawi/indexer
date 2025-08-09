//! Core WAH compression implementation.

use crate::types::{Result, Error, wah::*};
use std::fmt;

/// A WAH-compressed word that can be either a fill word or a literal word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WahWord(pub u64);

impl WahWord {
    /// Create a new literal word
    pub fn literal(value: u64) -> Self {
        WahWord(value & LITERAL_MASK)
    }

    /// Create a new fill word
    pub fn fill(value: bool, count: u64) -> Result<Self> {
        if count > MAX_FILL_COUNT {
            return Err(Error::Compression(format!(
                "Fill count {} exceeds maximum {}", count, MAX_FILL_COUNT
            )));
        }

        let mut word = FILL_FLAG | count;
        if value {
            word |= FILL_VAL;
        }
        Ok(WahWord(word))
    }

    /// Check if this is a fill word
    pub fn is_fill(&self) -> bool {
        (self.0 & FILL_FLAG) != 0
    }

    /// Get the fill value (true/false) for fill words
    pub fn fill_value(&self) -> bool {
        (self.0 & FILL_VAL) != 0
    }

    /// Get the fill count for fill words
    pub fn fill_count(&self) -> u64 {
        self.0 & MAX_FILL_COUNT
    }

    /// Get the literal value for literal words
    pub fn literal_value(&self) -> u64 {
        self.0 & LITERAL_MASK
    }

    /// Get the raw word value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for WahWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_fill() {
            write!(f, "Fill({}, {})", self.fill_value(), self.fill_count())
        } else {
            write!(f, "Literal(0x{:016x})", self.literal_value())
        }
    }
}

/// WAH encoder for compressing bitmaps
pub struct WahEncoder {
    output: Vec<WahWord>,
    current_run_value: Option<bool>,
    current_run_count: u64,
}

impl WahEncoder {
    /// Create a new WAH encoder
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
            current_run_value: None,
            current_run_count: 0,
        }
    }

    /// Encode a sequence of bits
    pub fn encode_bits(&mut self, bits: &[u8], bit_count: usize) -> Result<Vec<WahWord>> {
        let mut bit_pos = 0;

        while bit_pos < bit_count {
            let byte_idx = bit_pos / 8;
            let bit_idx = bit_pos % 8;

            if byte_idx >= bits.len() {
                break;
            }

            let bit_value = (bits[byte_idx] >> (7 - bit_idx)) & 1 == 1;
            self.process_bit(bit_value)?;
            bit_pos += 1;
        }

        self.flush()?;
        Ok(std::mem::take(&mut self.output))
    }

    /// Process a single bit
    fn process_bit(&mut self, bit: bool) -> Result<()> {
        match self.current_run_value {
            None => {
                // Start new run
                self.current_run_value = Some(bit);
                self.current_run_count = 1;
            }
            Some(run_value) if run_value == bit => {
                // Continue current run
                self.current_run_count += 1;

                // Check if we need to flush due to max count
                if self.current_run_count >= MAX_FILL_COUNT {
                    self.flush_current_run()?;
                }
            }
            Some(_) => {
                // Run value changed, flush current run and start new one
                self.flush_current_run()?;
                self.current_run_value = Some(bit);
                self.current_run_count = 1;
            }
        }

        Ok(())
    }

    /// Flush the current run
    fn flush_current_run(&mut self) -> Result<()> {
        if let Some(value) = self.current_run_value {
            if self.current_run_count > 0 {
                // For small runs, consider using literal words
                if self.current_run_count <= 64 {
                    self.add_literal_run(value, self.current_run_count as usize)?;
                } else {
                    self.output.push(WahWord::fill(value, self.current_run_count)?);
                }
            }
        }

        self.current_run_value = None;
        self.current_run_count = 0;
        Ok(())
    }

    /// Add a run as literal words
    fn add_literal_run(&mut self, value: bool, count: usize) -> Result<()> {
        let literal_value = if value { u64::MAX } else { 0 };
        let full_words = count / 64;
        let remaining_bits = count % 64;

        // Add full words
        for _ in 0..full_words {
            self.output.push(WahWord::literal(literal_value));
        }

        // Add partial word if needed
        if remaining_bits > 0 {
            let partial_value = if value {
                (1u64 << remaining_bits) - 1
            } else {
                0
            };
            self.output.push(WahWord::literal(partial_value));
        }

        Ok(())
    }

    /// Flush any remaining data
    fn flush(&mut self) -> Result<()> {
        self.flush_current_run()
    }
}

impl Default for WahEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// WAH decoder for decompressing bitmaps
pub struct WahDecoder {
    output: Vec<u8>,
}

impl WahDecoder {
    /// Create a new WAH decoder
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
        }
    }

    /// Decode a sequence of WAH words
    pub fn decode_words(&mut self, words: &[WahWord]) -> Result<Vec<u8>> {
        for word in words {
            self.decode_word(*word)?;
        }

        Ok(std::mem::take(&mut self.output))
    }

    /// Decode a single WAH word
    fn decode_word(&mut self, word: WahWord) -> Result<()> {
        if word.is_fill() {
            let value = word.fill_value();
            let count = word.fill_count();
            self.add_fill(value, count as usize)?;
        } else {
            self.add_literal(word.literal_value())?;
        }

        Ok(())
    }

    /// Add a fill sequence to the output
    fn add_fill(&mut self, value: bool, bit_count: usize) -> Result<()> {
        let byte_value = if value { 0xFF } else { 0x00 };
        let full_bytes = bit_count / 8;
        let remaining_bits = bit_count % 8;

        // Add full bytes
        self.output.resize(self.output.len() + full_bytes, byte_value);

        // Add partial byte if needed
        if remaining_bits > 0 {
            let partial_value = if value {
                (1u8 << remaining_bits) - 1
            } else {
                0
            };
            self.output.push(partial_value);
        }

        Ok(())
    }

    /// Add a literal word to the output
    fn add_literal(&mut self, value: u64) -> Result<()> {
        // Convert 64-bit word to bytes (little-endian)
        for i in 0..8 {
            let byte = ((value >> (i * 8)) & 0xFF) as u8;
            self.output.push(byte);
        }

        Ok(())
    }
}

impl Default for WahDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wah_word_literal() {
        let word = WahWord::literal(0x123456789ABCDEF0);
        assert!(!word.is_fill());
        assert_eq!(word.literal_value(), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_wah_word_fill() {
        let word = WahWord::fill(true, 1000).unwrap();
        assert!(word.is_fill());
        assert!(word.fill_value());
        assert_eq!(word.fill_count(), 1000);
    }

    #[test]
    fn test_wah_word_fill_false() {
        let word = WahWord::fill(false, 500).unwrap();
        assert!(word.is_fill());
        assert!(!word.fill_value());
        assert_eq!(word.fill_count(), 500);
    }

    #[test]
    fn test_encoder_decoder_round_trip() {
        // Test with a simple pattern that should compress well
        let original_bits = vec![0xFF, 0xFF, 0x00, 0x00]; // 16 ones followed by 16 zeros
        let bit_count = 32;

        let mut encoder = WahEncoder::new();
        let compressed = encoder.encode_bits(&original_bits, bit_count).unwrap();

        let mut decoder = WahDecoder::new();
        let decompressed = decoder.decode_words(&compressed).unwrap();

        // Check that we can decompress
        assert!(!decompressed.is_empty());

        // Check we get the same results by comparing the original bits
        let expected_bytes = (bit_count + 7) / 8;
        assert_eq!(&decompressed[..expected_bytes], &original_bits[..expected_bytes]);
    }

    #[test]
    fn test_fill_count_limit() {
        let result = WahWord::fill(true, MAX_FILL_COUNT + 1);
        assert!(result.is_err());
    }
}
