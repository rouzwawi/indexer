# C++ to Rust Migration Plan: Bitmap Indexer

## Project Overview

This project is a **WAH (Word-Aligned Hybrid) compressed bitmap indexing system** designed for efficient data indexing and retrieval. The system provides:

- Memory-mapped file management with automatic region allocation
- Hash-based file system for organizing indexed data
- WAH compression for space-efficient bitmap storage
- Bitmap operations (append, fill, iteration)
- SHA1-based file identification

## Current Architecture Analysis

### Core Components

1. **Memory Management (`mmf.hpp/cpp`)**
   - Memory-mapped file handling with boost::interprocess
   - Automatic file expansion with exponential growth (8 MiB → 16 MiB → 32 MiB...)
   - Page-based allocation (4 KiB pages)
   - Region mapping for efficient memory usage

2. **File System Layer (`fs.hpp/cpp`)**
   - SHA1-based hash tables for file organization
   - Collision handling with linked hash table pages
   - File creation and lookup operations

3. **WAH Compression (`wah.hpp`)**
   - 64-bit word-based compression
   - Fill words for run-length encoding
   - Literal words for uncompressible data
   - Bit manipulation utilities

4. **Bitmap Operations (`bitmap.hpp/cpp`)**
   - Append operations with word splitting
   - Fill operations for efficient run encoding
   - Page management for large bitmaps
   - Header management for metadata

5. **Bitmap Iteration (`biterator.hpp/cpp`)**
   - Forward iteration through compressed bitmaps
   - Decompression of WAH encoded data

6. **Utilities**
   - SHA1 hashing (boost + custom wrapper)
   - Type definitions (u4=uint32_t, u8=uint64_t)
   - FastDelegate (for callbacks, potentially unused)

### Dependencies
- **Boost Libraries**: interprocess, static_assert, cstdint, lexical_cast, date_time
- **Standard Libraries**: iostream, map, list, iterator, algorithm
- **System**: Memory mapping, file I/O

## Migration Strategy

### Phase 1: Project Setup & Foundation

#### 1.1 Rust Project Structure
```
bitmap-indexer-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── types.rs          # typedefs.hpp equivalent
│   ├── wah/
│   │   ├── mod.rs        # wah.hpp equivalent
│   │   └── compression.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── mmf.rs        # mmf.hpp/cpp equivalent
│   │   └── filesystem.rs # fs.hpp/cpp equivalent
│   ├── bitmap/
│   │   ├── mod.rs
│   │   ├── bitmap.rs     # bitmap.hpp/cpp equivalent
│   │   └── iterator.rs   # biterator.hpp/cpp equivalent
│   ├── hash/
│   │   └── sha1.rs       # sha1.hpp equivalent
│   └── bin/
│       └── main.rs       # test.cpp equivalent
├── tests/
├── benches/
└── README.md
```

#### 1.2 Cargo.toml Dependencies
```toml
[dependencies]
memmap2 = "0.9"           # Memory-mapped files (replaces boost::interprocess)
sha1 = "0.10"             # SHA1 hashing (replaces boost + custom)
byteorder = "1.5"         # Byte order handling
thiserror = "1.0"         # Error handling
anyhow = "1.0"            # Error context
serde = { version = "1.0", features = ["derive"] }  # Serialization

[dev-dependencies]
criterion = "0.5"         # Benchmarking
tempfile = "3.8"          # Temporary files for testing
```

### Phase 2: Core Module Migration

#### 2.1 Type System (`types.rs`)
- Replace C++ typedefs with Rust type aliases
- Use `u32` and `u64` directly instead of `u4`/`u8`
- Define constants using `const` instead of `#define`
- Implement proper error types with `thiserror`

#### 2.2 WAH Compression (`wah/`)
```rust
// Key improvements in Rust version:
pub struct WahWord(u64);

impl WahWord {
    const FILL_FLAG: u64 = 0x8000_0000_0000_0000;
    const FILL_VAL: u64 = 0x4000_0000_0000_0000;
    // ... other constants

    pub fn is_fill(&self) -> bool { /* ... */ }
    pub fn fill_value(&self) -> bool { /* ... */ }
    pub fn fill_count(&self) -> u32 { /* ... */ }
}
```

#### 2.3 Memory-Mapped Files (`storage/mmf.rs`)
- Replace boost::interprocess with `memmap2` crate
- Use `std::collections::HashMap` instead of `std::map`
- Implement proper RAII with Rust's ownership system
- Add comprehensive error handling

```rust
pub struct MemoryMappedFile {
    mappings: HashMap<usize, memmap2::MmapMut>,
    next_page: AtomicU32,
    base_path: PathBuf,
}

impl MemoryMappedFile {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> { /* ... */ }
    pub fn allocate_page(&self) -> Result<u32> { /* ... */ }
    pub fn get_page(&self, page: u32) -> Result<&mut [u8]> { /* ... */ }
}
```

#### 2.4 File System Layer (`storage/filesystem.rs`)
- Replace C-style structs with Rust structs
- Use `serde` for serialization instead of manual memory layout
- Implement proper hash table with collision resolution
- Add type safety for page addresses

