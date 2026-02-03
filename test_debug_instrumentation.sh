#!/bin/bash
# Simple test to verify debug instrumentation is working
# This script should be run from the oarfish repository root

set -e

echo "================================"
echo "Testing Debug Instrumentation"
echo "================================"
echo ""

# Check if oarfish binary exists
if [ ! -f "./target/release/oarfish" ] && [ ! -f "./target/debug/oarfish" ]; then
    echo "Building oarfish..."
    cargo build --release
fi

# Use release binary if available, otherwise debug
OARFISH_BIN="./target/release/oarfish"
if [ ! -f "$OARFISH_BIN" ]; then
    OARFISH_BIN="./target/debug/oarfish"
fi

echo "Using oarfish binary: $OARFISH_BIN"
echo ""

# Check if test data exists
if [ ! -f "./test_data/SIRV_isoforms_multi-fasta_170612a.fasta" ]; then
    echo "ERROR: Test data not found. Please ensure test_data directory exists."
    exit 1
fi

echo "Test data found: ./test_data/SIRV_isoforms_multi-fasta_170612a.fasta"
echo ""

# Show version
echo "Oarfish Version:"
$OARFISH_BIN --version
echo ""

echo "================================"
echo "Debug Instrumentation Test Info"
echo "================================"
echo ""
echo "To test the debug instrumentation:"
echo "1. Run oarfish with RUST_LOG=debug environment variable"
echo "2. Include the --write-assignment-probs flag"
echo "3. Look for 'DEBUG Issue #59' messages in the output"
echo ""
echo "Example command:"
echo "  RUST_LOG=debug $OARFISH_BIN \\"
echo "    -a your_alignments.bam \\"
echo "    -o output_prefix \\"
echo "    --write-assignment-probs"
echo ""
echo "Or with raw reads:"
echo "  RUST_LOG=debug $OARFISH_BIN \\"
echo "    --reads your_reads.fastq.gz \\"
echo "    --annotated ./test_data/SIRV_isoforms_multi-fasta_170612a.fasta \\"
echo "    --seq-tech ont-cdna \\"
echo "    -o output_prefix \\"
echo "    --write-assignment-probs"
echo ""
echo "================================"
echo "Debug Output Patterns to Look For"
echo "================================"
echo ""
echo "1. In EM algorithm (src/em.rs):"
echo "   'DEBUG Issue #59: Final EM iteration completed after N iterations'"
echo ""
echo "2. When writing probabilities (src/bulk.rs):"
echo "   'DEBUG Issue #59: Writing assignment probabilities'"
echo ""
echo "3. Per-read logging (src/util/write_function.rs):"
echo "   'DEBUG Issue #59: Read #N alignment data:'"
echo ""
echo "4. SPECIAL: Tracking transcript ENST00000491404.2:"
echo "   'DEBUG Issue #59 [ENST00000491404.2]: Read 'read_name' aligns to target transcript'"
echo "   For ALL reads aligning to this transcript, detailed info is logged"
echo ""
echo "See DEBUG_ISSUE_59.md for complete documentation."
echo "See TRACKING_TRANSCRIPT.md for analysis guide."
echo ""
echo "================================"
echo "Build Status: SUCCESS"
echo "Debug instrumentation is ready!"
echo "================================"
