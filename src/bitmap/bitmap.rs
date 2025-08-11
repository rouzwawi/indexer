//! Core bitmap implementation.

use crate::storage::{FileSystem, MemoryMappedFile};
use crate::types::Result;
use crate::types::{wah, Error, PAGE_SIZE};
use std::path::Path;

// ===== Bitmap on-page layout (4 KiB page) =====
// Header occupies the first 64 bytes (u32 words). Data area starts at offset 64.
const BM_MAGIC_WORD: u32 = u32::from_le_bytes(*b"btmp");

// Data layout
const BM_DATA_OFFSET: usize = 64; // bytes
const BM_DATA_BITS: u32 = 63; // number of data bits packed per word
const BM_WORD_BYTES: usize = 8; // u64 words
const BM_DATA_WORDS: usize = (PAGE_SIZE - BM_DATA_OFFSET) / BM_WORD_BYTES; // 504 for 4KiB pages

// ===== Memory-mapped structs over a bitmap page =====
#[repr(C)]
struct BitmapHeader {
    magic: u32,          // BM_MAGIC_WORD
    next_page: u32,      // page number or u32::MAX
    written_words: u32,  // number of data words written on this page
    cw_offset: u32,      // bit offset in current word [0..BM_DATA_BITS)
    length_0: u32,       // global length (low 32)
    length_1: u32,       // global length (high 32)
    last_page: u32,      // last page of bitmap
    last_fill_page: u32, // page number containing last fill word
    last_fill_pos: u32,  // position of last fill word within its page
    reserved: [u32; 7],  // pad to 64 bytes total
}

#[repr(C)]
struct BitmapPage {
    header: BitmapHeader,
    data: [u64; BM_DATA_WORDS],
}

/// Public view of a bitmap page header for external inspection/testing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitmapHeaderView {
    pub magic: u32,
    pub next_page: u32,
    pub written_words: u32,
    pub cw_offset: u32,
    pub length_0: u32,
    pub length_1: u32,
    pub last_page: u32,
    pub last_fill_page: u32,
    pub last_fill_pos: u32,
    pub reserved: [u32; 7],
}

impl BitmapHeaderView {
    #[inline]
    fn from_header(h: &BitmapHeader) -> Self {
        Self {
            magic: h.magic,
            next_page: h.next_page,
            written_words: h.written_words,
            cw_offset: h.cw_offset,
            length_0: h.length_0,
            length_1: h.length_1,
            last_page: h.last_page,
            last_fill_page: h.last_fill_page,
            last_fill_pos: h.last_fill_pos,
            reserved: h.reserved,
        }
    }
}

/// Parse a bitmap header from a page byte slice
pub fn header_from_page_bytes(page: &[u8]) -> Result<BitmapHeaderView> {
    verify_bitmap_page(page)?;
    let hdr = &page_as_ref(page).header;
    Ok(BitmapHeaderView::from_header(hdr))
}

#[inline]
fn page_as_ref(page: &[u8]) -> &BitmapPage {
    // SAFETY: Pages are 4KiB aligned and sized; the slice is exactly a page window.
    // Our struct layout is repr(C) and matches the page memory layout.
    unsafe { &*(page.as_ptr() as *const BitmapPage) }
}

#[inline]
fn page_as_mut(page: &mut [u8]) -> &mut BitmapPage {
    // SAFETY: See page_as_ref safety; plus mutable borrow ensures exclusivity.
    unsafe { &mut *(page.as_mut_ptr() as *mut BitmapPage) }
}

// ===== High-level API =====
/// Main bitmap index interface
pub struct BitmapIndex {
    filesystem: FileSystem,
}

impl BitmapIndex {
    /// Open or initialize an index at path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let storage = MemoryMappedFile::new(path)?;
        let filesystem = if storage.allocated_pages() == 0 {
            FileSystem::init(storage)?
        } else {
            FileSystem::new(storage)?
        };
        Ok(Self { filesystem })
    }

    /// Create a new bitmap with a given logical name
    pub fn create_bitmap(&mut self, name: &str) -> Result<BitmapWriter<'_>> {
        let file_page = self.filesystem.create_file(name)?;
        init_bitmap_first_page(&mut self.filesystem, file_page)?;
        Ok(BitmapWriter::new(&mut self.filesystem, file_page)?)
    }

    /// Open an existing bitmap for reading
    pub fn open_bitmap(&mut self, name: &str) -> Result<BitmapReader<'_>> {
        let file_page = self.filesystem.get_file_page(name)?;
        if file_page == 0 {
            return Err(Error::FileNotFound(name.to_string()));
        }
        Ok(BitmapReader::new(&mut self.filesystem, file_page)?)
    }
}

