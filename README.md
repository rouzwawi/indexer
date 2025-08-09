# Bitmap Indexer

A modern Rust implementation of a WAH (Word-Aligned Hybrid) compressed bitmap indexing system for efficient data indexing and retrieval.

## Features

- **Memory-mapped file management** with automatic region allocation
- **Hash-based file system** for organizing indexed data
- **WAH compression** for space-efficient bitmap storage
- **Bitmap operations** (append, fill, iteration)
- **SHA1-based file identification**
- **Memory safety** and **thread safety** through Rust's ownership system

## Project Status

This is a **migration in progress** from a C++ implementation to Rust. The project is currently in **Phase 1** of the migration plan.

### Completed
- ✅ Project structure setup
- ✅ Core type definitions
- ✅ WAH compression module
- ✅ Memory-mapped file management
- ✅ Hash-based file system foundation
- ✅ SHA1 hashing utilities

### In Progress
- 🚧 Bitmap operations implementation
- 🚧 Bitmap iteration
- 🚧 Integration testing

### Planned
- ⏳ Performance optimization
- ⏳ Comprehensive benchmarking
- ⏳ Documentation and examples
- ⏳ Migration from legacy C++ data files

## Building

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench

# Build and run the binary
cargo run -- /path/to/index
```

## Usage

```rust
use bitmap_indexer::BitmapIndex;

// Open or create a bitmap index
let mut index = BitmapIndex::open("data/index")?;

// Create a new bitmap
let mut writer = index.create_bitmap("my_bitmap")?;

// Write some data
writer.fill(true, 1000)?;
writer.append_bits(&[0xFF, 0x00], 16)?;

// Read the bitmap
let reader = index.open_bitmap("my_bitmap")?;
for bit in reader.take(10) {
    println!("Bit: {}", bit);
}
```

## Legacy C++ Implementation

The original C++ implementation has been moved to the `legacy/` directory. See `legacy/LEGACY_README.md` for information about the original codebase.

## Migration Plan

This project follows a comprehensive migration plan outlined in `RUST_MIGRATION_PLAN.md`. The migration is designed to:

1. Maintain performance and functionality parity
2. Add memory safety and thread safety
3. Provide modern tooling and development experience
4. Ensure compatibility with existing data files

## Architecture

The Rust implementation is organized into several key modules:

- **`types`**: Core type definitions and error handling
- **`wah`**: WAH compression implementation
- **`storage`**: Memory-mapped files and file system
- **`bitmap`**: Bitmap operations and management
- **`hash`**: SHA1 hashing utilities

## Contributing

This project is currently in active migration. Please see the migration plan for current priorities and areas where contributions would be most valuable.

## License

MIT OR Apache-2.0
