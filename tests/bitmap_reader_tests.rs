//! Tests for BitmapReader iterator implementation
//!
//! These tests verify that the BitmapReader can correctly decode WAH-compressed bitmap data
//! and iterate through the bits in the correct order.

use bitmap_indexer::{BitmapIndex};
use tempfile::TempDir;

#[test]
fn test_read_simple_fill_pattern() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create a bitmap with a simple fill pattern
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("test").unwrap();

    // Write 63 zeros, 63 ones, 63 zeros
    writer.append_bits(&[0u64], 63).unwrap();
    writer.append_bits(&[0x7FFF_FFFF_FFFF_FFFF], 63).unwrap(); // All ones in 63 bits
    writer.append_bits(&[0u64], 63).unwrap();
    writer.close().unwrap();

    // Read back and verify
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let reader = index.open_bitmap("test").unwrap();

    let bits: Vec<bool> = reader.collect();
    assert_eq!(bits.len(), 189); // 63 + 63 + 63

    // First 63 should be false
    for i in 0..63 {
        assert_eq!(bits[i], false, "Bit {} should be false", i);
    }

    // Next 63 should be true
    for i in 63..126 {
        assert_eq!(bits[i], true, "Bit {} should be true", i);
    }

    // Last 63 should be false
    for i in 126..189 {
        assert_eq!(bits[i], false, "Bit {} should be false", i);
    }
}

#[test]
fn test_read_literal_pattern() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create a bitmap with literal patterns
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("test").unwrap();

    // Write a specific 63-bit pattern: alternating bits (0101010...)
    let alternating = 0x5555_5555_5555_5555u64 & 0x7FFF_FFFF_FFFF_FFFF; // 63 bits
    writer.append_bits(&[alternating], 63).unwrap();
    writer.close().unwrap();

    // Read back and verify
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let reader = index.open_bitmap("test").unwrap();

    let bits: Vec<bool> = reader.collect();
    assert_eq!(bits.len(), 63);

    // Check alternating pattern: 0x5555555555555555 = 0101010101... (LSB first)
    for i in 0..63 {
        let expected = (i % 2) == 0; // Even positions should be true (bit 0=1, bit 2=1, etc.)
        assert_eq!(bits[i], expected, "Bit {} should be {}", i, expected);
    }
}

#[test]
fn test_read_mixed_fill_and_literal() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create a bitmap with mixed fill and literal
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("test").unwrap();

    // Write: 63 ones (fill), then literal pattern, then 63 zeros (fill)
    writer.append_bits(&[0x7FFF_FFFF_FFFF_FFFF], 63).unwrap(); // All ones
    let pattern = 0x00FF_00FF_00FF_00FFu64 & 0x7FFF_FFFF_FFFF_FFFF; // 8-bit pattern repeated
    writer.append_bits(&[pattern], 63).unwrap();
    writer.append_bits(&[0u64], 63).unwrap(); // All zeros
    writer.close().unwrap();

    // Read back and verify
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let reader = index.open_bitmap("test").unwrap();

    let bits: Vec<bool> = reader.collect();
    assert_eq!(bits.len(), 189);

    // First 63 should be true
    for i in 0..63 {
        assert_eq!(bits[i], true, "Bit {} should be true", i);
    }

    // Middle 63 should match the pattern
    for i in 63..126 {
        let bit_in_pattern = i - 63;  // Direct position within the 63-bit pattern
        let expected = (pattern >> bit_in_pattern) & 1 != 0;
        assert_eq!(bits[i], expected, "Bit {} should be {}", i, expected);
    }

    // Last 63 should be false
    for i in 126..189 {
        assert_eq!(bits[i], false, "Bit {} should be false", i);
    }
}

#[test]
fn test_read_partial_bits() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create a bitmap with non-word-aligned bit count
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("test").unwrap();

    // Write 10 bits: 1010101010
    let ten_bits = 0b1010101010u64;
    writer.append_bits(&[ten_bits], 10).unwrap();
    writer.close().unwrap();

    // Read back and verify
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let reader = index.open_bitmap("test").unwrap();

    let bits: Vec<bool> = reader.collect();
    assert_eq!(bits.len(), 10);

    // Check the pattern: 0,1,0,1,0,1,0,1,0,1
    for i in 0..10 {
        let expected = (i % 2) == 1;
        assert_eq!(bits[i], expected, "Bit {} should be {}", i, expected);
    }
}

#[test]
fn test_empty_bitmap() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create an empty bitmap (just initial fill with no additional bits)
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("test").unwrap();
    writer.close().unwrap();

    // Read back and verify
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let reader = index.open_bitmap("test").unwrap();

    let bits: Vec<bool> = reader.collect();
    assert_eq!(bits.len(), 0);
}

#[test]
fn test_zero_fill_with_literals() {
    // Test case for a fill word with zero fill count but with literals following
    // This tests the bug fix where we were losing literal count information
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create a bitmap that will compress to:
    // - A fill word with 0 fill count and 2 literals following
    // This can happen when we have less than 63 bits followed by literals

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("test").unwrap();

    // Write pattern that creates zero-fill with literals:
    // First literal word pattern
    let pattern1 = 0x5555555555555555u64 & 0x7FFF_FFFF_FFFF_FFFF; // Alternating bits
    writer.append_bits(&[pattern1], 63).unwrap();

    // Second literal word pattern
    let pattern2 = 0xAAAAAAAAAAAAAAAAu64 & 0x7FFF_FFFF_FFFF_FFFF; // Inverted alternating
    writer.append_bits(&[pattern2], 63).unwrap();

    writer.close().unwrap();

    // Read back and verify
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let reader = index.open_bitmap("test").unwrap();

    let bits: Vec<bool> = reader.collect();
    assert_eq!(bits.len(), 126);

    // Check first 63 bits match pattern1
    for i in 0..63 {
        let expected = (pattern1 >> i) & 1 != 0;
        assert_eq!(bits[i], expected, "Bit {} in first literal should be {}", i, expected);
    }

    // Check second 63 bits match pattern2
    for i in 0..63 {
        let expected = (pattern2 >> i) & 1 != 0;
        assert_eq!(bits[63 + i], expected, "Bit {} in second literal should be {}", i, expected);
    }
}
