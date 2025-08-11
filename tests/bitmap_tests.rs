use bitmap_indexer::bitmap::header_from_page_bytes;
use bitmap_indexer::storage::{filesystem::FileSystem, MemoryMappedFile};
use bitmap_indexer::wah::compression::WahWord;
use bitmap_indexer::{
    types::{wah, PAGE_SIZE},
    BitmapIndex,
};
use tempfile::TempDir;

// Test helpers for parsing bitmap page layout
const BM_DATA_OFFSET: usize = 64; // bytes
const BM_WORD_BYTES: usize = 8; // u64
const BM_DATA_WORDS: usize = (PAGE_SIZE - BM_DATA_OFFSET) / BM_WORD_BYTES; // 504 for 4KiB
const BM_MAGIC_WORD: u32 = u32::from_le_bytes(*b"btmp");

// Header reserved area offset (u32 little-endian fields)
const OFF_RESERVED_START: usize = 36;

fn read_u32_le(page: &[u8], offset: usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&page[offset..offset + 4]);
    u32::from_le_bytes(b)
}

fn read_u64_le(page: &[u8], offset: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&page[offset..offset + 8]);
    u64::from_le_bytes(b)
}

fn data_word(page: &[u8], index: usize) -> u64 {
    read_u64_le(page, BM_DATA_OFFSET + index * BM_WORD_BYTES)
}

fn reopen_fs(path: &std::path::Path) -> FileSystem {
    let storage = MemoryMappedFile::new(path).unwrap();
    FileSystem::new(storage).unwrap()
}

#[test]
fn test_new_bitmap_initial_page_header() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    // Create new index and bitmap
    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let _writer = index.create_bitmap("bm").unwrap();

    // Reopen FS for raw inspection
    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    assert!(file_page > 0);

    let page = fs.get_page_data(file_page).unwrap();
    let hdr = header_from_page_bytes(page).unwrap();

    assert_eq!(hdr.magic, BM_MAGIC_WORD);
    assert_eq!(hdr.next_page, u32::MAX);
    assert_eq!(
        hdr.written_words, 1,
        "first page starts with one word (initial FILL_0)"
    );
    assert_eq!(hdr.cw_offset, 0);
    assert_eq!(hdr.length_0, 0);
    assert_eq!(hdr.length_1, 0);
    assert_eq!(hdr.last_page, file_page);
    assert_eq!(hdr.last_fill_page, file_page);
    assert_eq!(hdr.last_fill_pos, 0);

    // Reserved area
    for i in 0..7 {
        let off = OFF_RESERVED_START + i * 4;
        assert_eq!(
            read_u32_le(page, off),
            0,
            "reserved u32 at {} should be zero",
            off
        );
    }

    // Data area initial word is canonical FILL_0, others zero
    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 0).unwrap().raw()
    );
    assert_eq!(data_word(page, 1), WahWord::literal(0).raw());
}

#[test]
fn test_write_literal_word_and_header_state() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Write one literal 63-bit word
    let lit = 0x1234_5678_9ABC_DEF0u64 & wah::DATA_BITS;
    writer.append_bits(&[lit], 63).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();
    let hdr = header_from_page_bytes(page).unwrap();

    assert_eq!(hdr.written_words, 2);
    assert_eq!(hdr.cw_offset, 0);
    assert_eq!(hdr.length_0, 63);
    assert_eq!(hdr.length_1, 0);

    // FILL_0 literal-count incremented, and literal stored at [1]
    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 1).unwrap().raw()
    );
    assert_eq!(data_word(page, 1), WahWord::literal(lit).raw());

    // Optional: reopen bitmap reader but do not iterate yet
    let mut index2 = BitmapIndex::open(&mmf_path).unwrap();
    let _reader = index2.open_bitmap("bm").unwrap();
    // for _b in _reader { /* not implemented yet */ }
}

#[test]
fn test_zero_fill_extension() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Append 63 zeros twice (should extend initial zero fill)
    writer.append_bits(&[0u64], 63).unwrap();
    writer.append_bits(&[0u64], 63).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();
    let hdr = header_from_page_bytes(page).unwrap();

    // Two compressed words added to initial fill (count = 2), no new words written
    assert_eq!(hdr.written_words, 1);
    assert_eq!(hdr.cw_offset, 0);
    assert_eq!(hdr.length_0, 126);
    assert_eq!(hdr.length_1, 0);
    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 2, 0).unwrap().raw()
    );
}

