# Tracking Transcript ENST00000491404.2

## Quick Start

To debug the discrepancy for transcript `ENST00000491404.2`, run oarfish with debug logging enabled:

```bash
RUST_LOG=debug oarfish \
  -a your_alignments.bam \
  -o output_prefix \
  --write-assignment-probs 2>&1 | tee debug_output.log
```

## What Gets Logged

### For Every Read Aligning to ENST00000491404.2

The instrumentation will log:

1. **Read identification**:
   - Read name
   - Total number of alignments for this read

2. **Unnormalized probabilities**:
   - Denominator (sum of count × prob × cov_prob for all alignments)
   - For each alignment:
     - Transcript name and ID
     - EM count estimate
     - Alignment probability
     - Coverage probability
     - Contribution to denominator

3. **Filtering stage**:
   - Pre-filter probability for each alignment
   - Whether it passes DISPLAY_THRESH (0.001)

4. **Final normalized probabilities**:
   - Post-normalization probability for each alignment
   - Whether it was written to .prob or filtered out
   - Total probability mass for the read (should be 1.0)

### At the End

Summary for the transcript:
- Transcript ID in reference
- **EM count estimate from .quant file**
- **Total number of reads aligning to this transcript**

## Analyzing the Output

### To Find the Discrepancy

1. **Grep for the specific transcript**:
   ```bash
   grep "ENST00000491404.2" debug_output.log > transcript_debug.log
   ```

2. **Extract the EM count** (from summary at end):
   ```bash
   grep "EM count estimate" transcript_debug.log
   # Example output: EM count estimate (.quant): 10.5000
   ```

3. **Calculate sum of posterior probabilities**:
   - Find all lines with "WRITTEN to .prob"
   - Sum the `post_norm` values for ENST00000491404.2 across all reads
   - Compare this sum to the EM count from step 2

4. **Investigate filtering**:
   - Look for reads where ENST00000491404.2 was "FILTERED OUT"
   - Check the pre_filter probability values
   - These reads contribute to .quant but not to .prob

### Example Analysis

If you see many reads like:
```
ENST00000491404.2 (id=123): pre_filter=0.000856, post_norm=0.000000 (FILTERED OUT, not in .prob)
```

This indicates reads where ENST00000491404.2 has a non-zero EM count contribution, but the posterior probability after normalization falls below DISPLAY_THRESH (0.001), so it's excluded from the .prob file.

### Understanding the Numbers

For each read aligning to ENST00000491404.2:
- **contribution** = count × prob × cov_prob (contributes to denominator)
- **pre_filter** = contribution / denominator (probability before filtering)
- **post_norm** = pre_filter / sum_of_passing_probs (final probability in .prob)

The EM algorithm uses all contributions, but .prob file only includes post_norm values > DISPLAY_THRESH.

## Changing the Target Transcript

To track a different transcript, edit `src/util/write_function.rs`:
```rust
const TARGET_TRANSCRIPT: &str = "YOUR_TRANSCRIPT_ID_HERE";
```

Then rebuild:
```bash
cargo build --release
```
