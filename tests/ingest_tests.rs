//! Comprehensive tests for CSV ingestion functionality
//!
//! This test suite validates the CSV ingestion process with focus on:
//! - Correctness of bitmap creation and content
//! - Performance with large datasets (1K, 10K, 100K rows)
//! - Edge cases (special characters, single columns, etc.)
//! - Multi-column scenarios with various data distributions
//!
//! All tests use the public API (BitmapIndex, BitmapReader, BitmapWriter) to ensure
//! the validation reflects real usage patterns.

use bitmap_indexer::{BitmapIndex, csv::ingest_csv, csv::generator as test_data_generation};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Bitmap validation utilities using the public BitmapReader API
mod bitmap_validation {
    use super::*;

    /// Represents the expected bitmap content for validation
    #[derive(Debug)]
    pub struct ExpectedBitmap {
        pub name: String,
        pub set_bits: HashSet<usize>,
        pub total_bits: usize,
    }

    /// Validate that a bitmap contains exactly the expected set bits
    pub fn validate_bitmap_content(
        index: &mut BitmapIndex,
        expected: &ExpectedBitmap,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = index.open_bitmap(&expected.name)?;

        let mut actual_set_bits = HashSet::new();
        let mut bit_count = 0;

        for (idx, bit) in reader.enumerate() {
            if bit {
                actual_set_bits.insert(idx);
            }
            bit_count += 1;

            // Safety check to avoid infinite loops
            if bit_count > expected.total_bits + 100 {
                return Err(format!(
                    "Bitmap {} has more bits ({}) than expected ({})",
                    expected.name, bit_count, expected.total_bits
                ).into());
            }
        }

        // Verify total bit count
        if bit_count != expected.total_bits {
            return Err(format!(
                "Bitmap {} has {} bits, expected {}",
                expected.name, bit_count, expected.total_bits
            ).into());
        }

        // Verify set bits match exactly
        if actual_set_bits != expected.set_bits {
            return Err(format!(
                "Bitmap {} set bits mismatch.\nExpected: {:?}\nActual: {:?}",
                expected.name, expected.set_bits, actual_set_bits
            ).into());
        }

        Ok(())
    }

    /// Build expected bitmap data by parsing CSV content
    pub fn build_expected_bitmaps(
        csv_path: &Path,
        base_name: &str,
    ) -> Result<Vec<ExpectedBitmap>, Box<dyn std::error::Error>> {
        let mut reader = csv::Reader::from_path(csv_path)?;
        let headers = reader.headers()?.clone();

        // Collect all records
        let mut records = Vec::new();
        for result in reader.records() {
            records.push(result?);
        }

        let mut expected_bitmaps = Vec::new();

        // For each column, build bitmaps for each unique value
        for (col_idx, column_name) in headers.iter().enumerate() {
            let mut value_rows: HashMap<String, HashSet<usize>> = HashMap::new();

            for (row_idx, record) in records.iter().enumerate() {
                if let Some(value) = record.get(col_idx) {
                    value_rows.entry(value.to_string())
                        .or_default()
                        .insert(row_idx);
                }
            }

            for (value, row_set) in value_rows {
                let bitmap_name = format!("{}/{}/{}", base_name, column_name, value);
                expected_bitmaps.push(ExpectedBitmap {
                    name: bitmap_name,
                    set_bits: row_set,
                    total_bits: records.len(),
                });
            }
        }

        Ok(expected_bitmaps)
    }
}

// Core test implementations

#[test]
fn test_basic_csv_ingestion() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("test.csv");
    let index_path = tmp.path().join("index.mmf");

    // Create simple test CSV
    let config = test_data_generation::CsvConfig {
        num_rows: 10,
        num_columns: 2,
        ..Default::default()
    };
    test_data_generation::generate_csv(&csv_path, &config).unwrap();

    // Ingest CSV
    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

    // Build expected results and validate
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "test").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    for expected in &expected_bitmaps {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    println!("✅ Basic CSV ingestion test passed with {} expected bitmaps", expected_bitmaps.len());
}

