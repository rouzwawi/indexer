//! Command line utility with cat and tee commands.
//!
//! This tool provides Unix-like cat and tee functionality using the memory-mapped filesystem.

use bitmap_indexer::storage::FileSystem;
use bitmap_indexer::storage::mmf::MemoryMappedFile;
use std::env;
use std::io::{self, BufRead, BufReader};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: {} <index_path> <command> [args...]", args[0]);
        println!("Commands:");
        println!("  cat <filename>     - Read file and print to stdout");
        println!("  tee <filename>     - Read stdin, write to file and stdout");
        return Ok(());
    }

    let index_path = &args[1];
    let command = &args[2];

    // Initialize the memory-mapped filesystem
    let mmf = MemoryMappedFile::new(index_path)?;
    let mut fs = if mmf.allocated_pages() == 0 {
        FileSystem::init(mmf)?
    } else {
        FileSystem::new(mmf)?
    };

    match command.as_str() {
        "cat" => {
            if args.len() != 4 {
                println!("Usage: {} {} cat <filename>", args[0], args[1]);
                return Ok(());
            }
            cat_command(&mut fs, &args[3])?;
        },
        "tee" => {
            if args.len() < 4 {
                println!("Usage: {} {} tee <filename>", args[0], args[1]);
                return Ok(());
            }
            let overwrite = args.len() > 4 && (args[4] == "--overwrite" || args[4] == "-o");
            tee_command(&mut fs, &args[3], overwrite)?;
        },
        _ => {
            println!("Unknown command: {}", command);
            println!("Available commands: cat, tee");
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
    let content_end = page_data.iter().position(|&b| b == 0).unwrap_or(page_data.len());
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
