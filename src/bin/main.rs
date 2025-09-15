//! Command line utility with cat and tee commands.
//!
//! This tool provides Unix-like cat and tee functionality using the memory-mapped filesystem.

use bitmap_indexer::bitmap::BitmapIndex;
use bitmap_indexer::storage::mmf::MemoryMappedFile;
use bitmap_indexer::storage::FileSystem;
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
struct Args {
    _program_name: String,
    index_path: String,
    command: Command,
}

#[derive(Debug)]
enum Command {
    Cat { filename: String },
    Tee { filename: String, overwrite: bool },
    Fill { num_files: u32 },
    Ingest { csv_file: String },
}

fn parse_args() -> anyhow::Result<Args> {
    let mut args = env::args();
    let program_name = args.next().unwrap_or_else(|| "bitmap-indexer".to_string());

    let index_path = args.next()
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: <index_path>"))?;

    let command_str = args.next()
        .ok_or_else(|| anyhow::anyhow!("Missing required argument: <command>"))?;

    let command = match command_str.as_str() {
        "cat" => {
            let filename = args.next()
                .ok_or_else(|| anyhow::anyhow!("cat command requires <filename> argument"))?;
            Command::Cat { filename }
        }
        "tee" => {
            let filename = args.next()
                .ok_or_else(|| anyhow::anyhow!("tee command requires <filename> argument"))?;

            let overwrite = args.any(|arg| arg == "--overwrite" || arg == "-o");
            Command::Tee { filename, overwrite }
        }
        "fill" => {
            let num_files_str = args.next()
                .ok_or_else(|| anyhow::anyhow!("fill command requires <num_files> argument"))?;
            let num_files = num_files_str.parse::<u32>()
                .map_err(|_| anyhow::anyhow!("Invalid number for num_files: {}", num_files_str))?;
            Command::Fill { num_files }
        }
        "ingest" => {
            let csv_file = args.next()
                .ok_or_else(|| anyhow::anyhow!("ingest command requires <csv_file> argument"))?;
            Command::Ingest { csv_file }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown command: {}. Available commands: cat, tee, fill, ingest", command_str));
        }
    };

    Ok(Args {
        _program_name: program_name,
        index_path,
        command,
    })
}

fn print_usage(program_name: &str) {
    println!("Usage: {} <index_path> <command> [args...]", program_name);
    println!();
    println!("Commands:");
    println!("  cat <filename>              - Read file and print to stdout");
    println!("  tee <filename> [-o|--overwrite] - Read stdin, write to file and stdout");
    println!("  fill <num_files>            - Fill files with sequential numbers");
    println!("  ingest <csv_file>           - Ingest CSV data into bitmap indexes");
}

fn main() -> anyhow::Result<()> {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            print_usage(&env::args().next().unwrap_or_else(|| "bitmap-indexer".to_string()));
            std::process::exit(1);
        }
    };

    // Initialize the memory-mapped filesystem
    let mmf = MemoryMappedFile::new(&args.index_path)?;
    let mut fs = if mmf.allocated_pages() == 0 {
        FileSystem::init(mmf)?
    } else {
        FileSystem::new(mmf)?
    };

    match args.command {
        Command::Cat { filename } => {
            cat_command(&mut fs, &filename)?;
        }
        Command::Tee { filename, overwrite } => {
            tee_command(&mut fs, &filename, overwrite)?;
        }
        Command::Fill { num_files } => {
            fill_command(&mut fs, num_files)?;
            println!("Fill done");
        }
        Command::Ingest { csv_file } => {
            ingest_command(&args.index_path, &csv_file)?;
        }
    }

    Ok(())
}

/// Implementation of cat command - reads file from memory-mapped filesystem and prints to stdout
fn cat_command(fs: &mut FileSystem, filename: &str) -> anyhow::Result<()> {
    let file_page = fs.get_file_page(filename)?;
    if file_page == 0 {
        return Err(anyhow::anyhow!("File not found: {}", filename));
    }

    // Read the page content directly
    let page_data = fs.get_page_data(file_page)?;

    // Find the end of the content (look for null terminator or use full page)
    let content_end = page_data
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(page_data.len());
    let content = &page_data[..content_end];

    // Convert to string and print
    match std::str::from_utf8(content) {
        Ok(text) => print!("{}", text),
        Err(_) => {
            // If not valid UTF-8, print as binary data
            for &byte in content {
                print!("{}", byte as char);
            }
        }
    }

    Ok(())
}

/// Implementation of tee command - reads stdin, writes to memory-mapped filesystem and stdout
fn tee_command(fs: &mut FileSystem, filename: &str, overwrite: bool) -> anyhow::Result<()> {
    let existing_file_page = fs.get_file_page(filename)?;
    let file_page = if existing_file_page != 0 && !overwrite {
        existing_file_page
    } else {
        fs.create_file(filename)?
    };

    // Read all stdin content
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    let mut content = Vec::new();

    for line in reader.lines() {
        let line = line?;
        // Echo to stdout
        println!("{}", line);
        // Collect content for writing to file
        content.extend_from_slice(line.as_bytes());
        content.push(b'\n');
    }

    // Write content to the file page
    let page_data = fs.get_page_data_mut(file_page)?;
    let content_end = page_data.iter().position(|&b| b == 0).unwrap_or(0);
    let copy_len = std::cmp::min(content.len(), page_data.len() - content_end);
    page_data[content_end..content_end + copy_len].copy_from_slice(&content[..copy_len]);

    // Null-terminate if there's space
    if content_end + copy_len < page_data.len() {
        page_data[content_end + copy_len] = 0;
    }

    // Sync the page to ensure it's written
    fs.sync_page(file_page)?;

    Ok(())
}

fn fill_command(fs: &mut FileSystem, files: u32) -> anyhow::Result<()> {
    for i in 0..files {
        let filename = format!("fillfile_{}", i);
        let file_page = fs.create_file(&filename)?;
        let page_data = fs.get_page_data_mut(file_page)?;
        page_data.fill(i as u8);
    }
    println!("Max collision level: {}", fs.max_collision_level);
    println!("Allocated tables: {}", fs.allocated_tables);
    fs.mmf().sync_all()?;
    Ok(())
}

fn ingest_command(index_path: &str, csv_file: &str) -> anyhow::Result<()> {
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