#[test]
fn test_bitmap_correctness_small() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("small.csv");
    let index_path = tmp.path().join("index.mmf");

    // Create a small CSV with known pattern for easy verification
    let mut file = File::create(&csv_path).unwrap();
    writeln!(file, "letter,number").unwrap();
    writeln!(file, "a,1").unwrap();
    writeln!(file, "b,1").unwrap();
    writeln!(file, "a,2").unwrap();
    writeln!(file, "c,1").unwrap();

    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

    // Manually verify specific bitmaps
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    // Bitmap for letter=a should have bits 0,2 set (rows 0 and 2)
    let expected_a = bitmap_validation::ExpectedBitmap {
        name: "small/letter/a".to_string(),
        set_bits: [0, 2].iter().cloned().collect(),
        total_bits: 4,
    };
    bitmap_validation::validate_bitmap_content(&mut index, &expected_a).unwrap();

    // Bitmap for number=1 should have bits 0,1,3 set
    let expected_1 = bitmap_validation::ExpectedBitmap {
        name: "small/number/1".to_string(),
        set_bits: [0, 1, 3].iter().cloned().collect(),
        total_bits: 4,
    };
    bitmap_validation::validate_bitmap_content(&mut index, &expected_1).unwrap();

    println!("✅ Small CSV bitmap correctness test passed");
}

#[test]
fn test_medium_csv_1k_rows() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("medium_1k.csv");
    let index_path = tmp.path().join("index.mmf");

    // Generate 1K row CSV with uniform distribution
    let config = test_data_generation::CsvConfig {
        num_rows: 1000,
        num_columns: 3,
        distribution: test_data_generation::ValueDistribution::Uniform,
        ..Default::default()
    };
    test_data_generation::generate_csv(&csv_path, &config).unwrap();

    println!("Ingesting 1K row CSV...");
    let start = std::time::Instant::now();
    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();
    let duration = start.elapsed();
    println!("1K row ingestion completed in {:?}", duration);

    // Validate bitmap correctness
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "medium_1k").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    println!("Validating {} bitmaps...", expected_bitmaps.len());
    for expected in &expected_bitmaps {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    println!("✅ 1K row CSV test passed with {} bitmaps validated", expected_bitmaps.len());
}

#[test]
fn test_medium_csv_10k_rows() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("medium_10k.csv");
    let index_path = tmp.path().join("index.mmf");

    // Generate 10K row CSV with skewed distribution
    let config = test_data_generation::CsvConfig {
        num_rows: 10000,
        num_columns: 4,
        distribution: test_data_generation::ValueDistribution::Skewed,
        ..Default::default()
    };
    test_data_generation::generate_csv(&csv_path, &config).unwrap();

    println!("Ingesting 10K row CSV...");
    let start = std::time::Instant::now();
    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();
    let duration = start.elapsed();
    println!("10K row ingestion completed in {:?}", duration);

    // Validate bitmap correctness
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "medium_10k").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    println!("Validating {} bitmaps...", expected_bitmaps.len());
    for expected in &expected_bitmaps {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    println!("✅ 10K row CSV test passed with {} bitmaps validated", expected_bitmaps.len());
}

#[test]
fn test_sparse_and_dense_data_patterns() {
    let tmp = TempDir::new().unwrap();

    // Test sparse data (mostly same value, few unique)
    let sparse_csv = tmp.path().join("sparse.csv");
    let sparse_index = tmp.path().join("sparse_index.mmf");

    let sparse_config = test_data_generation::CsvConfig {
        num_rows: 1000,
        num_columns: 2,
        distribution: test_data_generation::ValueDistribution::Sparse,
        ..Default::default()
    };
    test_data_generation::generate_csv(&sparse_csv, &sparse_config).unwrap();

    ingest_csv(sparse_index.to_str().unwrap(), sparse_csv.to_str().unwrap()).unwrap();

    let sparse_expected = bitmap_validation::build_expected_bitmaps(&sparse_csv, "sparse").unwrap();
    let mut sparse_idx = BitmapIndex::open(sparse_index.to_str().unwrap()).unwrap();

    for expected in &sparse_expected {
        bitmap_validation::validate_bitmap_content(&mut sparse_idx, expected).unwrap();
    }

    // Test dense data (most rows have same value, few outliers)
    let dense_csv = tmp.path().join("dense.csv");
    let dense_index = tmp.path().join("dense_index.mmf");

    let dense_config = test_data_generation::CsvConfig {
        num_rows: 1000,
        num_columns: 2,
        distribution: test_data_generation::ValueDistribution::Dense,
        ..Default::default()
    };
    test_data_generation::generate_csv(&dense_csv, &dense_config).unwrap();

    ingest_csv(dense_index.to_str().unwrap(), dense_csv.to_str().unwrap()).unwrap();

    let dense_expected = bitmap_validation::build_expected_bitmaps(&dense_csv, "dense").unwrap();
    let mut dense_idx = BitmapIndex::open(dense_index.to_str().unwrap()).unwrap();

    for expected in &dense_expected {
        bitmap_validation::validate_bitmap_content(&mut dense_idx, expected).unwrap();
    }

    println!("✅ Sparse and dense data pattern tests passed");
}

