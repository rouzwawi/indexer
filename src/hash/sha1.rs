//! SHA1 hashing implementation
//!
//! Provides SHA1 hashing functionality for file identification.


use sha1::{Sha1, Digest};

/// SHA1 hash type (20 bytes)
pub type Sha1Hash = [u8; 20];

/// Compute SHA1 hash of input data
pub fn sha1_hash(data: &[u8]) -> Sha1Hash {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute SHA1 hash of a string
pub fn sha1_string(s: &str) -> Sha1Hash {
    sha1_hash(s.as_bytes())
}

/// Convert SHA1 hash to hex string for debugging
pub fn sha1_to_hex(hash: &Sha1Hash) -> String {
    hash.iter()
        .map(|byte| format!("{:02x}", byte))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_hash() {
        let data = b"hello world";
        let hash = sha1_hash(data);
        let hex = sha1_to_hex(&hash);
        // Expected SHA1 of "hello world"
        assert_eq!(hex, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }

    #[test]
    fn test_sha1_string() {
        let hash1 = sha1_string("test");
        let hash2 = sha1_hash(b"test");
        assert_eq!(hash1, hash2);
    }
}