#[test]
fn test_one_fill_new_word_and_extension() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Append one 63-bit all-ones word -> starts new fill at position 1
    let ones = wah::DATA_BITS;
    writer.append_bits(&[ones], 63).unwrap();
    // Append another 63 ones -> extends the last ones fill in-place
    writer.append_bits(&[ones], 63).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();
    let hdr = header_from_page_bytes(page).unwrap();

    assert_eq!(hdr.written_words, 2);
    assert_eq!(hdr.cw_offset, 0);
    assert_eq!(hdr.length_0, 126);
    assert_eq!(hdr.length_1, 0);

    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 0).unwrap().raw()
    ); // initial zero fill unchanged
    assert_eq!(data_word(page, 1), WahWord::fill(true, 2, 0).unwrap().raw()); // ones fill with count 2
}

#[test]
fn test_mixed_literal_and_fill_orders() {
    // literal then ones fill
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap1.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();
    let lit = 0x0123_4567_89AB_CDEFu64 & wah::DATA_BITS;
    writer.append_bits(&[lit], 63).unwrap(); // literal
    writer.append_bits(&[wah::DATA_BITS], 63).unwrap(); // ones fill (cannot extend previous zero fill due to literal_count>0)
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();

    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 1).unwrap().raw()
    ); // literal_count = 1
    assert_eq!(data_word(page, 1), WahWord::literal(lit).raw());
    assert_eq!(data_word(page, 2), WahWord::fill(true, 1, 0).unwrap().raw());

    // fill then literal
    let tmp2 = TempDir::new().unwrap();
    let mmf_path2 = tmp2.path().join("bitmap2.mmf");
    let mut index2 = BitmapIndex::open(&mmf_path2).unwrap();
    let mut writer2 = index2.create_bitmap("bm").unwrap();
    writer2.append_bits(&[wah::DATA_BITS], 63).unwrap(); // ones fill -> new fill at pos 1
    writer2.append_bits(&[lit], 63).unwrap(); // literal -> increments literal_count on last fill
    writer2.close().unwrap();

    let mut fs2 = reopen_fs(&mmf_path2);
    let file_page2 = fs2.get_file_page("bm").unwrap();
    let page2 = fs2.get_page_data(file_page2).unwrap();

    assert_eq!(
        data_word(page2, 0),
        WahWord::fill(false, 0, 0).unwrap().raw()
    );
    assert_eq!(
        data_word(page2, 1),
        WahWord::fill(true, 1, 1).unwrap().raw()
    ); // ones fill count 1, literal_count 1
    assert_eq!(data_word(page2, 2), WahWord::literal(lit).raw());
}

#[test]
fn test_full_page_handling_and_linking() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Fill the rest of the first page with literals to reach exactly full page
    // First page already has written_words=1 for initial FILL_0.
    let lit1 = 0x00FF_00FF_00FF_00FFu64 & wah::DATA_BITS;
    let lit2 = 0x7F00_FF00_FF00_FF00u64 & wah::DATA_BITS;

    let lit1_array = vec![lit1; BM_DATA_WORDS - 1];
    writer
        .append_bits(&lit1_array, 63 * (BM_DATA_WORDS - 1))
        .unwrap();

    // Next write should trigger allocation of a new page and linking
    writer.append_bits(&[lit2], 63).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page0 = fs.get_page_data(file_page).unwrap();

    // Header checks on first page
    let hdr0 = header_from_page_bytes(page0).unwrap();
    assert_eq!(hdr0.magic, BM_MAGIC_WORD);
    assert_ne!(hdr0.next_page, u32::MAX, "next_page should be set");
    let next_page = hdr0.next_page;
    assert_eq!(hdr0.written_words, BM_DATA_WORDS as u32);
    assert_eq!(hdr0.cw_offset, 0);
    assert_eq!(hdr0.last_page, next_page);

    // Fill word on first word with literal count of BM_DATA_WORDS
    assert_eq!(
        data_word(page0, 0),
        WahWord::fill(false, 0, BM_DATA_WORDS as u32).unwrap().raw()
    );

    for i in 1..BM_DATA_WORDS {
        assert_eq!(data_word(page0, i), lit1);
    }

    // New page checks
    let page1 = fs.get_page_data(next_page).unwrap();
    let hdr1 = header_from_page_bytes(page1).unwrap();
    assert_eq!(hdr1.magic, BM_MAGIC_WORD);
    assert_eq!(hdr1.next_page, u32::MAX);
    // After one literal on new page, written_words should be 1
    assert_eq!(hdr1.written_words, 1);

    assert_eq!(data_word(page1, 0), WahWord::literal(lit2).raw());

    // Verify persistence across reopen
    drop(fs);
    let mut fs_again = reopen_fs(&mmf_path);
    let page0_again = fs_again.get_page_data(file_page).unwrap();
    let hdr0_again = header_from_page_bytes(page0_again).unwrap();
    assert_eq!(hdr0_again.next_page, next_page);
}

