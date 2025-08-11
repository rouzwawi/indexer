//! Core WAH compression implementation aligned with the legacy C++ layout.

use crate::types::{wah, Error, Result};
use std::fmt;

/// A WAH-compressed word that can be either a fill word or a literal word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WahWord(pub u64);

impl WahWord {
    /// Create a new literal word
    pub fn literal(value: u64) -> Self {
        WahWord(value & wah::DATA_BITS)
    }

    /// Create a new fill word
    ///
    /// count_words: number of 63-bit words in the fill (<= 2^31-1)
    /// literal_count: number of literal words that immediately follow this fill (<= 2^31-1)
    pub fn fill(value: bool, count_words: u32, literal_count: u32) -> Result<Self> {
        if (count_words as u64) > wah::FILL_BITS {
            return Err(Error::Compression("Fill count exceeds maximum".into()));
        }
        // Encode per C++: [1 bit FILL][1 bit VAL][31 bits LITERAL COUNT][31 bits FILL COUNT]
        let mut word = wah::FILL_FLAG | ((literal_count as u64) << 31) | (count_words as u64);
        if value {
            word |= wah::FILL_VAL;
        }
        Ok(WahWord(word))
    }

    /// Check if this is a fill word
    pub fn is_fill(&self) -> bool {
        (self.0 & wah::FILL_FLAG) != 0
    }

    /// Get the fill value (true/false) for fill words
    pub fn fill_value(&self) -> bool {
        (self.0 & wah::FILL_VAL) != 0
    }

    /// Get the fill count for fill words
    pub fn fill_count(&self) -> u32 {
        (self.0 & wah::FILL_BITS) as u32
    }

    /// Get the literal count associated with this fill (number of literal words following)
    pub fn literal_count(&self) -> u32 {
        ((self.0 & wah::LTRL_BITS) >> 31) as u32
    }

    /// Get the literal value for literal words
    pub fn literal_value(&self) -> u64 {
        self.0 & wah::DATA_BITS
    }

    /// Get the raw word value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for WahWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_fill() {
            write!(
                f,
                "Fill({}, fc:{}, lc:{})",
                if self.fill_value() { 1 } else { 0 },
                self.fill_count(),
                self.literal_count()
            )
        } else {
            write!(f, "Literal(0x{:016x})", self.literal_value())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wah_word_literal() {
        let word = WahWord::literal(0x123456789ABCDEF0);
        assert!(!word.is_fill());
        assert_eq!(word.literal_value(), 0x123456789ABCDEF0 & wah::DATA_BITS);
    }

    #[test]
    fn test_wah_word_fill() {
        let word = WahWord::fill(true, 1000, 3).unwrap();
        assert!(word.is_fill());
        assert!(word.fill_value());
        assert_eq!(word.fill_count(), 1000);
        assert_eq!(word.literal_count(), 3);
    }

    #[test]
    fn test_wah_word_fill_false() {
        let word = WahWord::fill(false, 500, 0).unwrap();
        assert!(word.is_fill());
        assert!(!word.fill_value());
        assert_eq!(word.fill_count(), 500);
    }
}
