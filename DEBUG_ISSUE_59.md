# Debug Instrumentation for Issue #59

## Overview
This document describes the temporary debugging instrumentation added to trace discrepancies between `*.quant` and `*.prob` files as reported in Issue #59.

**Pre-PR Commit ID**: `af64207206c22fcf20b038492cf008edd1b17c34`

## Purpose
The debugging code helps identify:
1. Discrepancies arising from rounding errors
2. Effects of probability filtering via `DISPLAY_THRESH`
3. Mismatches between probability inputs (`alns`, `probs`, `coverage_probs`) used during EM computation and the `write_out_prob()` function
4. **NEW**: Detailed tracking of specific transcript `ENST00000491404.2` to debug large discrepancies

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

- **NEW - Specific Transcript Tracking** (`ENST00000491404.2`):
  - **For ALL reads aligning to this transcript:**
    - Read name and total alignments
    - Denominator (unnormalized sum)
    - For each alignment: transcript name, count, prob, cov_prob, contribution
    - Pre-filter probabilities for all alignments
    - Which alignments pass/fail the DISPLAY_THRESH filter
    - Post-normalization probabilities
    - Final probabilities written to .prob file
  - **Summary at end:**
    - Transcript ID in reference
    - EM count estimate from .quant file
    - Total number of reads aligning to this transcript

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

### General Read Logging (first 5 reads)
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

### Specific Transcript Logging (ENST00000491404.2 - ALL reads)
```
DEBUG Issue #59 [ENST00000491404.2]: Read 'read_12345' aligns to target transcript
  Total alignments for this read: 4
  Denominator (unnormalized sum): 42.567890
  Aln 0: ENST00000491404.2 (id=123)
    count=10.5000, prob=0.950000, cov_prob=0.850000, contribution=8.456250
  Aln 1: ENST00000123456.1 (id=45)
    count=5.2000, prob=0.900000, cov_prob=0.750000, contribution=3.510000
  Aln 2: ENST00000987654.3 (id=67)
    count=8.1000, prob=0.880000, cov_prob=0.820000, contribution=5.848960
  Aln 3: ENST00000555555.2 (id=89)
    count=12.3000, prob=0.920000, cov_prob=0.900000, contribution=10.195200
  Filtering stage for target transcript ENST00000491404.2:
    ENST00000491404.2 (id=123): pre_filter_prob=0.198654, passed_threshold=true
    ENST00000123456.1 (id=45): pre_filter_prob=0.082456, passed_threshold=true
    ENST00000987654.3 (id=67): pre_filter_prob=0.137456, passed_threshold=true
    ENST00000555555.2 (id=89): pre_filter_prob=0.239456, passed_threshold=true
  Sum before renormalization (denom2): 0.658022
  FINAL probabilities written to .prob file for ENST00000491404.2:
    ENST00000491404.2 (id=123): pre_filter=0.198654, post_norm=0.301923 (WRITTEN to .prob)
    ENST00000123456.1 (id=45): pre_filter=0.082456, post_norm=0.125321 (WRITTEN to .prob)
    ENST00000987654.3 (id=67): pre_filter=0.137456, post_norm=0.208876 (WRITTEN to .prob)
    ENST00000555555.2 (id=89): pre_filter=0.239456, post_norm=0.363880 (WRITTEN to .prob)
  Total probability mass written for this read: 1.000000

... (repeated for every read aligning to ENST00000491404.2) ...

DEBUG Issue #59 [ENST00000491404.2]: Summary
  Transcript ID in reference: 123
  EM count estimate (.quant): 10.5000
  Number of reads aligning to this transcript: 42
  Note: Check sum of posterior probabilities in .prob file for this transcript
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
