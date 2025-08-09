//! SHA1 hash implementation for file identification.

use sha1::{Sha1, Digest};
use serde::{Serialize, Deserialize};
use std::fmt;

/// SHA1 hash wrapper for file identification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha1Hash([u32; 5]);

impl Sha1Hash {
    /// Create a SHA1 hash from a string
    pub fn from_string(input: &str) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let mut words = [0u32; 5];
        for i in 0..5 {
            words[i] = u32::from_be_bytes([
                result[i * 4],
                result[i * 4 + 1],
                result[i * 4 + 2],
                result[i * 4 + 3],
            ]);
        }
        Sha1Hash(words)
    }

    /// Create a SHA1 hash from bytes
    pub fn from_bytes(input: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(input);
        let result = hasher.finalize();
        let mut words = [0u32; 5];
        for i in 0..5 {
            words[i] = u32::from_be_bytes([
                result[i * 4],
                result[i * 4 + 1],
                result[i * 4 + 2],
                result[i * 4 + 3],
            ]);
        }
        Sha1Hash(words)
    }

    /// Get the hash as u32 array
    pub fn as_words(&self) -> &[u32; 5] {
        &self.0
    }

    /// Get the hash as a byte array
    pub fn as_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        for i in 0..5 {
            let word_bytes = self.0[i].to_be_bytes();
            bytes[i * 4..i * 4 + 4].copy_from_slice(&word_bytes);
        }
        bytes
    }

    /// Get the hash as a hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.as_bytes())
    }
}

impl fmt::Display for Sha1Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u32; 5]> for Sha1Hash {
    fn from(words: [u32; 5]) -> Self {
        Sha1Hash(words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_from_string() {
        let hash1 = Sha1Hash::from_string("hello");
        let hash2 = Sha1Hash::from_string("hello");
        let hash3 = Sha1Hash::from_string("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_sha1_hex() {
        let hash = Sha1Hash::from_string("hello");
        let hex = hash.to_hex();
        assert_eq!(hex.len(), 40); // SHA1 is 160 bits = 40 hex chars
    }

    #[test]
    fn test_sha1_display() {
        let hash = Sha1Hash::from_string("test");
        let display = format!("{}", hash);
        assert_eq!(display, hash.to_hex());
    }
}