#### 2.5 Bitmap Operations (`bitmap/bitmap.rs`)
- Convert C++ classes to Rust structs with associated functions
- Use Rust's ownership system instead of manual memory management
- Implement `Iterator` trait for bitmap iteration
- Add comprehensive bounds checking

### Phase 3: Advanced Features & Optimizations

#### 3.1 Safety Improvements
- Replace all `unsafe` pointer operations with safe Rust equivalents
- Add comprehensive input validation
- Implement proper error propagation with `Result<T, E>`
- Use Rust's type system to prevent common C++ errors

#### 3.2 Performance Optimizations
- Use `#[inline]` for hot path functions
- Implement SIMD operations where appropriate
- Use `Vec<T>` with proper capacity planning
- Add benchmarks to ensure performance matches C++ version

#### 3.3 Modern Rust Features
- Implement `Iterator` traits for bitmap traversal
- Use `serde` for serialization/deserialization
- Add proper `Debug`, `Clone`, `PartialEq` derives where appropriate
- Implement custom `Error` types with context

### Phase 4: Testing & Validation

#### 4.1 Unit Tests
- Port existing C++ test cases to Rust
- Add property-based testing with `proptest`
- Test edge cases that C++ version might miss
- Validate memory safety with `miri`

#### 4.2 Integration Tests
- Test complete bitmap lifecycle (create, append, iterate, close)
- Validate file format compatibility with C++ version
- Test large dataset handling
- Validate concurrent access patterns

#### 4.3 Benchmarks
- Compare performance with original C++ implementation
- Measure memory usage and allocation patterns
- Profile hot paths with `cargo flamegraph`
- Optimize based on benchmark results

### Phase 5: API Design & Documentation

#### 5.1 Public API Design
```rust
// Clean, idiomatic Rust API
pub struct BitmapIndex {
    storage: MemoryMappedFile,
    filesystem: FileSystem,
}

impl BitmapIndex {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>;
    pub fn create_bitmap(&mut self, name: &str) -> Result<BitmapWriter>;
    pub fn open_bitmap(&self, name: &str) -> Result<BitmapReader>;
}

pub struct BitmapWriter { /* ... */ }
impl BitmapWriter {
    pub fn append_bits(&mut self, bits: &[u8], count: usize) -> Result<()>;
    pub fn fill(&mut self, value: bool, count: usize) -> Result<()>;
}

pub struct BitmapReader { /* ... */ }
impl Iterator for BitmapReader {
    type Item = bool;
    fn next(&mut self) -> Option<Self::Item> { /* ... */ }
}
```

#### 5.2 Documentation
- Comprehensive rustdoc documentation
- Code examples for common use cases
- Migration guide from C++ version
- Performance comparison documentation

### Phase 6: Deployment & Migration

#### 6.1 Compatibility Layer
- Ensure file format compatibility with C++ version
- Provide tools for migrating existing data files
- Document any breaking changes in file format

#### 6.2 Feature Parity Validation
- Verify all C++ functionality is available in Rust version
- Test with real-world datasets
- Validate performance meets or exceeds C++ version

## Benefits of Rust Migration

### Safety Benefits
- **Memory Safety**: Eliminates segfaults, buffer overflows, and use-after-free bugs
- **Thread Safety**: Rust's ownership system prevents data races
- **Type Safety**: Strong type system catches errors at compile time

### Performance Benefits
- **Zero-cost abstractions**: High-level code without runtime overhead
- **Better optimization**: LLVM backend with modern optimization passes
- **Memory efficiency**: No garbage collector, predictable memory usage

### Maintenance Benefits
- **Modern tooling**: Cargo for dependency management, rustfmt, clippy
- **Better testing**: Built-in testing framework and benchmarking
- **Documentation**: Integrated documentation generation

### Ecosystem Benefits
- **Active ecosystem**: Growing collection of high-quality crates
- **Cross-platform**: Better support for different architectures
- **Future-proof**: Language designed for long-term evolution

## Risk Mitigation

### Performance Risks
- **Mitigation**: Extensive benchmarking throughout migration
- **Fallback**: Keep C++ version available during transition

### Compatibility Risks
- **Mitigation**: Thorough testing with existing data files
- **Fallback**: Provide migration tools and format conversion utilities

### Timeline Risks
- **Mitigation**: Incremental migration with working milestones
- **Validation**: Each phase fully tested before proceeding

## Estimated Timeline

- **Phase 1 (Setup)**: 1-2 weeks
- **Phase 2 (Core Migration)**: 4-6 weeks
- **Phase 3 (Optimizations)**: 2-3 weeks
- **Phase 4 (Testing)**: 2-3 weeks
- **Phase 5 (Documentation)**: 1-2 weeks
- **Phase 6 (Deployment)**: 1-2 weeks

**Total Estimated Time**: 11-18 weeks

## Next Steps

1. **Review and approve this migration plan**
2. **Set up Rust development environment**
3. **Create initial Rust project structure**
4. **Begin Phase 1: Project setup and foundation**
5. **Establish benchmarking baseline with C++ version**

This migration will modernize the codebase while maintaining performance and adding the safety and reliability benefits of Rust.
