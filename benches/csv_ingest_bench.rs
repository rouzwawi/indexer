use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use bitmap_indexer::csv::{ingest_csv, ingest_csv_streaming};
use std::collections::HashSet;

/// Benchmark result for a single run
#[derive(Debug, Clone)]
struct BenchmarkResult {
    duration: Duration,
    rows_per_second: f64,
}

/// Generate a random 8-character alphanumeric string for run ID
fn generate_run_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use timestamp and thread ID for uniqueness
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Convert to base36 for alphanumeric string
    let chars = "0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = String::new();
    let mut value = timestamp;

    for _ in 0..8 {
        let index = (value % 36) as usize;
        result.push(chars.chars().nth(index).unwrap());
        value /= 36;
    }

    result
}

/// Run a single benchmark of CSV ingestion
fn run_single_benchmark(csv_path: &str, row_count: usize, run_paths: &mut HashSet<PathBuf>, use_streaming: bool) -> BenchmarkResult {
    // Create benchmarks directory if it doesn't exist
    let benchmarks_dir = Path::new("benchmarks");
    if !benchmarks_dir.exists() {
        fs::create_dir(benchmarks_dir).ok();
    }

    // Generate unique run ID and create run directory
    let run_id = generate_run_id();
    let run_dir = benchmarks_dir.join(&run_id);
    fs::create_dir_all(&run_dir).ok();

    // Track this path for cleanup
    run_paths.insert(run_dir.clone());

    // Create index path within run directory
    let index_path = run_dir.join("bench.idx");

    // Run the ingestion and time it
    let start = Instant::now();
    if use_streaming {
        // Use a reasonable chunk size for streaming (10,000 rows per chunk)
        ingest_csv_streaming(index_path.to_str().unwrap(), csv_path, 256*1024).expect("Streaming ingestion failed");
    } else {
        ingest_csv(index_path.to_str().unwrap(), csv_path).expect("Ingestion failed");
    }
    let duration = start.elapsed();

    // Calculate throughput
    let rows_per_second = row_count as f64 / duration.as_secs_f64();

    // Note: We don't clean up individual runs here, will clean up all at once

    BenchmarkResult {
        duration,
        rows_per_second,
    }
}

/// Run multiple iterations and compute statistics
fn run_benchmark(
    name: &str,
    csv_path: &str,
    row_count: usize,
    iterations: usize,
    use_streaming: bool,
) {
    println!("\n=== Benchmark: {} ===", name);
    println!("CSV File: {}", csv_path);
    println!("Rows: {}", row_count);
    println!("Iterations: {}", iterations);
    println!();

    let mut results = Vec::new();
    let mut run_paths = HashSet::new();

    for i in 1..=iterations {
        print!("  Run {}/{}...", i, iterations);
        let result = run_single_benchmark(csv_path, row_count, &mut run_paths, use_streaming);
        println!(" {:.2}s ({:.0} rows/sec)",
                 result.duration.as_secs_f64(),
                 result.rows_per_second);
        results.push(result);
    }

    // Clean up all run directories
    for path in run_paths {
        fs::remove_dir_all(path).ok();
    }

    // Calculate statistics
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let avg_duration = total_duration / iterations as u32;

    let min_duration = results.iter().map(|r| r.duration).min().unwrap();
    let max_duration = results.iter().map(|r| r.duration).max().unwrap();

    let avg_throughput = results.iter().map(|r| r.rows_per_second).sum::<f64>() / iterations as f64;
    let min_throughput = results.iter().map(|r| r.rows_per_second).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_throughput = results.iter().map(|r| r.rows_per_second).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

    println!("\nResults:");
    println!("  Duration:");
    println!("    Average: {:.2}s", avg_duration.as_secs_f64());
    println!("    Min:     {:.2}s", min_duration.as_secs_f64());
    println!("    Max:     {:.2}s", max_duration.as_secs_f64());
    println!("  Throughput:");
    println!("    Average: {:.0} rows/sec", avg_throughput);
    println!("    Min:     {:.0} rows/sec", min_throughput);
    println!("    Max:     {:.0} rows/sec", max_throughput);
}

fn main() {
    // Configuration
    let csv_path = "1m-s.csv";
    let row_count = 1_000_000;

    // Check if CSV file exists
    if !Path::new(csv_path).exists() {
        eprintln!("Error: CSV file '{}' not found", csv_path);
        eprintln!("Please ensure the 1m-s.csv file exists in the current directory");
        eprintln!("You can generate it with:");
        eprintln!("  ./mmfs data/test generate-csv -r 1000000 -c 3 -d skewed > 1m-s.csv");
        std::process::exit(1);
    }

    // Warm-up run (not counted)
    println!("Performing warm-up run...");
    let mut warmup_paths = HashSet::new();
    run_single_benchmark(csv_path, row_count, &mut warmup_paths, false);
    // Clean up warmup run
    for path in warmup_paths {
        fs::remove_dir_all(path).ok();
    }

    // Run the single-pass benchmark
    run_benchmark(
        "CSV Ingestion - Single Pass",
        csv_path,
        row_count,
        3, // Number of iterations
        false, // Not streaming
    );

    // Run the streaming benchmark
    run_benchmark(
        "CSV Ingestion - Streaming (10K chunks)",
        csv_path,
        row_count,
        3, // Number of iterations
        true, // Use streaming
    );

    // Final cleanup - remove entire benchmarks directory
    if Path::new("benchmarks").exists() {
        fs::remove_dir_all("benchmarks").ok();
    }
}