#[test]
fn test_full_page_handling_and_linking_with_cw_offset() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Establish a non-zero cw_offset with a specific 7-bit pattern (0b0101101 = 0x2D)
    let pbits: u64 = 0x2D;
    let cw_offset = 7u32;
    writer.append_bits(&[pbits], cw_offset as usize).unwrap();

    // Fill the rest of the first page with literals to reach exactly full page
    // First page already has written_words=1 for initial FILL_0.
    let lit = 0x00FF_00FF_00FF_00FFu64 & wah::DATA_BITS;

    let lit_array = vec![lit; BM_DATA_WORDS];
    writer.append_bits(&lit_array, 63 * BM_DATA_WORDS).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page0 = fs.get_page_data(file_page).unwrap();

    // Header checks on first page
    let hdr0 = header_from_page_bytes(page0).unwrap();
    assert_eq!(hdr0.magic, BM_MAGIC_WORD);
    assert_ne!(hdr0.next_page, u32::MAX, "next_page should be set");
    let next_page = hdr0.next_page;
    assert_eq!(hdr0.written_words, BM_DATA_WORDS as u32);
    assert_eq!(hdr0.cw_offset, 0);
    assert_eq!(hdr0.last_page, next_page);

    assert_eq!(
        data_word(page0, 0),
        WahWord::fill(false, 0, BM_DATA_WORDS as u32).unwrap().raw()
    );
    assert_eq!(
        data_word(page0, 1),
        ((lit << cw_offset) | pbits) & wah::DATA_BITS
    );
    let rotated_lit = ((lit << cw_offset) | (lit >> (63 - cw_offset))) & wah::DATA_BITS;
    for i in 2..BM_DATA_WORDS {
        assert_eq!(data_word(page0, i), rotated_lit);
    }

    // New page checks
    let page1 = fs.get_page_data(next_page).unwrap();
    let hdr1 = header_from_page_bytes(page1).unwrap();
    assert_eq!(hdr1.magic, BM_MAGIC_WORD);
    assert_eq!(hdr1.next_page, u32::MAX);
    // After one literal on new page, written_words should be 1
    assert_eq!(hdr1.written_words, 1);
    assert_eq!(hdr1.cw_offset, cw_offset);

    assert_eq!(data_word(page1, 0), rotated_lit);
    assert_eq!(
        data_word(page1, 1),
        (lit >> (63 - cw_offset)) & wah::DATA_BITS
    );

    // Verify persistence across reopen
    drop(fs);
    let mut fs_again = reopen_fs(&mmf_path);
    let page0_again = fs_again.get_page_data(file_page).unwrap();
    let hdr0_again = header_from_page_bytes(page0_again).unwrap();
    assert_eq!(hdr0_again.next_page, next_page);
}

#[test]
fn test_two_phase_transfer_partial_alignment() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Step 1: write 10 ones (partial word)
    let ten_ones = (1u64 << 10) - 1; // 0x3FF
    writer.append_bits(&[ten_ones], 10).unwrap();

    // Verify the current word contains ten_ones before proceeding
    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();
    assert_eq!(data_word(page, 1), ten_ones);
    drop(fs);

    // Step 2: write one full 63-bit ones literal; with cw_offset=10 this will
    // complete a word of all ones (creating a ones fill) and leave 10 ones in the next word
    writer.append_bits(&[wah::DATA_BITS], 63).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();

    // Validate header: written_words should be 2 (initial FILL_0 + new ones fill); cw_offset remains 10
    let hdr = header_from_page_bytes(page).unwrap();
    assert_eq!(hdr.written_words, 2);
    assert_eq!(hdr.cw_offset, 10);
    assert_eq!(hdr.length_0, 73);
    assert_eq!(hdr.length_1, 0);

    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 0).unwrap().raw()
    );
    assert_eq!(data_word(page, 1), WahWord::fill(true, 1, 0).unwrap().raw());
    assert_eq!(data_word(page, 2), WahWord::literal(ten_ones).raw());
}

#[test]
fn test_two_phase_multiple_literals_with_offset() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Establish a non-zero cw_offset with a specific 7-bit pattern (0b0101101 = 0x2D)
    let pbits: u64 = 0x2D;
    let cw_offset = 7u32;
    writer.append_bits(&[pbits], cw_offset as usize).unwrap();

    // Two distinct non-fill literal words
    let lit_a: u64 = 0x0123_4567_89AB_CDEFu64 & wah::DATA_BITS;
    let lit_b: u64 = 0x1357_9BDF_0246_8ACEu64 & wah::DATA_BITS;

    // Append two full literal words with non-zero cw_offset to exercise two-phase loop
    writer.append_bits(&[lit_a, lit_b], 126).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();
    let hdr = header_from_page_bytes(page).unwrap();

    // Initial fill + 2 full literal words
    assert_eq!(hdr.written_words, 3);
    assert_eq!(hdr.cw_offset, cw_offset);
    assert_eq!(hdr.length_0 as u64, cw_offset as u64 + 126);

    // FILL_0 literal count should be 2
    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 2).unwrap().raw()
    );

    // Expected packed words per two-phase algorithm
    let hi_len = cw_offset;
    let lo_len = 63 - hi_len;

    let word1 = (pbits | (lit_a << hi_len)) & wah::DATA_BITS;
    let word2 = ((lit_a >> lo_len) | (lit_b << hi_len)) & wah::DATA_BITS;
    let tail = (lit_b >> lo_len) & wah::DATA_BITS; // current partial word

    assert_eq!(data_word(page, 1), WahWord::literal(word1).raw());
    assert_eq!(data_word(page, 2), WahWord::literal(word2).raw());
    assert_eq!(data_word(page, 3), WahWord::literal(tail).raw());
}

