# Memory-Mapped File System Architecture

This document provides a comprehensive overview of the memory-mapped file system used by the bitmap indexer. The system is designed for efficient storage and retrieval of large amounts of bitmap data with automatic expansion capabilities.

## Overview

The memory-mapped file system uses a hierarchical structure:

```
File Series → Files → Regions → Pages
```

This design provides:
- **Scalability**: Can grow from MBs to TBs efficiently
- **Memory Efficiency**: Only maps regions that are actually used
- **Performance**: Direct memory access with OS-level caching
- **Reliability**: Automatic persistence through memory mapping
- **State Persistence**: Index file maintains allocated page count across restarts

## Detailed Architecture

### 1. Pages (4 KiB each)

Pages are the smallest unit of data storage:
- **Size**: 4,096 bytes (PAGE_SIZE)
- **Numbering**: Sequential starting from 0
- **Purpose**: Store actual bitmap data and metadata

### 2. Regions (8 MiB each, 2,048 pages)

Regions group pages for efficient memory mapping:
- **Size**: 8 MiB (REGION_SIZE)
- **Pages per region**: 2,048 (8 MiB / 4 KiB)
- **Numbering**: Sequential starting from 0
- **Memory mapping**: Each region is a separate memory-mapped segment

### 3. Files (Variable size)

Files contain one or more regions and grow exponentially:

| File | Regions | Pages    | Size (MiB) | Cumulative Size (MiB) |
|------|---------|----------|------------|------------------------|
| 0    | 1       | 2,048    | 8          | 8                      |
| 1    | 2       | 4,096    | 16         | 24                     |
| 2    | 4       | 8,192    | 32         | 56                     |
| 3    | 8       | 16,384   | 64         | 120                    |
| 4    | 16      | 32,768   | 128        | 248                    |
| 5    | 32      | 65,536   | 256        | 504                    |
| 6    | 64      | 131,072  | 512        | 1,016                  |
| 7    | 128     | 262,144  | 1,024      | 2,040                  |
| 8    | 128     | 262,144  | 1,024      | 3,064                  |
| ...  | 128     | 262,144  | 1,024      | ...                    |

### 4. File Series

The complete collection of files forms the file series, providing virtually unlimited storage capacity.

### 5. Index File (.idx)

A small metadata file that persists system state:
- **Size**: 64 bytes (matching legacy implementation)
- **Content**: Next available page number (first 4 bytes)
- **Purpose**: Maintain state across process restarts
- **Location**: Same directory as data files, with `.idx` extension

## Mathematical Relationships

### File Size Calculation

```rust
fn file_size(file: u32) -> u64 {
    let regions = if file >= 7 { 128 } else { 1 << file };
    regions as u64 * 8_388_608  // 8 MiB per region
}
```

### Region Distribution

Files 0-6 use exponential growth:
- File 0: 2^0 = 1 region
- File 1: 2^1 = 2 regions
- File 2: 2^2 = 4 regions
- ...
- File 6: 2^6 = 64 regions

Files 7+ use fixed size:
- File 7+: 128 regions each

### Region Addressing

Regions are numbered sequentially across all files:

```
File 0: Region 0
File 1: Regions 1-2
File 2: Regions 3-6
File 3: Regions 7-14
File 4: Regions 15-30
File 5: Regions 31-62
File 6: Regions 63-126
File 7: Regions 127-254
File 8: Regions 255-382
...
```

### Address Translation Functions

#### `regions_up_to(file)`
Returns total regions in files 0 through file-1:

```rust
fn regions_up_to(file: u32) -> u32 {
    if file == 0 {
        0
    } else if file <= 7 {
        (1 << file) - 1  // Geometric series sum
    } else {
        127 + 128 * (file - 7)  // Fixed increment
    }
}
```

#### `file_addr(region)`
Determines which file contains a given region:

```rust
fn file_addr(region: u32) -> u32 {
    let mut file = 0;
    while regions_up_to(file + 1) <= region {
        file += 1;
    }
    file
}
```

#### Page to Region Translation
```rust
fn region_for_page(page: u32) -> u32 {
    page / 2048  // 2048 pages per region
}

fn page_offset_in_region(page: u32) -> u32 {
    page % 2048  // Page offset within region
}
```

## Memory Management

### Lazy Loading

Regions are only memory-mapped when first accessed:

1. **Page Request**: Application requests page N
2. **Region Lookup**: Calculate which region contains page N
3. **Check Mapping**: Is region already mapped?
4. **Map if Needed**: If not mapped, create memory mapping
5. **Return Address**: Provide pointer to page data

