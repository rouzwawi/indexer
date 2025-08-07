//! SHA1 hashing utilities
//!
//! This module provides SHA1 hashing functionality for file identification
//! and hash table operations.

pub mod sha1;

pub use sha1::{Sha1Hash, sha1_hash, sha1_string, sha1_to_hex};