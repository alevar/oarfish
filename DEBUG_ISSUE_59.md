# Debug Instrumentation for Issue #59

## Overview
This document describes the temporary debugging instrumentation added to trace discrepancies between `*.quant` and `*.prob` files as reported in Issue #59.

**Pre-PR Commit ID**: `af64207206c22fcf20b038492cf008edd1b17c34`

## Purpose
The debugging code helps identify:
1. Discrepancies arising from rounding errors
2. Effects of probability filtering via `DISPLAY_THRESH`
3. Mismatches between probability inputs (`alns`, `probs`, `coverage_probs`) used during EM computation and the `write_out_prob()` function

## Changes Made

### 1. `src/util/write_function.rs` - Enhanced `write_out_prob()` Function
Added comprehensive debug logging to track:
- **Alignment probability inputs** (first 5 reads):
  - Number of alignments per read
  - Reference IDs, base probabilities, and coverage probabilities
  
- **Filtering effects**:
  - Pre-filter probabilities before `DISPLAY_THRESH` is applied
  - Post-filter probabilities showing which transcripts passed/failed the threshold
  - List of filtered-out transcripts with their probability values
  
- **Normalization tracking**:
  - Denominator values before and after filtering
  - Post-normalization probabilities
  - Sum of normalized probabilities (should equal 1.0)

- **Summary statistics**:
  - Total reads processed
  - Total transcripts
  - Total equivalence classes
  - Model coverage status

### 2. `src/em.rs` - Enhanced EM Algorithm Logging
Added final iteration logging:
- Total count across all transcripts
- Number of transcripts with non-zero counts
- Top 10 transcripts by count for verification

### 3. `src/bulk.rs` - Count Summary Logging
Added logging when counts are passed to `write_out_prob()`:
- Total count summary
- Number of non-zero transcripts
- Helps verify consistency between EM output and prob file input

## How to Enable Debug Logging

To see the debug output, set the `RUST_LOG` environment variable before running oarfish:

```bash
# For basic debug output
RUST_LOG=debug oarfish [your options]

# For verbose debug output including all traces
RUST_LOG=trace oarfish [your options]

# For debugging only oarfish-specific code
RUST_LOG=oarfish=debug oarfish [your options]
```

### Example Usage

```bash
# With alignment file
RUST_LOG=debug oarfish \
  -a alignments.bam \
  -o output \
  --write-assignment-probs

# With raw reads
RUST_LOG=debug oarfish \
  --reads reads.fastq.gz \
  --annotated transcriptome.fa \
  --seq-tech ont-cdna \
  -o output \
  --write-assignment-probs
```

## Debug Output Format

The debug logs follow this pattern:

```
DEBUG Issue #59: Read #0 alignment data:
  Number of alignments: 3
  Alignment 0: ref_id=10, prob=0.95, cov_prob=0.8
  Alignment 1: ref_id=25, prob=0.90, cov_prob=0.7
  Alignment 2: ref_id=42, prob=0.85, cov_prob=0.6
  Denominator (before filtering): 15.234
  Pre-filter probabilities (DISPLAY_THRESH=0.001): 3 transcripts
    Transcript 10: prob=0.450000
    Transcript 25: prob=0.350000
    Transcript 42: prob=0.200000
  Post-filter: 3 transcripts passed threshold
  Denominator2 (sum of filtered probs): 1.0
  Post-normalization probabilities:
    Transcript 10: normalized_prob=0.450000
    Transcript 25: normalized_prob=0.350000
    Transcript 42: normalized_prob=0.200000
  Sum of normalized probabilities: 1.000000
```

## Identifying the Debug Code

All debug code is marked with comments containing:
- `DEBUG: Issue #59`
- Reference to pre-PR commit: `af64207206c22fcf20b038492cf008edd1b17c34`

This makes it easy to identify and remove the instrumentation later.

## Files Modified

1. `src/util/write_function.rs` - Primary instrumentation
2. `src/em.rs` - EM algorithm logging
3. `src/bulk.rs` - Count transfer logging

## Removal

To remove all debug instrumentation:

```bash
# Search for all debug code
git grep "DEBUG: Issue #59"

# Or use this command to see all affected lines
git diff af64207206c22fcf20b038492cf008edd1b17c34 --stat
```

All changes are clearly marked and can be reverted by checking out the pre-PR commit or manually removing the marked sections.

## Notes

- Debug logging is only enabled when `RUST_LOG` environment variable is set appropriately
- The instrumentation focuses on the first 5 reads to avoid excessive log output
- Summary statistics are logged for all reads
- No functional changes were made to the quantification algorithm
- The code compiles successfully in both debug and release modes