// ===== Core bitmap writer =====
pub struct BitmapWriter<'a> {
    fs: &'a mut FileSystem,
    first_page_num: u32,
    current_page_num: u32,
}

impl<'a> BitmapWriter<'a> {
    fn new(fs: &'a mut FileSystem, first_page_num: u32) -> Result<Self> {
        // Load last page from header
        let current_page_num = {
            let page = fs.get_page_data(first_page_num)?;
            verify_bitmap_page(page)?;
            page_as_ref(page).header.last_page
        };

        Ok(Self {
            fs,
            first_page_num,
            current_page_num,
        })
    }

    /// Append bits to the bitmap. Input is a sequence of 64-bit words.
    /// Only the lowest `bit_count` bits across the provided words are consumed.
    pub fn append_bits(&mut self, data_words: &[u64], bit_count: usize) -> Result<()> {
        if bit_count == 0 {
            return Ok(());
        }

        // Number of full 63-bit words and remaining bits
        let full_words = (bit_count as u32) / BM_DATA_BITS;
        let remain = (bit_count as u32) % BM_DATA_BITS;

        let (mut written_words, mut cw_offset) = self.read_offsets()?;
        let lo_len = BM_DATA_BITS - cw_offset;
        let hi_len = cw_offset;

        if hi_len != 0 {
            // Two-phase transfer of low and high parts
            let mut cw = self.read_current_word(written_words)?;
            let mut i = 0u32;
            while i < full_words {
                let bulk_words =
                    core::cmp::min(BM_DATA_WORDS as u32 - written_words, full_words - i);
                for j in 0..bulk_words {
                    let data = data_words[(i + j) as usize];
                    // phase 1: move low data to high word pos
                    cw |= data << hi_len;
                    // current word now full
                    self.full_word(cw, &mut written_words)?;
                    cw = 0;
                    // phase 2: move high data to low word pos
                    cw |= data >> lo_len;
                }
                i += bulk_words;
                self.full_page(&mut written_words, &mut cw_offset)?;
            }
            self.write_current_word(cw, written_words)?;
        } else {
            // Single-phase transfer of full words
            let mut i = 0u32;
            while i < full_words {
                let bulk_words =
                    core::cmp::min(BM_DATA_WORDS as u32 - written_words, full_words - i);
                for j in 0..bulk_words {
                    let data = data_words[(i + j) as usize];
                    self.full_word(data, &mut written_words)?;
                }
                i += bulk_words;
                self.full_page(&mut written_words, &mut cw_offset)?;
            }
        }

        // Handle remaining bits
        if remain != 0 {
            let data = data_words[full_words as usize];

            // low part fits in current word
            let lo_len_eff = core::cmp::min(remain, BM_DATA_BITS - cw_offset);
            let hi_len_eff = remain - lo_len_eff;

            // write low bits at current offset
            let mut cw = self.read_current_word(written_words)?;
            let mask_lo = if lo_len_eff == 64 {
                u64::MAX
            } else {
                (1u64 << lo_len_eff) - 1
            };
            cw |= (data & mask_lo) << cw_offset;
            cw_offset = (cw_offset + lo_len_eff) % BM_DATA_BITS;

            if cw_offset == 0 {
                // Word is full
                self.full_word(cw, &mut written_words)?;
                self.full_page(&mut written_words, &mut cw_offset)?;
                cw = 0;
            }

            if hi_len_eff != 0 {
                // bits spill to next word at offset 0
                let mask_hi = ((1u64 << hi_len_eff) - 1) << lo_len_eff;
                cw |= (data & mask_hi) >> lo_len_eff;
                cw_offset += hi_len_eff;
                self.write_current_word(cw, written_words)?;
            } else {
                self.write_current_word(cw, written_words)?;
            }
        }

        self.increment_length(bit_count as u64)?;
        self.write_offsets(written_words, cw_offset)?;
        self.fs.sync_page(self.current_page_num)?;
        Ok(())
    }

