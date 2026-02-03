use crate::prog_opts::ReadAssignmentProbOut;
use crate::util::oarfish_types::EMInfo;
use crate::util::parquet_utils;
use itertools::izip;

use arrow2::{
    array::Array,
    chunk::Chunk,
    datatypes::{Field, Schema},
};
use either::Either;
use lz4::EncoderBuilder;
use path_tools::WithAdditionalExtension;
use swapvec::SwapVec;

use std::path::{Path, PathBuf};
use std::{
    fs,
    fs::File,
    fs::OpenOptions,
    fs::create_dir_all,
    io::{self, BufWriter, Write},
};
// DEBUG: Issue #59 - Added tracing for debugging (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
use tracing::debug;

pub fn write_single_cell_output(
    output: &PathBuf,
    info: serde_json::Value,
    header: &noodles_sam::header::Header,
    counts: &sprs::TriMatI<f32, u32>,
) -> io::Result<()> {
    // if there is a parent directory
    if let Some(p) = output.parent() {
        // unless this was a relative path with one component,
        // which we should treat as the file prefix, then grab
        // the non-empty parent and create it.
        if p != Path::new("") {
            create_dir_all(p)?;
        }
    }

    {
        let info_path = output.with_additional_extension(".meta_info.json");
        let write = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(info_path)
            .expect("Couldn't create output file");

        serde_json::ser::to_writer_pretty(write, &info)?;
    }

    let out_path = output.with_additional_extension(".count.mtx");
    sprs::io::write_matrix_market(out_path, counts)?;

    let out_path = output.with_additional_extension(".features.txt");
    File::create(&out_path)?;
    let write = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)
        .expect("Couldn't create output file");
    let mut writer = BufWriter::new(write);

    for (rseq, _rmap) in header.reference_sequences().iter() {
        writeln!(writer, "{}", rseq).expect("Couldn't write to output file.");
    }
    Ok(())
}

//this part is taken from dev branch
pub fn write_output(
    output: &PathBuf,
    info: serde_json::Value,
    header: &noodles_sam::header::Header,
    counts: &[f64],
    aux_counts: &[crate::util::aux_counts::CountInfo],
) -> io::Result<()> {
    // if there is a parent directory
    if let Some(p) = output.parent() {
        // unless this was a relative path with one component,
        // which we should treat as the file prefix, then grab
        // the non-empty parent and create it.
        if p != Path::new("") {
            create_dir_all(p)?;
        }
    }

    {
        let info_path = output.with_additional_extension(".meta_info.json");
        let write = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(info_path)
            .expect("Couldn't create output file");

        serde_json::ser::to_writer_pretty(write, &info)?;
    }

    let out_path = output.with_additional_extension(".quant");
    File::create(&out_path)?;

    let write = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)
        .expect("Couldn't create output file");
    let mut writer = BufWriter::new(write);

    writeln!(writer, "tname\tlen\tnum_reads").expect("Couldn't write to output file.");
    // loop over the transcripts in the header and fill in the relevant
    // information here.

    for (i, (rseq, rmap)) in header.reference_sequences().iter().enumerate() {
        writeln!(writer, "{}\t{}\t{}", rseq, rmap.length(), counts[i])
            .expect("Couldn't write to output file.");
    }

    // write the auxiliary count info
    let out_path = output.with_additional_extension(".ambig_info.tsv");
    File::create(&out_path)?;

    let write = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)
        .expect("Couldn't create output file");
    let mut writer = BufWriter::new(write);

    writeln!(writer, "unique_reads\tambig_reads\ttotal_reads")
        .expect("Couldn't write to output file.");
    // loop over the transcripts in the header and fill in the relevant
    // information here.

    for (i, (_rseq, _rmap)) in header.reference_sequences().iter().enumerate() {
        let total = aux_counts[i].total_count;
        let unique = aux_counts[i].unique_count;
        let ambig = total.saturating_sub(unique);
        writeln!(writer, "{}\t{}\t{}", unique, ambig, total)
            .expect("Couldn't write to output file.");
    }

    Ok(())
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn write_out_cdf(
    output: &String,
    prob: &str,
    rate: &str,
    bins: &u32,
    alpha: f64,
    beta: f64,
    emi: &EMInfo,
    txps_name: &[String],
) -> io::Result<()> {
    let output_directory = format!("{}/{}/CDFOutput", output, bins);
    fs::create_dir_all(output_directory.clone())?;

    let out_path: String = if prob == "entropy" {
        format!(
            "{}/{}_{}_{}_{}_cdf.tsv",
            output_directory, prob, rate, alpha, beta
        )
    } else {
        format!("{}/{}_{}_cdf.tsv", output_directory, prob, rate)
    };

    File::create(out_path.clone())?;

    let write_cdf = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)
        .expect("Couldn't create output file");
    let mut writer_cdf = BufWriter::new(write_cdf);

    writeln!(writer_cdf, "Txps_Name\tCDF_Values").expect("Couldn't write to output file.");
    for (i, txp) in txps_name.iter().enumerate() {
        let cdf_values: String = emi.txp_info[i]
            .coverage_prob
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
            .join("\t");

        writeln!(writer_cdf, "{}\t{}", *txp, cdf_values,).expect("Couldn't write to output file.");
    }

    Ok(())
}

