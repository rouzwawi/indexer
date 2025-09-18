//! CSV ingestion functionality for bitmap indexing.
//!
//! This module provides functionality to parse CSV files and create bitmap indexes
//! for efficient querying. Each unique value in each column gets its own bitmap
//! indicating which rows contain that value.

pub mod generator;

use crate::bitmap::BitmapIndex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// WAH-compatible BitSet implementation using 63-bit data words
struct WahBitSet {
    words: Vec<u64>,
    size: usize,
}

impl WahBitSet {
    /// Create a new WahBitSet with the given capacity
    fn new(size: usize) -> Self {
        // WAH uses 63-bit data words
        let num_words = (size + 62) / 63;
        WahBitSet {
            words: vec![0u64; num_words],
            size,
        }
    }

    /// Set the bit at the given index
    fn set(&mut self, index: usize) {
        if index < self.size {
            // WAH uses 63-bit words
            let word_idx = index / 63;
            let bit_idx = index % 63;
            self.words[word_idx] |= 1u64 << bit_idx;
        }
    }

    #[allow(dead_code)]
    fn contains(&self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }
        let word_idx = index / 63;
        let bit_idx = index % 63;
        (self.words[word_idx] & (1u64 << bit_idx)) != 0
    }

    /// Get the raw WAH-compatible words
    fn into_words(self) -> Vec<u64> {
        // Already in WAH format, just return the words
        if self.words.is_empty() {
            vec![0u64]
        } else {
            self.words
        }
    }

    /// Count the number of set bits (for reporting)
    fn count_ones(&self) -> usize {
        self.words.iter()
            .map(|&w| w.count_ones() as usize)
            .sum()
    }
}

/// Ingest a CSV file and create bitmap indexes for all columns and values.
///
/// For each column in the CSV, creates bitmaps named `{base_name}/{column}/{value}`
/// where each bitmap indicates which rows contain the specified value in that column.
///
/// # Arguments
/// * `index_path` - Path to the bitmap index file
/// * `csv_file` - Path to the CSV file to ingest
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(anyhow::Error)` on any failure during parsing or bitmap creation
///
/// # Example
/// ```no_run
/// use bitmap_indexer::csv::ingest_csv;
///
/// ingest_csv("index.mmf", "data.csv").expect("Failed to ingest CSV");
/// ```
pub fn ingest_csv(index_path: &str, csv_file: &str) -> anyhow::Result<()> {
    // Open the CSV file and create bitmap index
    let mut index = BitmapIndex::open(index_path)?;

    // Extract base filename (remove path and extension)
    let csv_path = Path::new(csv_file);
    let base_name = csv_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid CSV filename: {}", csv_file))?;

    // Read and parse CSV
    let mut reader = csv::Reader::from_path(csv_file)?;
    let headers = reader.headers()?.clone();
    let column_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

    // Collect all records to determine row count and build value mappings
    let mut records = Vec::new();
    for result in reader.records() {
        let record = result?;
        records.push(record);
    }

    let row_count = records.len();
    println!("Processing {} rows with {} columns", row_count, column_names.len());

    // Build all bitsets in a single pass over the records
    // Using compound keys: "columnName/value"
    let mut all_bitsets: HashMap<String, WahBitSet> = HashMap::new();

    // Single iteration over all records
    for (row_idx, record) in records.iter().enumerate() {
        // Process all columns for this record
        for (col_idx, column_name) in column_names.iter().enumerate() {
            if let Some(value) = record.get(col_idx) {
                // Create compound key for this column/value combination
                let key = format!("{}/{}", column_name, value);
                let bitset = all_bitsets
                    .entry(key)
                    .or_insert_with(|| WahBitSet::new(row_count));
                bitset.set(row_idx);
            }
        }
    }

    // Write all bitmaps to index
    for (key, bitset) in all_bitsets {
        let bitmap_name = format!("{}/{}", base_name, key);
        let mut writer = index.create_bitmap(&bitmap_name)?;

        // Get count before consuming the bitset
        let set_count = bitset.count_ones();

        // Get WAH-compatible words directly
        let bit_data = bitset.into_words();

        // Append the bits to the bitmap
        writer.append_bits(&bit_data, row_count)?;
        writer.close()?;

        println!("Created bitmap: {} ({} rows set)", bitmap_name, set_count);
    }

    println!("CSV ingestion completed successfully");
    Ok(())
}

