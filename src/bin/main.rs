//! Command line utility with cat and tee commands.
//!
//! This tool provides Unix-like cat and tee functionality using the memory-mapped filesystem.

use bitmap_indexer::csv;
use bitmap_indexer::storage::mmf::MemoryMappedFile;
use bitmap_indexer::storage::FileSystem;
use std::env;
use std::io::{self, BufRead, BufReader};

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
    Ingest { csv_file: String, chunk_size: usize },
    GenerateCsv { rows: usize, columns: usize, distribution: String, special: bool },
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
            let mut chunk_size = 256 * 1024;
            let mut csv_file = String::new();

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--chunk-size" | "-c" => {
                        chunk_size = args.next()
                            .ok_or_else(|| anyhow::anyhow!("ingest command requires <chunk_size> argument"))?
                            .parse::<usize>()
                            .map_err(|_| anyhow::anyhow!("Invalid number for chunk_size: {}", chunk_size))?;
                    }
                    _ if csv_file.is_empty() => {
                        csv_file = arg;
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown option for ingest: {}", arg));
                    }
                }
            }

            if csv_file.is_empty() {
                return Err(anyhow::anyhow!("ingest command requires <csv_file> argument"));
            }

            Command::Ingest { csv_file, chunk_size }
        }
        "generate-csv" => {
            let mut rows = 100;
            let mut columns = 2;
            let mut distribution = "uniform".to_string();
            let mut special = false;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--rows" | "-r" => {
                        rows = args.next()
                            .ok_or_else(|| anyhow::anyhow!("--rows requires a value"))?
                            .parse::<usize>()
                            .map_err(|_| anyhow::anyhow!("Invalid number for rows"))?;
                    }
                    "--columns" | "-c" => {
                        columns = args.next()
                            .ok_or_else(|| anyhow::anyhow!("--columns requires a value"))?
                            .parse::<usize>()
                            .map_err(|_| anyhow::anyhow!("Invalid number for columns"))?;
                    }
                    "--distribution" | "-d" => {
                        distribution = args.next()
                            .ok_or_else(|| anyhow::anyhow!("--distribution requires a value"))?;
                        if !["uniform", "skewed", "sparse", "dense"].contains(&distribution.as_str()) {
                            return Err(anyhow::anyhow!("Invalid distribution: {}. Must be one of: uniform, skewed, sparse, dense", distribution));
                        }
                    }
                    "--special" => {
                        special = true;
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown option for generate-csv: {}", arg));
                    }
                }
            }
            Command::GenerateCsv { rows, columns, distribution, special }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown command: {}. Available commands: cat, tee, fill, ingest, generate-csv", command_str));
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
    println!("  ingest <csv_file> [-c|--chunk-size <size>] - Ingest CSV data into bitmap indexes");
    println!("    Options:");
    println!("      --chunk-size, -c <size> - Chunk size (default: 16384)");
    println!("  generate-csv [options]      - Generate CSV data to stdout");
    println!("    Options:");
    println!("      --rows, -r <num>        - Number of rows (default: 100)");
    println!("      --columns, -c <num>     - Number of columns (default: 2)");
    println!("      --distribution, -d <type> - Value distribution: uniform, skewed, sparse, dense (default: uniform)");
    println!("      --special               - Generate edge case CSV with special characters");
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

    // GenerateCsv command doesn't need filesystem initialization
    if let Command::GenerateCsv { rows, columns, distribution, special } = args.command {
        if special {
            csv::generator::generate_edge_case_csv_to_writer(&mut std::io::stdout())?;
        } else {
            let config = csv::generator::CsvConfig {
                num_rows: rows,
                num_columns: columns,
                column_prefix: "col".to_string(),
                value_prefix: "val".to_string(),
                distribution: match distribution.as_str() {
                    "skewed" => csv::generator::ValueDistribution::Skewed,
                    "sparse" => csv::generator::ValueDistribution::Sparse,
                    "dense" => csv::generator::ValueDistribution::Dense,
                    _ => csv::generator::ValueDistribution::Uniform,
                },
            };
            csv::generator::generate_csv_to_stdout(&config)?;
        }
        return Ok(());
    }

    // Initialize the memory-mapped filesystem for other commands
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
        Command::Ingest { csv_file, chunk_size } => {
            csv::ingest_csv_streaming(&args.index_path, &csv_file, chunk_size)?;
        }
        Command::GenerateCsv { .. } => {
            unreachable!("GenerateCsv was already handled above");
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
