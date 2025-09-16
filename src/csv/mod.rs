//! CSV ingestion functionality for bitmap indexing.
//!
//! This module provides functionality to parse CSV files and create bitmap indexes
//! for efficient querying. Each unique value in each column gets its own bitmap
//! indicating which rows contain that value.

use crate::bitmap::BitmapIndex;
use std::collections::HashMap;
use std::path::Path;

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

    // Build value-to-bitmap mappings for each column
    for (col_idx, column_name) in column_names.iter().enumerate() {
        // Map each unique value to a list of row indices where it appears
        let mut value_rows: HashMap<String, Vec<usize>> = HashMap::new();

        for (row_idx, record) in records.iter().enumerate() {
            if let Some(value) = record.get(col_idx) {
                value_rows.entry(value.to_string()).or_default().push(row_idx);
            }
        }

        // Create bitmap for each unique value in this column
        for (value, row_indices) in value_rows {
            let bitmap_name = format!("{}/{}/{}", base_name, column_name, value);
            let mut writer = index.create_bitmap(&bitmap_name)?;

            // Create bit pattern: set bits to 1 for rows where this value appears
            let mut bit_data = vec![0u64];
            let mut current_word = 0u64;
            let mut current_bit = 0;

            for row_idx in 0..row_count {
                if row_indices.contains(&row_idx) {
                    current_word |= 1u64 << current_bit;
                }
                current_bit += 1;

                // Move to next word if we've filled 63 bits (WAH uses 63-bit words)
                if current_bit >= 63 {
                    let current_word_index = bit_data.len() - 1;
                    bit_data[current_word_index] = current_word;
                    if row_idx + 1 < row_count {
                        bit_data.push(0u64);
                        current_word = 0;
                        current_bit = 0;
                    }
                }
            }

            // Write the final word if we have remaining bits
            if current_bit > 0 {
                let last_index = bit_data.len() - 1;
                bit_data[last_index] = current_word;
            }

            // Append the bits to the bitmap
            writer.append_bits(&bit_data, row_count)?;
            writer.close()?;

            println!("Created bitmap: {} ({} rows set)", bitmap_name, row_indices.len());
        }
    }

    println!("CSV ingestion completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

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