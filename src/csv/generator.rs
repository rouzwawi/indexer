//! CSV data generation utilities for testing and benchmarking.
//!
//! This module provides configurable CSV generation with various data distribution patterns.

use std::fmt::Write as FmtWrite;
use std::io::{self, Write};
use std::path::Path;
use std::fs::File;
use anyhow::Result;

/// Supported data distribution patterns for test CSV generation
#[derive(Clone, Copy)]
pub enum ValueDistribution {
    /// Each value appears exactly once
    Uniform,
    /// Some values appear much more frequently than others (80/20 distribution)
    Skewed,
    /// Very few values appear (sparse data)
    Sparse,
    /// Most rows have the same value (dense data)
    Dense,
}

/// Configuration for generating test CSV files
pub struct CsvConfig {
    pub num_rows: usize,
    pub num_columns: usize,
    pub column_prefix: String,
    pub value_prefix: String,
    pub distribution: ValueDistribution,
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            num_rows: 100,
            num_columns: 2,
            column_prefix: "col".to_string(),
            value_prefix: "val".to_string(),
            distribution: ValueDistribution::Uniform,
        }
    }
}

/// Generate a synthetic CSV file with configurable characteristics
pub fn generate_csv(path: &Path, config: &CsvConfig) -> Result<()> {
    let mut file = File::create(path)?;
    generate_csv_to_writer(&mut file, config)?;
    Ok(())
}

/// Generate CSV data to any writer (file, stdout, etc.)
pub fn generate_csv_to_writer<W: Write>(
    writer: &mut W,
    config: &CsvConfig,
) -> Result<()> {
    // Write header
    let mut header = String::new();
    for i in 0..config.num_columns {
        if i > 0 {
            header.push(',');
        }
        write!(header, "{}{}", config.column_prefix, i)?;
    }
    writeln!(writer, "{}", header)?;

    // Generate data rows based on distribution
    for row_idx in 0..config.num_rows {
        let mut row = String::new();

        for col_idx in 0..config.num_columns {
            if col_idx > 0 {
                row.push(',');
            }

            let value = generate_value_for_position(
                row_idx,
                col_idx,
                config.num_rows,
                &config.value_prefix,
                config.distribution
            );
            row.push_str(&value);
        }
        writeln!(writer, "{}", row)?;
    }

    Ok(())
}

/// Generate a value for a specific position based on distribution pattern
fn generate_value_for_position(
    row_idx: usize,
    col_idx: usize,
    total_rows: usize,
    value_prefix: &str,
    distribution: ValueDistribution,
) -> String {
    match distribution {
        ValueDistribution::Uniform => {
            // Each value appears roughly the same number of times
            let num_unique_values = (total_rows as f64).sqrt().max(2.0) as usize;
            let value_idx = (row_idx + col_idx * 1000) % num_unique_values;
            format!("{}{}", value_prefix, value_idx)
        },
        ValueDistribution::Skewed => {
            // 80% of rows use 20% of values (Pareto distribution)
            if row_idx < (total_rows * 8) / 10 {
                // 80% of rows use values 0-1 (20% of value space)
                let value_idx = row_idx % 2;
                format!("{}{}", value_prefix, value_idx)
            } else {
                // 20% of rows use values 2-9 (80% of value space)
                let value_idx = 2 + (row_idx % 8);
                format!("{}{}", value_prefix, value_idx)
            }
        },
        ValueDistribution::Sparse => {
            // Most rows have the same value, few have unique values
            if row_idx < (total_rows * 9) / 10 {
                format!("{}common", value_prefix)
            } else {
                format!("{}rare{}", value_prefix, row_idx)
            }
        },
        ValueDistribution::Dense => {
            // All rows have the same value except a few outliers
            if row_idx < (total_rows * 95) / 100 {
                format!("{}majority", value_prefix)
            } else {
                format!("{}outlier{}", value_prefix, row_idx)
            }
        }
    }
}

/// Create a CSV with special characters and edge cases
pub fn generate_edge_case_csv(path: &Path) -> Result<()> {
    let mut file = File::create(path)?;
    generate_edge_case_csv_to_writer(&mut file)?;
    Ok(())
}

/// Generate edge case CSV to any writer
pub fn generate_edge_case_csv_to_writer<W: Write>(
    writer: &mut W,
) -> Result<()> {
    writeln!(writer, "name,description,category")?;
    writeln!(writer, "simple,Basic value,cat1")?;
    writeln!(writer, "\"quoted,value\",\"Value with, comma\",cat2")?;
    writeln!(writer, "unicode_αβγ,Value with unicode: αβγδε,cat3")?;
    writeln!(writer, "newline_test,\"Value with\nnewline\",cat1")?;
    writeln!(writer, "\"\"\"escaped\"\"\",Triple quoted value,cat2")?;
    writeln!(writer, "empty_field,,cat3")?;
    writeln!(writer, "special!@#$%,Special chars: !@#$%^&*(),cat1")?;

    Ok(())
}

/// Create a single-column CSV for edge case testing
pub fn generate_single_column_csv(path: &Path, num_rows: usize) -> Result<()> {
    let mut file = File::create(path)?;
    generate_single_column_csv_to_writer(&mut file, num_rows)?;
    Ok(())
}

/// Generate single column CSV to any writer
pub fn generate_single_column_csv_to_writer<W: Write>(
    writer: &mut W,
    num_rows: usize,
) -> Result<()> {
    writeln!(writer, "single_col")?;
    for i in 0..num_rows {
        writeln!(writer, "value{}", i % 5)?; // 5 unique values cycling
    }
    Ok(())
}

/// Generate CSV directly to stdout
pub fn generate_csv_to_stdout(config: &CsvConfig) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    generate_csv_to_writer(&mut handle, config)
}