    /// Fill with a repeated bit value
    pub fn fill(&mut self, value: bool, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        // Simple literal-based fill for now; compression folding will be added later
        let bit: u64 = if value { 1 } else { 0 };
        let (mut cw_offset, mut written_words) = self.read_offsets()?;
        for _ in 0..count {
            // Append one bit at a time into 63-bit words
            let mut cw = self.read_current_word(written_words)?;
            cw |= bit << cw_offset;
            cw_offset += 1;
            if cw_offset >= BM_DATA_BITS {
                self.full_word(cw, &mut written_words)?;
                self.full_page(&mut written_words, &mut cw_offset)?;
                cw_offset = 0;
                self.write_current_word(0, written_words)?;
            } else {
                self.write_current_word(cw, written_words)?;
            }
        }
        self.increment_length(count as u64)?;
        self.write_offsets(written_words, cw_offset)?;
        self.fs.sync_page(self.current_page_num)?;
        Ok(())
    }

    /// Close writer (flushes first page headers)
    pub fn close(&mut self) -> Result<()> {
        // Ensure first page is synced
        self.fs.sync_page(self.first_page_num)?;
        // Ensure current page is synced
        self.fs.sync_page(self.current_page_num)?;
        Ok(())
    }

    // ===== Internal helpers =====
    #[inline]
    fn read_current_word(&mut self, written_words: u32) -> Result<u64> {
        let page = self.fs.get_page_data_mut(self.current_page_num)?;
        let p = page_as_mut(page);
        Ok(p.data[written_words as usize] & ((1u64 << BM_DATA_BITS) - 1))
    }

    #[inline]
    fn write_current_word(&mut self, value: u64, written_words: u32) -> Result<()> {
        let page = self.fs.get_page_data_mut(self.current_page_num)?;
        let p = page_as_mut(page);
        p.data[written_words as usize] = value;
        Ok(())
    }

    #[inline]
    fn full_word(&mut self, word_value: u64, written_words: &mut u32) -> Result<()> {
        // Apply WAH rules from legacy C++:
        // - If the 63-bit word is all-zeros or all-ones, try to compress by extending the last fill
        //   when possible; otherwise, start a new fill word at current position.
        // - Otherwise, store as literal and increment the literal-count of the last fill.

        let w63 = word_value & wah::DATA_BITS;
        let is_all_ones = w63 == wah::DATA_BITS;
        let is_all_zeros = w63 == 0;

        if is_all_ones || is_all_zeros {
            // Try to extend last fill if no literals were added since and same fill value
            let last_fill_info = {
                let first = self.fs.get_page_data(self.first_page_num)?;
                let fp = page_as_ref(first);
                (fp.header.last_fill_page, fp.header.last_fill_pos)
            };

            let want_fill_val_set = is_all_ones; // true for ones, false for zeros

            if last_fill_info.0 != u32::MAX {
                let (lf_page, lf_pos) = last_fill_info;
                if lf_page == self.current_page_num {
                    let page = self.fs.get_page_data_mut(self.current_page_num)?;
                    let p = page_as_mut(page);
                    let last_val = p.data[lf_pos as usize];
                    let no_literals_since = (last_val & wah::LTRL_BITS) == 0;
                    let last_fill_val_set = (last_val & wah::FILL_VAL) != 0;
                    if no_literals_since && last_fill_val_set == want_fill_val_set {
                        // extend existing fill by one word (increment low 31-bit count)
                        p.data[lf_pos as usize] = last_val.wrapping_add(1);
                        // do NOT advance written_words (this word is compressed away)
                        return Ok(());
                    }
                } else {
                    // last fill is on a different page
                    let page = self.fs.get_page_data_mut(lf_page)?;
                    let p = page_as_mut(page);
                    let last_val = p.data[lf_pos as usize];
                    let no_literals_since = (last_val & wah::LTRL_BITS) == 0;
                    let last_fill_val_set = (last_val & wah::FILL_VAL) != 0;
                    if no_literals_since && last_fill_val_set == want_fill_val_set {
                        p.data[lf_pos as usize] = last_val.wrapping_add(1);
                        self.fs.sync_page(lf_page)?;
                        return Ok(());
                    }
                }
            }

            // Start a new fill word at current position with count = 1
            let fill_word = if is_all_ones {
                wah::FILL_1
            } else {
                wah::FILL_0
            } + 1;
            self.write_current_word(fill_word, *written_words)?;

            // Update first page header with last fill location
            {
                let first = self.fs.get_page_data_mut(self.first_page_num)?;
                let fp = page_as_mut(first);
                fp.header.last_fill_page = self.current_page_num;
                fp.header.last_fill_pos = *written_words;
            }

            *written_words += 1;
            return Ok(());
        }

        // Literal word path: write the 63-bit literal and increment literal count on last fill
        self.write_current_word(w63, *written_words)?;

        // Increment literal count of the last fill word
        let last_fill_info = {
            let first = self.fs.get_page_data(self.first_page_num)?;
            let fp = page_as_ref(first);
            (fp.header.last_fill_page, fp.header.last_fill_pos)
        };
        if last_fill_info.0 != u32::MAX {
            let (lf_page, lf_pos) = last_fill_info;
            if lf_page == self.current_page_num {
                let page = self.fs.get_page_data_mut(self.current_page_num)?;
                let p = page_as_mut(page);
                p.data[lf_pos as usize] = p.data[lf_pos as usize].wrapping_add(1u64 << 31);
            } else {
                let page = self.fs.get_page_data_mut(lf_page)?;
                let p = page_as_mut(page);
                p.data[lf_pos as usize] = p.data[lf_pos as usize].wrapping_add(1u64 << 31);
                self.fs.sync_page(lf_page)?;
            }
        }

        *written_words += 1;
        Ok(())
    }

