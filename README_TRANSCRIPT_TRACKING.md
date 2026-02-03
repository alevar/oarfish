# Transcript ENST00000491404.2 Debug Tracking - Quick Reference

## 🎯 Purpose
Track ALL reads aligning to transcript `ENST00000491404.2` to understand the discrepancy between:
- EM count in `.quant` file
- Sum of posterior probabilities in `.prob` file

## 🚀 Quick Start

### 1. Run oarfish with debug logging
```bash
RUST_LOG=debug oarfish \
  -a your_alignments.bam \
  -o output_prefix \
  --write-assignment-probs 2>&1 | tee debug_output.log
```

### 2. Extract transcript-specific information
```bash
grep "ENST00000491404.2" debug_output.log > transcript_analysis.log
```

### 3. Find the EM count estimate
```bash
grep "EM count estimate" transcript_analysis.log
# Output: EM count estimate (.quant): X.XXXX
```

### 4. Extract all posterior probabilities
```bash
grep "WRITTEN to .prob" transcript_analysis.log | grep "post_norm"
```

## 📊 What You'll See

### For Each Read Aligning to ENST00000491404.2

```
DEBUG Issue #59 [ENST00000491404.2]: Read 'read_name' aligns to target transcript
  Total alignments for this read: 3
  Denominator (unnormalized sum): 42.567890
  
  Aln 0: ENST00000491404.2 (id=123)
    count=10.5000, prob=0.950000, cov_prob=0.850000, contribution=8.456250
  Aln 1: OtherTranscript (id=45)
    count=5.2000, prob=0.900000, cov_prob=0.750000, contribution=3.510000
  ...
  
  FINAL probabilities written to .prob file for ENST00000491404.2:
    ENST00000491404.2 (id=123): pre_filter=0.198654, post_norm=0.301923 (WRITTEN to .prob)
    OtherTranscript (id=45): pre_filter=0.082456, post_norm=0.125321 (WRITTEN to .prob)
```

**Key Values:**
- `count`: EM estimate for that transcript
- `contribution`: count × prob × cov_prob (adds to denominator)
- `pre_filter`: contribution / denominator (before filtering)
- `post_norm`: final probability (written to .prob if > 0.001)

### At the End

```
DEBUG Issue #59 [ENST00000491404.2]: Summary
  Transcript ID in reference: 123
  EM count estimate (.quant): 10.5000
  Number of reads aligning to this transcript: 42
```

## 🔍 Finding the Discrepancy

### Step 1: Get EM count from summary
```bash
grep "EM count estimate" transcript_analysis.log
# Example: 10.5000
```

### Step 2: Sum all post_norm values
```bash
# Extract all post_norm values for ENST00000491404.2
grep "ENST00000491404.2.*WRITTEN to .prob" transcript_analysis.log | \
  grep -oP 'post_norm=\K[0-9.]+' | \
  awk '{sum += $1} END {print "Sum of posterior probs:", sum}'
```

### Step 3: Compare
- If sum ≈ EM count: Good! Small differences due to rounding
- If sum < EM count: Check for filtered reads (see below)
- If sum > EM count: Unexpected, investigate further

### Step 4: Find filtered reads
```bash
# Find reads where ENST00000491404.2 was filtered out
grep "ENST00000491404.2.*FILTERED OUT" transcript_analysis.log
```

These reads contribute to the EM count but their posterior probability was below DISPLAY_THRESH (0.001), so they're excluded from .prob file.

## 📈 Example Analysis

If you see:
```
ENST00000491404.2: pre_filter=0.000856, post_norm=0.000000 (FILTERED OUT, not in .prob)
```

This means:
1. Before filtering, probability was 0.000856
2. This is below DISPLAY_THRESH (0.001)
3. Excluded from .prob file
4. But the read still contributed to EM count via the EM algorithm

**This is the source of many discrepancies!**

## 🔧 Changing the Target Transcript

To track a different transcript:

1. Edit `src/util/write_function.rs` line 269:
   ```rust
   const TARGET_TRANSCRIPT: &str = "YOUR_TRANSCRIPT_ID";
   ```

2. Rebuild:
   ```bash
   cargo build --release
   ```

## 📚 More Information

- **TRACKING_TRANSCRIPT.md**: Detailed analysis guide
- **DEBUG_ISSUE_59.md**: Complete instrumentation documentation
- **test_debug_instrumentation.sh**: Test/demo script

## ✅ What's Instrumented

- ✅ Read name for every read aligning to target
- ✅ EM counts used in probability calculations
- ✅ Alignment probabilities (prob, cov_prob)
- ✅ Contribution to denominator calculation
- ✅ Pre-filter probabilities
- ✅ Filtering decisions (DISPLAY_THRESH = 0.001)
- ✅ Post-normalization probabilities
- ✅ Final values written to .prob vs filtered out
- ✅ Summary: EM count and total read count

## 🎓 Understanding the Math

For each read:
1. `contribution = count × prob × cov_prob`
2. `denominator = sum of all contributions`
3. `pre_filter = contribution / denominator`
4. If `pre_filter >= 0.001`:
   - `post_norm = pre_filter / sum_of_passing_probs`
   - Written to .prob file
5. If `pre_filter < 0.001`:
   - Filtered out, not in .prob
   - But still contributed to EM count!

**This explains discrepancies!**
