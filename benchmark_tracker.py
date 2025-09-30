#!/usr/bin/env python3
"""
CSV Ingestion Benchmark Tracker

This script runs the benchmark and tracks performance improvements over time.
Results are stored in a CSV file for analysis.
"""

import subprocess
import re
import csv
import json
from datetime import datetime
from pathlib import Path
import sys
import argparse

RESULTS_FILE = "benchmark_results.csv"
BENCHMARK_LOG = "benchmark_log.json"

def run_benchmark():
    """Run the Rust benchmark and parse the output."""
    try:
        # Build in release mode first
        print("Building in release mode...")
        subprocess.run(["cargo", "build", "--release"], check=True)

        # Run the benchmark
        print("Running benchmark...")
        result = subprocess.run(
            ["cargo", "run", "--release", "--bin", "csv_ingest_bench"],
            capture_output=True,
            text=True,
            check=True
        )

        # Parse the output
        output = result.stdout
        print(output)  # Show the output to the user

        # Extract average duration - look for the Duration section
        duration_match = re.search(r'Duration:\s+Average:\s+([\d.]+)s', output)
        if duration_match:
            avg_duration = float(duration_match.group(1))
        else:
            raise ValueError("Could not parse average duration from output")

        # Extract average throughput - look for the Throughput section
        throughput_match = re.search(r'Throughput:\s+Average:\s+([\d.]+)\s+rows/sec', output)
        if throughput_match:
            avg_throughput = float(throughput_match.group(1))
        else:
            raise ValueError("Could not parse average throughput from output")

        return {
            'duration': avg_duration,
            'throughput': avg_throughput,
            'output': output
        }

    except subprocess.CalledProcessError as e:
        print(f"Error running benchmark: {e}")
        if e.stderr:
            print(f"Error output: {e.stderr}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

def save_result(optimization_name, result, notes=""):
    """Save benchmark result to CSV file."""
    timestamp = datetime.now().isoformat()

    # Check if file exists and read baseline if it does
    baseline_duration = None
    baseline_throughput = None

    if Path(RESULTS_FILE).exists():
        with open(RESULTS_FILE, 'r') as f:
            reader = csv.DictReader(f)
            rows = list(reader)
            if rows:
                # First row is baseline
                baseline_duration = float(rows[0]['duration_seconds'])
                baseline_throughput = float(rows[0]['throughput_rows_per_sec'])

    # Calculate improvements
    if baseline_duration:
        duration_improvement = (baseline_duration - result['duration']) / baseline_duration * 100
        throughput_improvement = (result['throughput'] - baseline_throughput) / baseline_throughput * 100
        speedup = baseline_duration / result['duration']
    else:
        duration_improvement = 0
        throughput_improvement = 0
        speedup = 1.0

    # Prepare the row
    row = {
        'timestamp': timestamp,
        'optimization': optimization_name,
        'duration_seconds': result['duration'],
        'throughput_rows_per_sec': result['throughput'],
        'duration_improvement_pct': duration_improvement,
        'throughput_improvement_pct': throughput_improvement,
        'speedup': speedup,
        'notes': notes
    }

    # Write to CSV
    file_exists = Path(RESULTS_FILE).exists()
    with open(RESULTS_FILE, 'a', newline='') as f:
        fieldnames = ['timestamp', 'optimization', 'duration_seconds',
                     'throughput_rows_per_sec', 'duration_improvement_pct',
                     'throughput_improvement_pct', 'speedup', 'notes']
        writer = csv.DictWriter(f, fieldnames=fieldnames)

        if not file_exists:
            writer.writeheader()

        writer.writerow(row)

    # Also save full output to log
    log_entry = {
        'timestamp': timestamp,
        'optimization': optimization_name,
        'result': result,
        'notes': notes
    }

    log_data = []
    if Path(BENCHMARK_LOG).exists():
        with open(BENCHMARK_LOG, 'r') as f:
            log_data = json.load(f)

    log_data.append(log_entry)

    with open(BENCHMARK_LOG, 'w') as f:
        json.dump(log_data, f, indent=2)

    return row

def print_summary():
    """Print a summary of all benchmark results."""
    if not Path(RESULTS_FILE).exists():
        print("No benchmark results found.")
        return

    with open(RESULTS_FILE, 'r') as f:
        reader = csv.DictReader(f)
        rows = list(reader)

    if not rows:
        print("No benchmark results found.")
        return

    print("\n" + "="*80)
    print("BENCHMARK SUMMARY")
    print("="*80)

    baseline = rows[0]
    print(f"\nBaseline ({baseline['optimization']}):")
    print(f"  Duration: {float(baseline['duration_seconds']):.2f}s")
    print(f"  Throughput: {float(baseline['throughput_rows_per_sec']):.0f} rows/sec")

    if len(rows) > 1:
        print("\nOptimizations:")
        for row in rows[1:]:
            print(f"\n{row['optimization']}:")
            print(f"  Duration: {float(row['duration_seconds']):.2f}s " +
                  f"({float(row['duration_improvement_pct']):.1f}% improvement)")
            print(f"  Throughput: {float(row['throughput_rows_per_sec']):.0f} rows/sec " +
                  f"({float(row['throughput_improvement_pct']):.1f}% improvement)")
            print(f"  Speedup: {float(row['speedup']):.2f}x")
            if row['notes']:
                print(f"  Notes: {row['notes']}")

    print("\n" + "="*80)

def main():
    parser = argparse.ArgumentParser(description='Track CSV ingestion benchmark performance')
    parser.add_argument('optimization', nargs='?', help='Name of the optimization being tested')
    parser.add_argument('--notes', default='', help='Additional notes about this run')
    parser.add_argument('--summary-only', action='store_true', help='Just print summary without running benchmark')

    args = parser.parse_args()

    if args.summary_only:
        print_summary()
        return

    if not args.optimization:
        parser.error("optimization argument is required when not using --summary-only")

    print(f"Running benchmark for: {args.optimization}")
    print("-" * 40)

    # Run the benchmark
    result = run_benchmark()

    # Save the result
    saved = save_result(args.optimization, result, args.notes)

    # Print summary
    print("\n" + "="*40)
    print(f"Results for '{args.optimization}':")
    print(f"  Duration: {saved['duration_seconds']:.2f}s")
    print(f"  Throughput: {saved['throughput_rows_per_sec']:.0f} rows/sec")

    if float(saved['duration_improvement_pct']) != 0:
        print(f"  Improvement: {saved['duration_improvement_pct']:.1f}%")
        print(f"  Speedup: {saved['speedup']:.2f}x")

    # Show overall summary
    print_summary()

if __name__ == "__main__":
    main()