#[test]
fn test_two_phase_runs_of_100_bits_interleaved_with_literals() {
    let tmp = TempDir::new().unwrap();
    let mmf_path = tmp.path().join("bitmap.mmf");

    let mut index = BitmapIndex::open(&mmf_path).unwrap();
    let mut writer = index.create_bitmap("bm").unwrap();

    // Make cw_offset non-zero with 5 zero bits (ensures partial word but no data set)
    writer.append_bits(&[0u64], 5).unwrap();

    // 100 ones, then a literal, then 100 zeros, then a literal
    let ones = wah::DATA_BITS;
    let zeros = 0u64;
    let ones_100 = [ones, ones];
    let zeros_100 = [zeros, zeros];
    let lit_x: u64 = 0x1F00_EE00_DD00_CC00u64 & wah::DATA_BITS;
    let lit_y: u64 = 0x0F0F_F0F0_0F0F_F0F0u64 & wah::DATA_BITS;

    writer.append_bits(&ones_100, 100).unwrap();
    writer.append_bits(&[lit_x], 63).unwrap();
    writer.append_bits(&zeros_100, 100).unwrap();
    writer.append_bits(&[lit_y], 63).unwrap();
    writer.close().unwrap();

    let mut fs = reopen_fs(&mmf_path);
    let file_page = fs.get_file_page("bm").unwrap();
    let page = fs.get_page_data(file_page).unwrap();
    let hdr = header_from_page_bytes(page).unwrap();

    // Total bits: 5 + 100 + 63 + 100 + 63 = 331
    assert_eq!(hdr.length_0, 331);
    assert_eq!(hdr.length_1, 0);

    // Expected words written on first page:
    // - initial FILL_0 (1)
    // - after 100 ones with cw_offset=5: a literal (2)
    // - lit_x completes previous partial -> a literal (3)
    // - starting 100 zeros completes lit_x tail -> a literal (4)
    // - zeros then create a new zero fill (5)
    // - lit_y produces a literal after the new zero fill (6)
    assert_eq!(hdr.written_words, 6);

    // cw_offset evolution: start 5 -> +100%63= +37 => 42; +63 => 42; +100%63= +37 => 16; +63 => 16
    assert_eq!(hdr.cw_offset, 16);

    // Validate the 6 written words are what we expect
    assert_eq!(
        data_word(page, 0),
        WahWord::fill(false, 0, 3).unwrap().raw()
    ); // initial zero fill with literal_count=3
       // After 100 ones starting at cw_offset=5, first full word is a literal with lower 5 zeros and upper 58 ones
    assert_eq!(
        data_word(page, 1),
        WahWord::literal((ones << 5) & wah::DATA_BITS).raw()
    );
    // Next full word combines low 42 ones with high 21 bits of lit_x
    assert_eq!(
        data_word(page, 2),
        WahWord::literal(((1u64 << 42) - 1) | ((lit_x << 42) & wah::DATA_BITS)).raw()
    );
    // Then we flush the remaining 21 low bits of lit_x as a literal
    assert_eq!(
        data_word(page, 3),
        WahWord::literal((lit_x >> 21) & wah::DATA_BITS).raw()
    );
    // Zeros create a new zero fill (count=1) and later lit_y increments its literal_count to 1
    assert_eq!(
        data_word(page, 4),
        WahWord::fill(false, 1, 1).unwrap().raw()
    );
    // lit_y written at cw_offset=16
    assert_eq!(
        data_word(page, 5),
        WahWord::literal((lit_y << 16) & wah::DATA_BITS).raw()
    );
    // The 7th word should contain the 16 remaining bits from the partial word
    assert_eq!(
        data_word(page, 6),
        WahWord::literal(lit_y >> (63 - 16)).raw()
    );

    // Header should point to the last fill we created (at index 4)
    assert_eq!(hdr.last_fill_page, file_page);
    assert_eq!(hdr.last_fill_pos, 4);
}