/// Memory-efficient streaming version of CSV ingestion.
///
/// This version processes the CSV file in chunks, making it suitable for
/// files too large to fit in memory.
pub fn ingest_csv_streaming(index_path: &str, csv_file: &str, chunk_size: usize) -> anyhow::Result<()> {
    // Ensure chunk size is multiple of WAH word size (63)
    let chunk_size = if chunk_size % 63 != 0 {
        ((chunk_size / 63) + 1) * 63
    } else {
        chunk_size
    };

    // First pass: Count rows and get column names
    let mut reader = csv::Reader::from_path(csv_file)?;
    let column_names: Vec<String> = reader
        .headers()?
        .iter()
        .map(|h| h.to_string())
        .collect();

    // Get base name from CSV file
    let base_name = Path::new(csv_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data");

    println!("Processing CSV file: {}", csv_file);
    println!("Base name: {}", base_name);
    println!("Column names: {:?}", column_names);
    println!("Chunk size: {}", chunk_size);

    // Track which bitmaps we've created
    let mut created_bitmaps: HashSet<String> = HashSet::new();

    let mut chunk = Vec::with_capacity(chunk_size);
    let mut base_row_idx = 0;
    let mut index = BitmapIndex::open(index_path)?;

    // Count total rows
    let mut row_count = 0;
    for record_result in reader.records() {
        let record = record_result?;
        chunk.push(record);
        row_count += 1;

        // Process chunk when it reaches the desired size
        if chunk.len() >= chunk_size {
            println!("Processing chunk: {} ({} rows processed total)", base_row_idx/chunk_size, row_count);
            process_and_flush_chunk(
                &chunk,
                &column_names,
                &mut created_bitmaps,
                base_row_idx,
                chunk_size,
                &base_name,
                &mut index,
            )?;
            base_row_idx += chunk.len();
            chunk.clear();
        }
    }

    // Process remaining records (pad to chunk_size with zeros)
    if !chunk.is_empty() {
        let actual_size = chunk.len();
        // Pad the logical size to be a multiple of 63
        let padded_size = if actual_size % 63 != 0 {
            ((actual_size / 63) + 1) * 63
        } else {
            actual_size
        };

        println!("Processing chunk: {} ({} rows processed total)", base_row_idx/chunk_size, row_count);
        process_and_flush_chunk(
            &chunk,
            &column_names,
            &mut created_bitmaps,
            base_row_idx,
            padded_size,
            &base_name,
            &mut index,
        )?;
        base_row_idx += padded_size;
    }

    // Final pass: ensure all bitmaps have the correct total length
    // base_row_idx is the total bits written so far (possibly padded)
    // row_count is the actual number of rows in the CSV
    // If we've written less than row_count, pad all bitmaps
    if base_row_idx < row_count {
        let remaining = row_count - base_row_idx;
        for key in &created_bitmaps {
            let bitmap_name = format!("{}/{}", base_name, key);
            let mut writer = index.open_or_create_bitmap(&bitmap_name)?;
            writer.fill(false, remaining)?;
            writer.close()?;
        }
    }
    Ok(())
}

/// Process a chunk and immediately flush to disk
fn process_and_flush_chunk(
    chunk: &[csv::StringRecord],
    column_names: &[String],
    created_bitmaps: &mut HashSet<String>,
    base_row_idx: usize,
    chunk_logical_size: usize,
    base_name: &str,
    index: &mut BitmapIndex,
) -> anyhow::Result<()> {

    let mut chunk_bitsets: HashMap<String, WahBitSet> = HashMap::new();
    for (chunk_idx, record) in chunk.iter().enumerate() {
        for (col_idx, column_name) in column_names.iter().enumerate() {
            if let Some(value) = record.get(col_idx) {
                let key = format!("{}/{}", column_name, value);
                chunk_bitsets.entry(key)
                    .or_insert_with(|| WahBitSet::new(chunk_logical_size))
                    .set(chunk_idx);
            }
        }
    }

    let all_values: HashSet<String> = chunk_bitsets.keys().chain(created_bitmaps.iter()).cloned().collect();

    // Now process each value - either with data from chunk or zeros
    for key in &all_values {
        let is_new = !created_bitmaps.contains(key);
        let bitmap_name = format!("{}/{}", base_name, key);

        if is_new {
            // New value - create bitmap and backfill
            let mut writer = index.create_bitmap(&bitmap_name)?;

            // Backfill zeros for all previous chunks
            if base_row_idx > 0 {
                writer.fill(false, base_row_idx)?;
                println!("Backfilled new bitmap: {} ({} rows set)", bitmap_name, base_row_idx);
            }

            // Write this chunk's data
            let bitset = chunk_bitsets.remove(key).unwrap();
            let set_count = bitset.count_ones();
            let bits = bitset.into_words();
            writer.append_bits(&bits, chunk_logical_size)?;
            writer.close()?;

            println!("Created new bitmap: {} ({} rows set)", bitmap_name, set_count);

            created_bitmaps.insert(key.clone());
        } else {
            // Existing bitmap - just append this chunk's data
            let mut writer = index.open_or_create_bitmap(&bitmap_name)?;

            // Append this chunk's data if it exists
            if chunk_bitsets.contains_key(key) {
                let bitset = chunk_bitsets.remove(key).unwrap();
                let set_count = bitset.count_ones();
                let bits = bitset.into_words();
                writer.append_bits(&bits, chunk_logical_size)?;
                println!("Wrote to existing bitmap: {} ({} rows set)", bitmap_name, set_count);
            } else {
                // Otherwise fill with zeros
                writer.fill(false, chunk_logical_size)?;
                println!("Filled existing bitmap: {} ({} rows set)", bitmap_name, chunk_logical_size);
            }
            writer.close()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_wah_bitset() {
        let mut bitset = WahBitSet::new(200);

        // Set some bits across word boundaries
        bitset.set(0);
        bitset.set(62);  // Last bit of first word (63-bit)
        bitset.set(63);  // First bit of second word
        bitset.set(125); // Middle of second word
        bitset.set(126); // First bit of third word
        bitset.set(199); // Last valid bit

        // Check they're set
        assert!(bitset.contains(0));
        assert!(bitset.contains(62));
        assert!(bitset.contains(63));
        assert!(bitset.contains(125));
        assert!(bitset.contains(126));
        assert!(bitset.contains(199));

        // Check unset bits
        assert!(!bitset.contains(1));
        assert!(!bitset.contains(61));
        assert!(!bitset.contains(64));
        assert!(!bitset.contains(127));
        assert!(!bitset.contains(200)); // Out of bounds

        // Test word conversion
        let words = bitset.into_words();
        assert_eq!(words.len(), 4); // ceil(200/63) = 4

        // First word should have bits 0 and 62 set
        assert_eq!(words[0] & 1, 1); // bit 0
        assert_eq!((words[0] >> 62) & 1, 1); // bit 62

        // Second word should have bit 0 (global bit 63) set
        assert_eq!(words[1] & 1, 1);
    }

    #[test]
    fn test_wah_bitset_count() {
        let mut bitset = WahBitSet::new(100);

        // Set exactly 10 bits
        for i in (0..100).step_by(10) {
            bitset.set(i);
        }

        assert_eq!(bitset.count_ones(), 10);
    }

    #[test]
    fn test_streaming_ingestion() {
        let tmp = TempDir::new().unwrap();
        let csv_path = tmp.path().join("test_stream.csv");
        let index_path = tmp.path().join("test_stream.idx");

        // Create a test CSV with enough rows to span multiple chunks
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "col1,col2").unwrap();

        // Write 200 rows to test chunking (with chunk size 63)
        for i in 0..200 {
            let val1 = if i < 100 { "a" } else { "b" };  // 'a' appears in first half, 'b' in second
            let val2 = match i % 3 {
                0 => "x",
                1 => "y",
                _ => "z",
            };
            writeln!(file, "{},{}", val1, val2).unwrap();
        }

        // Test streaming with small chunk size
        ingest_csv_streaming(
            index_path.to_str().unwrap(),
            csv_path.to_str().unwrap(),
            63,  // Small chunk size to test multiple chunks
        ).unwrap();

        // Verify the bitmaps were created correctly
        let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

        // Check that all expected bitmaps exist and count bits
        let count_a = {
            let reader = index.open_bitmap("test_stream/col1/a").unwrap();
            let bits: Vec<bool> = reader.take(200).collect(); // Only count first 200 bits
            bits.iter().filter(|&&b| b).count()
        };
        let count_b = {
            let reader = index.open_bitmap("test_stream/col1/b").unwrap();
            let bits: Vec<bool> = reader.take(200).collect(); // Only count first 200 bits
            bits.iter().filter(|&&b| b).count()
        };
        let count_x = {
            let reader = index.open_bitmap("test_stream/col2/x").unwrap();
            let bits: Vec<bool> = reader.take(200).collect();
            bits.iter().filter(|&&b| b).count()
        };
        let count_y = {
            let reader = index.open_bitmap("test_stream/col2/y").unwrap();
            let bits: Vec<bool> = reader.take(200).collect();
            bits.iter().filter(|&&b| b).count()
        };
        let count_z = {
            let reader = index.open_bitmap("test_stream/col2/z").unwrap();
            let bits: Vec<bool> = reader.take(200).collect();
            bits.iter().filter(|&&b| b).count()
        };

        // Verify counts
        assert_eq!(count_a, 100);  // 'a' appears 100 times
        assert_eq!(count_b, 100);  // 'b' appears 100 times

        // x, y, z should each appear ~67 times (200/3)
        assert!(count_x >= 66 && count_x <= 68);
        assert!(count_y >= 66 && count_y <= 68);
        assert!(count_z >= 66 && count_z <= 68);
        assert_eq!(count_x + count_y + count_z, 200);
    }

    #[test]
    fn test_ingest_simple_csv() {
        let tmp = TempDir::new().unwrap();
        let csv_path = tmp.path().join("test.csv");
        let index_path = tmp.path().join("index.mmf");

        // Create simple CSV
        let mut file = File::create(&csv_path).unwrap();
        writeln!(file, "col1,col2").unwrap();
        writeln!(file, "a,x").unwrap();
        writeln!(file, "b,y").unwrap();
        writeln!(file, "a,z").unwrap();

        // Ingest CSV
        ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

        // Verify bitmaps were created by opening the index
        let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

        // Should be able to open bitmaps for each value
        let _reader_a = index.open_bitmap("test/col1/a").unwrap();
        let _reader_b = index.open_bitmap("test/col1/b").unwrap();
        let _reader_x = index.open_bitmap("test/col2/x").unwrap();
        let _reader_y = index.open_bitmap("test/col2/y").unwrap();
        let _reader_z = index.open_bitmap("test/col2/z").unwrap();
    }
}