pub(crate) fn write_infrep_file(
    output_path: &Path,
    fields: Vec<Field>,
    chunk: Chunk<Box<dyn Array>>,
) -> anyhow::Result<()> {
    let output_path = output_path
        .to_path_buf()
        .with_additional_extension(".infreps.pq");
    let schema = Schema::from(fields);
    parquet_utils::write_chunk_to_file(output_path.to_str().unwrap(), schema, chunk)
}

pub fn write_out_prob(
    output: &PathBuf,
    emi: &EMInfo,
    counts: &[f64],
    names_vec: SwapVec<String>,
    txps_name: &[String],
) -> anyhow::Result<()> {
    if let Some(p) = output.parent() {
        // unless this was a relative path with one component,
        // which we should treat as the file prefix, then grab
        // the non-empty parent and create it.
        if p != Path::new("") {
            create_dir_all(p)?;
        }
    }

    let compressed = matches!(
        emi.eq_map.filter_opts.write_assignment_probs_type,
        Some(ReadAssignmentProbOut::Compressed)
    );

    let extension = if compressed { ".prob.lz4" } else { ".prob" };
    let out_path = output.with_additional_extension(extension);
    File::create(&out_path)?;

    let write_prob = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)
        .expect("Couldn't create output file");

    let mut writer_prob = if compressed {
        Either::Right(EncoderBuilder::new().level(4).build(write_prob)?)
    } else {
        Either::Left(BufWriter::with_capacity(1024 * 1024, write_prob))
    };

    writeln!(writer_prob, "{}\t{}", txps_name.len(), emi.eq_map.len())
        .expect("couldn't write to prob output file");
    for tname in txps_name {
        writeln!(writer_prob, "{}", tname).expect("couldn't write to prob output file");
    }

    let model_coverage = emi.eq_map.filter_opts.model_coverage;
    //let names_vec = emi.eq_map.take_read_names_vec()?;

    let names_iter = names_vec.into_iter();

    let mut txps = Vec::<usize>::new();
    let mut txp_probs = Vec::<f64>::new();

    // DEBUG: Issue #59 - Track statistics for debugging (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
    let mut read_count = 0_usize;
    
    // DEBUG: Issue #59 - Track specific transcript for detailed analysis (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
    const TARGET_TRANSCRIPT: &str = "ENST00000491404.2";
    let target_transcript_id = txps_name.iter().position(|name| name == TARGET_TRANSCRIPT);
    let mut target_transcript_read_count = 0_usize;

    for ((alns, probs, coverage_probs), name) in izip!(emi.eq_map.iter(), names_iter) {
        // DEBUG: Issue #59 - Log alignment inputs for first few reads (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        let read_idx = read_count;
        
        // DEBUG: Issue #59 - Check if this read aligns to the target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        let aligns_to_target = if let Some(target_id) = target_transcript_id {
            alns.iter().any(|a| a.ref_id as usize == target_id)
        } else {
            false
        };
        
        if aligns_to_target {
            target_transcript_read_count += 1;
        }
        
        if read_idx < 5 {
            debug!("DEBUG Issue #59: Read #{} alignment data:", read_idx);
            debug!("  Number of alignments: {}", alns.len());
            for (idx, (a, p, cp)) in izip!(alns, probs, coverage_probs).enumerate() {
                debug!("  Alignment {}: ref_id={}, prob={}, cov_prob={}",
                    idx, a.ref_id, *p, *cp);
            }
        }
        read_count += 1;
        let mut denom = 0.0_f64;

        for (a, p, cp) in izip!(alns, probs, coverage_probs) {
            let target_id = a.ref_id as usize;
            let prob = *p as f64;
            let cov_prob = if model_coverage { *cp } else { 1.0 };
            denom += counts[target_id] * prob * cov_prob;
        }

        // DEBUG: Issue #59 - Log denominator calculation for first few reads (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if read_idx < 5 {
            debug!("  Denominator (before filtering): {}", denom);
        }
        
        // DEBUG: Issue #59 - Log detailed info for reads aligning to target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if aligns_to_target {
            let rn_temp = name.as_ref().expect("could not extract read name from file");
            let read_name = rn_temp.trim_end_matches('\0');
            debug!("DEBUG Issue #59 [{}]: Read '{}' aligns to target transcript", TARGET_TRANSCRIPT, read_name);
            debug!("  Total alignments for this read: {}", alns.len());
            debug!("  Denominator (unnormalized sum): {:.6}", denom);
            for (idx, (a, p, cp)) in izip!(alns, probs, coverage_probs).enumerate() {
                let tid = a.ref_id as usize;
                let transcript_name = &txps_name[tid];
                let count = counts[tid];
                let prob_val = *p as f64;
                let cov_prob_val = if model_coverage { *cp } else { 1.0 };
                let contribution = count * prob_val * cov_prob_val;
                debug!("    Aln {}: {} (id={})", idx, transcript_name, tid);
                debug!("      count={:.4}, prob={:.6}, cov_prob={:.6}, contribution={:.6}",
                    count, prob_val, cov_prob_val, contribution);
            }
        }

        let rn = name.expect("could not extract read name from file");
        let read = rn.trim_end_matches('\0');

        write!(writer_prob, "{}\t", read).expect("couldn't write to prob output file");

        txps.clear();
        txp_probs.clear();

        const DISPLAY_THRESH: f64 = 0.001;
        let mut denom2 = 0.0_f64;

        // DEBUG: Issue #59 - Track pre-filter probabilities for first few reads (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        let mut pre_filter_probs = if read_idx < 5 {
            Some(Vec::<(usize, f64)>::new())
        } else {
            None
        };
        
        // DEBUG: Issue #59 - Track probabilities for target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        let mut target_transcript_probs = if aligns_to_target {
            Some(Vec::<(usize, String, f64, f64)>::new()) // (tid, name, pre_filter_prob, post_filter_prob)
        } else {
            None
        };

        for (a, p, cp) in izip!(alns, probs, coverage_probs) {
            let target_id = a.ref_id as usize;
            let prob = *p as f64;
            let cov_prob = if model_coverage { *cp } else { 1.0 };
            let nprob = ((counts[target_id] * prob * cov_prob) / denom).clamp(0.0, 1.0);

            // DEBUG: Issue #59 - Store pre-filter probabilities (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
            if let Some(ref mut pre_filter) = pre_filter_probs {
                pre_filter.push((target_id, nprob));
            }
            
            // DEBUG: Issue #59 - Store probabilities for target transcript tracking (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
            if let Some(ref mut target_probs) = target_transcript_probs {
                target_probs.push((target_id, txps_name[target_id].clone(), nprob, 0.0));
            }

            if nprob >= DISPLAY_THRESH {
                txps.push(target_id);
                txp_probs.push(nprob);
                denom2 += nprob;
            }
        }

        // DEBUG: Issue #59 - Log filtering effects for first few reads (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if let Some(pre_filter) = pre_filter_probs {
            debug!("  Pre-filter probabilities (DISPLAY_THRESH={}): {} transcripts",
                DISPLAY_THRESH, pre_filter.len());
            for (tid, prob) in &pre_filter {
                debug!("    Transcript {}: prob={:.6}", tid, prob);
            }
            debug!("  Post-filter: {} transcripts passed threshold", txps.len());
            debug!("  Denominator2 (sum of filtered probs): {}", denom2);

            // Log which transcripts were filtered out
            let filtered_out: Vec<_> = pre_filter.iter()
                .filter(|(_, p)| *p < DISPLAY_THRESH)
                .collect();
            if !filtered_out.is_empty() {
                debug!("  Filtered out {} transcripts:", filtered_out.len());
                for (tid, prob) in filtered_out {
                    debug!("    Transcript {}: prob={:.6} (below threshold)", tid, prob);
                }
            }
        }
        
        // DEBUG: Issue #59 - Log filtering for target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if let Some(ref target_probs) = target_transcript_probs {
            debug!("  Filtering stage for target transcript {}:", TARGET_TRANSCRIPT);
            for (tid, tname, pre_prob, _) in target_probs {
                let passed = pre_prob >= &DISPLAY_THRESH;
                debug!("    {} (id={}): pre_filter_prob={:.6}, passed_threshold={}",
                    tname, tid, pre_prob, passed);
            }
            debug!("  Sum before renormalization (denom2): {:.6}", denom2);
        }

        for p in txp_probs.iter_mut() {
            *p /= denom2;
        }
        
        // DEBUG: Issue #59 - Update post-normalization probs for target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if let Some(ref mut target_probs) = target_transcript_probs {
            for (tid, _, _, post_prob) in target_probs.iter_mut() {
                // Find this transcript in the final output
                if let Some(pos) = txps.iter().position(|&t| t == *tid) {
                    *post_prob = txp_probs[pos];
                }
            }
        }

        // DEBUG: Issue #59 - Log normalized probabilities for first few reads (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if read_idx < 5 {
            debug!("  Post-normalization probabilities:");
            for (tid, prob) in izip!(&txps, &txp_probs) {
                debug!("    Transcript {}: normalized_prob={:.6}", tid, prob);
            }
            let prob_sum: f64 = txp_probs.iter().sum();
            debug!("  Sum of normalized probabilities: {:.6}", prob_sum);
        }
        
        // DEBUG: Issue #59 - Final summary for target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
        if let Some(target_probs) = target_transcript_probs {
            debug!("  FINAL probabilities written to .prob file for {}:", TARGET_TRANSCRIPT);
            for (tid, tname, pre_prob, post_prob) in &target_probs {
                if *post_prob > 0.0 {
                    debug!("    {} (id={}): pre_filter={:.6}, post_norm={:.6} (WRITTEN to .prob)",
                        tname, tid, pre_prob, post_prob);
                } else {
                    debug!("    {} (id={}): pre_filter={:.6}, post_norm={:.6} (FILTERED OUT, not in .prob)",
                        tname, tid, pre_prob, post_prob);
                }
            }
            let prob_sum: f64 = txp_probs.iter().sum();
            debug!("  Total probability mass written for this read: {:.6}", prob_sum);
        }

        let txp_ids = txps
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join("\t");
        let prob_vals = txp_probs
            .iter()
            .map(|x| format!("{:.3}", x))
            .collect::<Vec<String>>()
            .join("\t");
        writeln!(writer_prob, "{}\t{}\t{}", txps.len(), txp_ids, prob_vals)
            .expect("couldn't write to prob output file");
    }

    // DEBUG: Issue #59 - Log summary statistics (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
    debug!("DEBUG Issue #59: Completed writing probabilities for {} reads", read_count);
    debug!("  Total transcripts: {}", txps_name.len());
    debug!("  Total equivalence classes: {}", emi.eq_map.len());
    debug!("  Model coverage enabled: {}", model_coverage);
    
    // DEBUG: Issue #59 - Log summary for target transcript (pre-PR commit: af64207206c22fcf20b038492cf008edd1b17c34)
    if let Some(target_id) = target_transcript_id {
        debug!("DEBUG Issue #59 [{}]: Summary", TARGET_TRANSCRIPT);
        debug!("  Transcript ID in reference: {}", target_id);
        debug!("  EM count estimate (.quant): {:.4}", counts[target_id]);
        debug!("  Number of reads aligning to this transcript: {}", target_transcript_read_count);
        debug!("  Note: Check sum of posterior probabilities in .prob file for this transcript");
    } else {
        debug!("DEBUG Issue #59: Target transcript {} not found in reference", TARGET_TRANSCRIPT);
    }

    if let Either::Right(lz4) = writer_prob {
        let (_output, result) = lz4.finish();
        result?;
    }

    Ok(())
}
