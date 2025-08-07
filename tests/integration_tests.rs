use bitmap_indexer::BitmapIndex;
use tempfile::TempDir;

#[test]
fn test_bitmap_index_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let index = BitmapIndex::open(temp_dir.path()).unwrap();

    // Test that we can create the index
    assert_eq!(index.list_bitmaps().unwrap().len(), 0);

    // Test creating a bitmap would work (placeholder until implementation is complete)
    // let mut writer = index.create_bitmap("test").unwrap();
    // writer.append_bits(&[0b10101010], 8).unwrap();
    // drop(writer);

    // assert_eq!(index.list_bitmaps().unwrap().len(), 1);
    // assert!(index.list_bitmaps().unwrap().contains(&"test".to_string()));
}

#[test]
fn test_wah_compression_basic() {
    use bitmap_indexer::wah::{WahEncoder, WahDecoder};

    let mut encoder = WahEncoder::new();
    encoder.append_bits(&[0b10101010], 8).unwrap();
    encoder.fill(true, 1000).unwrap();
    
    let words = encoder.finish().unwrap();
    assert!(!words.is_empty());

    let decoder = WahDecoder::new(words);
    let mut iter = decoder.iter();
    
    // Should have bits to iterate over
    assert!(iter.next().is_some());
}

#[test]
fn test_hash_functionality() {
    use bitmap_indexer::hash::{sha1_string, sha1_to_hex};

    let hash1 = sha1_string("test");
    let hash2 = sha1_string("test");
    let hash3 = sha1_string("different");

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);

    let hex = sha1_to_hex(&hash1);
    assert_eq!(hex.len(), 40); // SHA1 is 20 bytes = 40 hex chars
}