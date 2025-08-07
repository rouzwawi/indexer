# Bitmap Indexer

A high-performance WAH (Word-Aligned Hybrid) compressed bitmap indexing system implemented in Rust.

## Overview

This project provides efficient bitmap indexing and retrieval capabilities using WAH compression. The system is designed for applications requiring fast bitmap operations on large datasets, such as database indexing, data analytics, and information retrieval systems.

## Features

- **WAH Compression**: Space-efficient bitmap storage using Word-Aligned Hybrid compression
- **Memory-Mapped Files**: Efficient file I/O with automatic memory management
- **Hash-Based File System**: Fast file organization using SHA1-based hash tables
- **Safe Memory Management**: Rust's ownership system eliminates memory safety issues
- **High Performance**: Zero-cost abstractions with optimized algorithms
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

### Building

```bash
# Clone the repository
git clone <repository-url>
cd bitmap-indexer

# Build the project
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Usage

```rust
use bitmap_indexer::BitmapIndex;

// Open or create a bitmap index
let mut index = BitmapIndex::open("./data")?;

// Create a new bitmap
let mut writer = index.create_bitmap("example")?;
writer.append_bits(&[0b10101010], 8)?;
writer.fill(true, 1000)?;

// Read the bitmap
let reader = index.open_bitmap("example")?;
for bit in reader.take(10) {
    println!("Bit: {}", bit);
}
```

### Command Line Interface

```bash
# Run the demo application
cargo run --bin bitmap-indexer ./data

# Get help
cargo run --bin bitmap-indexer -- --help
```

## Architecture

The system is organized into several key modules:

- **`types`**: Type definitions and error handling
- **`wah`**: WAH compression implementation
- **`storage`**: Memory-mapped file management and file system
- **`bitmap`**: High-level bitmap operations and iteration
- **`hash`**: SHA1 hashing utilities

## Project Structure

```
bitmap-indexer/
├── Cargo.toml                 # Rust project configuration
├── src/                       # Rust source code
│   ├── lib.rs                # Main library entry point
│   ├── types.rs              # Type definitions and errors
│   ├── wah/                  # WAH compression
│   │   ├── mod.rs
│   │   └── compression.rs
│   ├── storage/              # Storage layer
│   │   ├── mod.rs
│   │   ├── mmf.rs            # Memory-mapped files
│   │   └── filesystem.rs     # Hash-based file system
│   ├── bitmap/               # Bitmap operations
│   │   ├── mod.rs
│   │   ├── bitmap.rs         # Reader/writer
│   │   └── iterator.rs       # Bitmap iteration
│   ├── hash/                 # Hashing utilities
│   │   └── sha1.rs
│   └── bin/
│       └── main.rs           # CLI application
├── tests/                    # Integration tests
├── benches/                  # Benchmarks
├── legacy/                   # Original C++ implementation
│   ├── *.cpp                 # C++ source files
│   ├── headers/              # C++ header files
│   ├── Makefile             # C++ build system
│   └── doc/                 # C++ documentation
└── RUST_MIGRATION_PLAN.md   # Detailed migration plan
```

## Legacy C++ Code

The original C++ implementation has been moved to the `legacy/` directory. This code is preserved for:

- Reference during migration
- Performance comparisons
- Compatibility testing
- Historical documentation

To build the legacy C++ version:

```bash
cd legacy
make
./test
```

## Migration Status

This project is currently in migration from C++ to Rust. See [RUST_MIGRATION_PLAN.md](RUST_MIGRATION_PLAN.md) for detailed migration progress and strategy.

### Current Status
- ✅ Project structure setup
- ✅ Type system migration
- ✅ Basic module structure
- 🚧 WAH compression implementation
- 🚧 Memory-mapped file system
- 🚧 Bitmap operations
- ⏳ Performance optimization
- ⏳ Comprehensive testing

## Performance

The Rust implementation is designed to match or exceed the performance of the original C++ version while providing memory safety guarantees. Benchmarks will be added as the implementation progresses.

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench
```

## Documentation

Generate and view the documentation:

```bash
cargo doc --open
```

## License

This project is licensed under the MIT OR Apache-2.0 license - see the LICENSE files for details.

## Acknowledgments

- Original C++ implementation provided the foundation for this Rust port
- WAH compression algorithm based on academic research
- Built with the excellent Rust ecosystem