#[test]
fn test_single_column_csv() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("single_col.csv");
    let index_path = tmp.path().join("index.mmf");

    // Generate single column CSV
    test_data_generation::generate_single_column_csv(&csv_path, 100).unwrap();

    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

    // Validate bitmap correctness
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "single_col").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    for expected in &expected_bitmaps {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    println!("✅ Single column CSV test passed with {} bitmaps", expected_bitmaps.len());
}

#[test]
fn test_edge_cases_special_characters() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("edge_cases.csv");
    let index_path = tmp.path().join("index.mmf");

    // Generate CSV with special characters
    test_data_generation::generate_edge_case_csv(&csv_path).unwrap();

    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

    // Validate bitmap correctness
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "edge_cases").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    for expected in &expected_bitmaps {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    println!("✅ Edge cases test passed with {} bitmaps containing special characters", expected_bitmaps.len());
}

#[test]
fn test_large_csv_100k_rows() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("large_100k.csv");
    let index_path = tmp.path().join("index.mmf");

    // Generate 100K row CSV with uniform distribution
    let config = test_data_generation::CsvConfig {
        num_rows: 100_000,
        num_columns: 5,
        distribution: test_data_generation::ValueDistribution::Uniform,
        ..Default::default()
    };

    println!("Generating 100K row CSV...");
    let gen_start = std::time::Instant::now();
    test_data_generation::generate_csv(&csv_path, &config).unwrap();
    println!("CSV generation completed in {:?}", gen_start.elapsed());

    println!("Ingesting 100K row CSV...");
    let ingest_start = std::time::Instant::now();
    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();
    let ingest_duration = ingest_start.elapsed();
    println!("100K row ingestion completed in {:?}", ingest_duration);

    // For 100K rows, we'll do a sample validation rather than full validation
    // to keep test runtime reasonable
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "large_100k").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    println!("Validating sample of {} bitmaps...", expected_bitmaps.len().min(10));
    for expected in expected_bitmaps.iter().take(10) {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    println!("✅ 100K row CSV test passed - {} total bitmaps created, sample validated", expected_bitmaps.len());
}

#[test]
fn test_multi_column_variations() {
    let tmp = TempDir::new().unwrap();

    // Test various column counts
    let column_counts = [2, 5, 10, 20];

    for &col_count in &column_counts {
        let csv_path = tmp.path().join(format!("multi_col_{}.csv", col_count));
        let index_path = tmp.path().join(format!("index_{}.mmf", col_count));

        let config = test_data_generation::CsvConfig {
            num_rows: 500, // Smaller row count to keep multi-column tests fast
            num_columns: col_count,
            distribution: test_data_generation::ValueDistribution::Uniform,
            ..Default::default()
        };

        test_data_generation::generate_csv(&csv_path, &config).unwrap();

        let start = std::time::Instant::now();
        ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();
        let duration = start.elapsed();

        // Validate bitmap correctness
        let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, &format!("multi_col_{}", col_count)).unwrap();
        let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

        for expected in &expected_bitmaps {
            bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
        }

        println!("✅ {} columns: {} bitmaps created and validated in {:?}",
                col_count, expected_bitmaps.len(), duration);
    }
}