### File Creation

Files are created on-demand:

1. **Region Request**: Need to map region R
2. **File Lookup**: Calculate which file contains region R
3. **File Creation**: Create file if it doesn't exist
4. **Size Setting**: Set file to correct size for its number
5. **Memory Mapping**: Map the specific region within the file

### Example Memory Layout

For a system with 10,000 allocated pages:

```
Pages 0-2,047:     Region 0  (File 0)
Pages 2,048-4,095: Region 1  (File 1)
Pages 4,096-6,143: Region 2  (File 1)
Pages 6,144-8,191: Region 3  (File 2)
Pages 8,192-9,999: Region 4 (File 2, partial)
```

Physical files on disk:
- `data.d0000`: 8 MiB (1 region)
- `data.d0001`: 16 MiB (2 regions)
- `data.d0002`: 32 MiB (4 regions, 2 used)
- `data.idx`: 64 bytes (metadata)

Memory mappings:
- Region 0: Mapped to `data.d0000` offset 0
- Region 1: Mapped to `data.d0001` offset 0
- Region 2: Mapped to `data.d0001` offset 8 MiB
- Region 3: Mapped to `data.d0002` offset 0
- Region 4: Mapped to `data.d0002` offset 8 MiB
- Index: Mapped to `data.idx` (persistent state)

## Performance Characteristics

### Space Efficiency

- **Early Growth**: Small files for small datasets
- **Large Scale**: Efficient for multi-GiB datasets
- **Sparse Access**: Only used regions consume memory

### Time Complexity

- **Page Allocation**: O(1)
- **Page Access**: O(1) after initial mapping
- **Region Mapping**: O(1) amortized
- **Address Translation**: O(1)

### Memory Usage

- **Virtual Memory**: Maps entire regions (8 MiB each)
- **Physical Memory**: Only pages actually accessed
- **OS Optimization**: Leverages OS virtual memory system

## Integration with Bitmap Indexer

The memory-mapped file system serves as the foundation for:

1. **Hash Table Storage**: File system metadata and directory
2. **Bitmap Data**: Compressed bitmap pages
3. **Index Structures**: B-tree nodes and other index data
4. **Metadata**: File headers, statistics, and configuration

Each bitmap file uses this system for:
- **Automatic Growth**: No pre-allocation needed
- **Efficient Access**: Direct memory access to data
- **Persistence**: Automatic write-through to disk
- **Crash Recovery**: Memory mappings survive process restart

## Usage Examples

### Basic Usage

```rust
use bitmap_indexer::storage::MemoryMappedFile;

// Create memory-mapped file system
let mmf = MemoryMappedFile::new("data/bitmap.mmf")?;

// Allocate pages
let page1 = mmf.allocate_page()?;  // Returns 0
let page2 = mmf.allocate_page()?;  // Returns 1

// Write data
{
    let data = mmf.get_page_mut(page1)?;
    data[0..8].copy_from_slice(b"bitmap01");
}

// Read data
{
    let data = mmf.get_page(page1)?;
    assert_eq!(&data[0..8], b"bitmap01");
}

// Get statistics
let stats = mmf.stats();
println!("Allocated pages: {}", stats.allocated_pages);
println!("Active regions: {}", stats.active_regions);
println!("Total mapped: {} MiB", stats.total_mapped_bytes / 1_048_576);

// Manually sync index if needed
mmf.sync_index()?;
```

### Large Scale Usage

```rust
// Allocate 1 million pages (4 GiB of data)
let mut pages = Vec::new();
for _ in 0..1_000_000 {
    pages.push(mmf.allocate_page()?);
}

// This will automatically create:
// - data.d0000: 1 region (8 MiB)
// - data.d0001: 2 regions (16 MiB)
// - data.d0002: 4 regions (32 MiB)
// - data.d0003: 8 regions (64 MiB)
// - data.d0004: 16 regions (128 MiB)
// - data.d0005: 32 regions (256 MiB)
// - data.d0006: 64 regions (512 MiB)
// - data.d0007: 128 regions (1024 MiB)
// - data.d0008: 128 regions (1024 MiB)
// - data.d0009: 128 regions (1024 MiB)
// - data.d000a: ~100 regions (~800 MiB)

println!("Created {} files", 11);
println!("Total size: ~4 GiB");
```

This architecture provides a robust, scalable foundation for the bitmap indexer's storage needs while maintaining excellent performance characteristics and memory efficiency.