    fn full_page(&mut self, written_words: &mut u32, cw_offset: &mut u32) -> Result<()> {
        if *written_words as usize != BM_DATA_WORDS {
            return Ok(());
        }

        // Save and reset offset for new page initialization
        let saved_cw_offset = *cw_offset;
        *cw_offset = 0;
        self.write_offsets(*written_words, *cw_offset)?;

        // Allocate a new page
        let next_page = self.fs.mmf().allocate_page()?;

        // Initialize the new page BEFORE publishing any links to it and persist it
        init_bitmap_page(&mut self.fs, next_page, u32::MAX, u32::MAX)?;
        self.fs.sync_page(next_page)?;

        // Publish the link from the current page and persist it
        {
            let current = self.fs.get_page_data_mut(self.current_page_num)?;
            let cp = page_as_mut(current);
            cp.header.next_page = next_page;
        }
        self.fs.sync_page(self.current_page_num)?;

        // Update the first page's last_page pointer and persist it
        {
            let first = self.fs.get_page_data_mut(self.first_page_num)?;
            let fp = page_as_mut(first);
            fp.header.last_page = next_page;
        }
        self.fs.sync_page(self.first_page_num)?;

        // Switch to the new page (already initialized)
        self.load_page(next_page)?;

        // Restore offset
        *cw_offset = saved_cw_offset;
        *written_words = 0;
        self.write_offsets(*written_words, *cw_offset)?;
        Ok(())
    }

    fn load_page(&mut self, page_num: u32) -> Result<()> {
        self.current_page_num = page_num;
        let page = self.fs.get_page_data(page_num)?;
        verify_bitmap_page(page)?;
        Ok(())
    }

    fn read_offsets(&mut self) -> Result<(u32, u32)> {
        let current = self.fs.get_page_data(self.current_page_num)?;
        let cp = page_as_ref(current);
        Ok((cp.header.written_words, cp.header.cw_offset))
    }

    fn write_offsets(&mut self, written_words: u32, cw_offset: u32) -> Result<()> {
        let current = self.fs.get_page_data_mut(self.current_page_num)?;
        let cp = page_as_mut(current);
        cp.header.written_words = written_words;
        cp.header.cw_offset = cw_offset;
        Ok(())
    }