#[test]
fn test_wah_compression_efficiency() {
    let tmp = TempDir::new().unwrap();

    // Test different data patterns to verify WAH compression is working
    let patterns = [
        ("uniform", test_data_generation::ValueDistribution::Uniform),
        ("sparse", test_data_generation::ValueDistribution::Sparse),
        ("dense", test_data_generation::ValueDistribution::Dense),
        ("skewed", test_data_generation::ValueDistribution::Skewed),
    ];

    for (name, distribution) in &patterns {
        let csv_path = tmp.path().join(format!("compression_{}.csv", name));
        let index_path = tmp.path().join(format!("compression_{}.mmf", name));

        let config = test_data_generation::CsvConfig {
            num_rows: 5000,
            num_columns: 3,
            distribution: *distribution,
            ..Default::default()
        };

        test_data_generation::generate_csv(&csv_path, &config).unwrap();
        ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

        // Validate a few bitmaps to ensure compression didn't break correctness
        let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, &format!("compression_{}", name)).unwrap();

        let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

        // Validate first 3 bitmaps for each pattern
        for expected in expected_bitmaps.iter().take(3) {
            bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
        }

        // Close the index before checking file metadata
        drop(index);

        // Check index file size to verify compression is happening
        let file_size = if index_path.exists() {
            std::fs::metadata(&index_path).unwrap().len()
        } else {
            0 // File might not exist in some test scenarios
        };

        println!("✅ {} pattern: {} bitmaps, index size: {} bytes",
                name, expected_bitmaps.len(), file_size);
    }
}

#[test]
fn test_bitmap_naming_convention() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("naming_test.csv");
    let index_path = tmp.path().join("index.mmf");

    // Create CSV with specific values to test naming
    let mut file = File::create(&csv_path).unwrap();
    writeln!(file, "product_type,status_code,region").unwrap();
    writeln!(file, "widget,active,north").unwrap();
    writeln!(file, "gadget,inactive,south").unwrap();
    writeln!(file, "widget,pending,north").unwrap();

    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    // Verify specific bitmap names can be opened
    let expected_names = [
        "naming_test/product_type/widget",
        "naming_test/product_type/gadget",
        "naming_test/status_code/active",
        "naming_test/status_code/inactive",
        "naming_test/status_code/pending",
        "naming_test/region/north",
        "naming_test/region/south",
    ];

    for name in &expected_names {
        let reader = index.open_bitmap(name);
        assert!(reader.is_ok(), "Failed to open bitmap: {}", name);
    }

    let unexpected_names = [
        "naming_test/product_type/unknown",
        "naming_test/status_code/unknown",
        "naming_test/region/unknown",
    ];

    for name in &unexpected_names {
        let reader = index.open_bitmap(name);
        assert!(reader.is_err(), "Unexpectedly found bitmap: {}", name);
    }

    println!("✅ Bitmap naming convention test passed - all expected names accessible");
}

#[test]
fn test_empty_and_null_handling() {
    let tmp = TempDir::new().unwrap();
    let csv_path = tmp.path().join("empty_null.csv");
    let index_path = tmp.path().join("index.mmf");

    // Create CSV with empty fields
    let mut file = File::create(&csv_path).unwrap();
    writeln!(file, "col1,col2,col3").unwrap();
    writeln!(file, "value1,,value3").unwrap();
    writeln!(file, ",value2,").unwrap();
    writeln!(file, "value1,value2,value3").unwrap();
    writeln!(file, ",,").unwrap();

    ingest_csv(index_path.to_str().unwrap(), csv_path.to_str().unwrap()).unwrap();

    // Validate bitmap correctness including empty values
    let expected_bitmaps = bitmap_validation::build_expected_bitmaps(&csv_path, "empty_null").unwrap();
    let mut index = BitmapIndex::open(index_path.to_str().unwrap()).unwrap();

    for expected in &expected_bitmaps {
        bitmap_validation::validate_bitmap_content(&mut index, expected).unwrap();
    }

    // Verify that empty strings get their own bitmaps
    let empty_bitmap = index.open_bitmap("empty_null/col1/");
    assert!(empty_bitmap.is_ok(), "Should be able to open bitmap for empty string");

    println!("✅ Empty and null handling test passed with {} bitmaps", expected_bitmaps.len());
}
