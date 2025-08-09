# Legacy C++ Implementation

This directory contains the original C++ implementation of the WAH (Word-Aligned Hybrid) compressed bitmap indexing system.

## Status

This C++ implementation is now considered **legacy code** and is being migrated to Rust. The new Rust implementation will provide:

- Memory safety and thread safety
- Modern tooling and dependency management
- Better performance through zero-cost abstractions
- Comprehensive testing and benchmarking

## Original C++ Components

- **Memory Management**: `mmf.hpp/cpp` - Memory-mapped file handling
- **File System Layer**: `fs.hpp/cpp` - SHA1-based hash tables
- **WAH Compression**: `wah.hpp` - 64-bit word-based compression
- **Bitmap Operations**: `bitmap.hpp/cpp` - Core bitmap functionality
- **Bitmap Iteration**: `biterator.hpp/cpp` - Forward iteration
- **Utilities**: SHA1 hashing, type definitions

## Building (Legacy)

To build the legacy C++ version:

```bash
cd legacy
make
```

## Migration

This code is being migrated to Rust as part of a comprehensive modernization effort. See `../RUST_MIGRATION_PLAN.md` for details on the migration strategy and timeline.

The legacy implementation will be maintained for compatibility testing during the migration process.
