//! Main binary for bitmap indexer demonstration and testing
//!
//! This replaces the functionality of test.cpp from the C++ version.

use anyhow::Result;
use bitmap_indexer::BitmapIndex;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <data_directory>", args[0]);
        println!("       {} --help", args[0]);
        return Ok(());
    }

    if args[1] == "--help" {
        print_help();
        return Ok(());
    }

    let data_dir = &args[1];
    println!("Opening bitmap index at: {}", data_dir);

    // Create or open the bitmap index
    let mut index = BitmapIndex::open(data_dir)?;
    println!("Bitmap index opened successfully");

    // Example operations
    demo_bitmap_operations(&mut index)?;

    println!("Demo completed successfully");
    Ok(())
}

fn demo_bitmap_operations(index: &mut BitmapIndex) -> Result<()> {
    println!("\n=== Bitmap Operations Demo ===");

    // Create a test bitmap
    println!("Creating bitmap 'test'...");
    let mut writer = index.create_bitmap("test")?;
    
    // Add some test data
    println!("Adding test data...");
    writer.append_bits(&[0b10101010, 0b11110000], 16)?;
    writer.fill(true, 1000)?;
    writer.fill(false, 500)?;
    
    drop(writer); // Close the writer

    // Read back the data
    println!("Reading bitmap data...");
    let reader = index.open_bitmap("test")?;
    
    let first_20_bits: Vec<bool> = reader.take(20).collect();
    println!("First 20 bits: {:?}", first_20_bits);

    // List all bitmaps
    println!("Available bitmaps: {:?}", index.list_bitmaps()?);

    Ok(())
}

fn print_help() {
    println!("Bitmap Indexer - WAH compressed bitmap indexing system");
    println!();
    println!("USAGE:");
    println!("    bitmap-indexer <data_directory>");
    println!();
    println!("ARGS:");
    println!("    <data_directory>    Directory to store bitmap index files");
    println!();
    println!("OPTIONS:");
    println!("    --help              Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    bitmap-indexer ./data           # Open index in ./data directory");
    println!("    bitmap-indexer /tmp/bitmaps     # Open index in /tmp/bitmaps");
}