    fn increment_length(&mut self, add_bits: u64) -> Result<()> {
        let first = self.fs.get_page_data_mut(self.first_page_num)?;
        let fp = page_as_mut(first);
        let mut length = (fp.header.length_0 as u64) | ((fp.header.length_1 as u64) << 32);
        length = length.saturating_add(add_bits);
        fp.header.length_0 = (length & 0xFFFF_FFFF) as u32;
        fp.header.length_1 = (length >> 32) as u32;
        Ok(())
    }
}

// ===== Core bitmap reader (skeleton) =====
pub struct BitmapReader<'a> {
    fs: &'a mut FileSystem,
    _first_page_num: u32,
    _current_page_num: u32,
    bit_pos: u64,
    total_bits: u64,
}

impl<'a> BitmapReader<'a> {
    fn new(fs: &'a mut FileSystem, first_page_num: u32) -> Result<Self> {
        if first_page_num == 0 {
            return Err(Error::InvalidOperation("Invalid bitmap page".into()));
        }
        let page = fs.get_page_data(first_page_num)?;
        verify_bitmap_page(page)?;
        let hdr = &page_as_ref(page).header;
        let total_bits = (hdr.length_0 as u64) | ((hdr.length_1 as u64) << 32);
        Ok(Self {
            fs,
            _first_page_num: first_page_num,
            _current_page_num: first_page_num,
            bit_pos: 0,
            total_bits,
        })
    }
}

impl<'a> Iterator for BitmapReader<'a> {
    type Item = bool;
    fn next(&mut self) -> Option<Self::Item> {
        // Implementation will be added in a subsequent step
        let _ = &self.fs; // silence unused for now
        if self.bit_pos >= self.total_bits {
            return None;
        }
        // Placeholder: return false for now and advance
        self.bit_pos += 1;
        Some(false)
    }
}

// ===== Initialization helpers =====
fn init_bitmap_first_page(fs: &mut FileSystem, page: u32) -> Result<()> {
    init_bitmap_page(fs, page, page, page)
}

fn init_bitmap_page(
    fs: &mut FileSystem,
    page: u32,
    last_fill_page: u32,
    last_page: u32,
) -> Result<()> {
    let p = fs.get_page_data_mut(page)?;
    let bp = page_as_mut(p);
    // Ensure page is empty
    if bp.header.magic != 0 {
        return Err(Error::InvalidOperation(
            "Bitmap page already initialized".into(),
        ));
    }

    // Initialize header
    bp.header.magic = BM_MAGIC_WORD;
    let is_first_page = page == last_page;
    bp.header.next_page = u32::MAX;
    bp.header.written_words = if is_first_page { 1 } else { 0 };
    bp.header.cw_offset = 0;
    bp.header.length_0 = 0;
    bp.header.length_1 = 0;
    bp.header.last_page = last_page;
    bp.header.last_fill_page = last_fill_page;
    bp.header.last_fill_pos = 0;
    // zero reserved
    bp.header.reserved = [0; 7];

    // If first page, write initial fill-0 word
    if is_first_page {
        bp.data[0] = wah::FILL_0;
    }
    Ok(())
}

#[inline]
fn verify_bitmap_page(page: &[u8]) -> Result<()> {
    if page.len() < BM_DATA_OFFSET + 8 {
        return Err(Error::InvalidFormat("Bitmap page too small".into()));
    }
    let hdr = &page_as_ref(page).header;
    if hdr.magic != BM_MAGIC_WORD {
        return Err(Error::InvalidFormat("Bitmap magic mismatch".into()));
    }
    Ok(())
}

// Extract up to 63 bits starting at a bit offset from a byte slice, MSB-first in each byte
#[allow(dead_code)]
fn extract_bits_msb(input: &[u8], start_bit: usize, bit_len: usize) -> u64 {
    let mut out: u64 = 0;
    let mut produced = 0usize;
    while produced < bit_len {
        let bit_index = start_bit + produced;
        let byte_idx = bit_index / 8;
        let bit_in_byte = 7 - (bit_index % 8);
        let bit = if byte_idx < input.len() {
            (input[byte_idx] >> bit_in_byte) & 1
        } else {
            0
        } as u64;
        out |= bit << produced;
        produced += 1;
    }
    out
}
