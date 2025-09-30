# Flame Graph Performance Analysis - 1m-s.csv Streaming Ingest

## Summary
Successfully profiled the streaming CSV ingest process for `1m-s.csv` (1 million rows) using cargo-flamegraph. The flame graph is saved as `flamegraph.svg` and can be viewed in your browser.

## Key Observations from Processing Output

### Processing Statistics
- **Total rows processed**: 1,000,000 rows
- **Chunk size**: 262,206 rows
- **Total chunks**: 4 chunks
- **Columns**: 3 columns (col0, col1, col2)

### Performance Patterns Observed

1. **Chunk 0 (Initial)**: Created 6 new bitmaps
   - All operations were bitmap creation
   - No backfilling required

2. **Chunks 1-2 (Middle)**: Wrote to existing bitmaps only
   - Simple append operations
   - Most efficient processing

3. **Chunk 3 (Final)**: Heavy backfilling operations
   - Created 24 new bitmaps
   - Each new bitmap required backfilling 786,618 rows
   - This is the most expensive chunk by far

## Identified Performance Bottlenecks

Based on the output patterns and code analysis, the major performance bottlenecks are:

### 1. **Inefficient Fill Method (CRITICAL BOTTLENECK)**
**Code Location**: `src/bitmap/bitmap.rs:283-308`

The `fill` method is extremely inefficient - it writes **one bit at a time** in a loop:
```rust
for _ in 0..count {
    // Append one bit at a time into 63-bit words
    let mut cw = self.read_current_word(written_words)?;
    cw |= bit << cw_offset;
    // ... more operations per bit ...
}
```

**Impact**:
- For backfilling 786,618 zeros, it performs 786,618 individual operations
- Each operation involves: read word, modify, write word, check boundaries
- This happened **24 times** in chunk 3 = ~18.9 million bit operations!

### 2. **Backfilling Operations Pattern**
- When new values appear late in the dataset (chunk 3), the system must:
  - Create new bitmaps
  - Backfill zeros for all previous chunks (786,618 rows in this case)
  - This happened 24 times in chunk 3 alone
- **Impact**: O(n*m*b) where n = rows already processed, m = new unique values, b = bit operations per row

### 2. **Memory Allocation for Backfilling**
- Each backfill operation allocates memory for the entire previous dataset
- Multiple large memory allocations happening sequentially

### 3. **File I/O Operations**
- Each bitmap operation requires file system operations
- Opening, writing, and closing files repeatedly

## Recommendations for Optimization

### CRITICAL - Immediate Fix Required
1. **Optimize the `fill` method** in `src/bitmap/bitmap.rs`:
   ```rust
   // Current inefficient approach:
   for _ in 0..count {
       // One bit at a time
   }

   // Optimized approach:
   pub fn fill(&mut self, value: bool, count: usize) -> Result<()> {
       if count == 0 { return Ok(()); }

       // Fill complete 63-bit words at once
       let full_words = count / 63;
       let remaining_bits = count % 63;

       // Create a full word of all 0s or all 1s
       let full_word = if value { 0x7FFFFFFFFFFFFFFF } else { 0 };

       // Write full words in bulk
       for _ in 0..full_words {
           self.append_full_word(full_word)?;
       }

       // Handle remaining bits
       if remaining_bits > 0 {
           let partial_word = if value {
               (1u64 << remaining_bits) - 1
           } else { 0 };
           self.append_partial_word(partial_word, remaining_bits)?;
       }

       Ok(())
   }
   ```
   **Expected improvement**: 63x faster for backfilling operations

### High Priority
1. **Implement Lazy Backfilling**: Instead of immediately backfilling when new values appear, keep a record and batch the backfill operations
2. **Value Discovery Phase**: Do a quick first pass to discover all unique values before creating bitmaps
3. **Batch File Operations**: Group multiple bitmap writes together

### Medium Priority
1. **Parallel Processing**: Process different columns in parallel threads
2. **Memory Pool**: Pre-allocate memory pools for bitmap operations
3. **Compression during Backfill**: Apply WAH compression while backfilling zeros

### Low Priority
1. **Adaptive Chunk Sizing**: Adjust chunk size based on value distribution
2. **Cache Bitmap Writers**: Keep writers open across chunks instead of closing/reopening

## How to View the Flame Graph

The flame graph (`flamegraph.svg`) has been generated and opened in your browser. In the flame graph:
- **Width** represents the time spent in each function
- **Height** represents the call stack depth
- **Click** on any box to zoom in on that function and its children
- Look for the widest boxes at the bottom levels - these are where most time is spent

## Optimization Results

### Performance Improvement Achieved
After implementing the optimized `fill` method that uses WAH fill words:

**Before optimization:**
- Total time: 11.2 seconds
- User CPU time: 2.09 seconds
- System time: 3.41 seconds

**After optimization:**
- Total time: 8.6 seconds (23% faster)
- User CPU time: 0.68 seconds (3x faster!)
- System time: 3.50 seconds (similar)

### What Changed
The optimized `fill` method now:
1. Completes partial literal words with multiple bits at once
2. Creates or extends WAH fill words with exact counts instead of looping
3. Handles bulk filling with WAH compression (single fill word represents thousands of 63-bit words)

### Key Insights
- **CPU time reduced by 3x**: The optimization dramatically reduced CPU-bound operations
- **System time unchanged**: I/O operations are now the primary bottleneck
- **WAH compression working**: Fill words now properly compress runs of zeros during backfilling

### Remaining Optimization Opportunities
1. **I/O optimization**: System time dominates - consider batching file operations
2. **Value discovery**: Pre-scan to discover all unique values before creating bitmaps
3. **Parallel processing**: Process columns in parallel threads
4. **Memory-mapped I/O improvements**: Reduce page syncing frequency
