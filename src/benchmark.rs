use crate::aos;
use crate::aos_parallel;
use crate::types::{CodeInitParams, Share, DealMetrics, ReconstructMetrics, PhaseMetrics};
use ark_ff::{BigInt, PrimeField};
use chrono::Local;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt::Debug;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use humantime::format_duration;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Result of a single benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub setup_time: Duration,
    pub deal_time: Duration,
    pub reconstruct_time: Duration,
    pub total_time: Duration,
    pub params: BenchmarkParams,
    pub success: bool,
    pub deal_metrics: Option<DealMetrics>,
    pub reconstruct_metrics: Option<ReconstructMetrics>,
}

/// Configuration for a benchmark run
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub c_value: usize,
    pub secret_value: u128,
    pub shares_to_remove: usize,
    pub decoder_type: DecoderImplementation,
    pub ldpc_rate: AR4JARate,
    pub ldpc_info_size: AR4JAInfoSize,
    pub max_iterations: usize,
    pub llr_bits: u64,
    pub implementation: Implementation,
}

/// Implementation of Hash for BenchmarkParams to allow it to be used in a HashMap key
impl Hash for BenchmarkParams {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.c_value.hash(state);
        self.secret_value.hash(state);
        self.shares_to_remove.hash(state);
        
        // Hash the discriminant for enum types
        std::mem::discriminant(&self.decoder_type).hash(state);
        std::mem::discriminant(&self.ldpc_rate).hash(state);
        std::mem::discriminant(&self.ldpc_info_size).hash(state);
        
        self.max_iterations.hash(state);
        self.llr_bits.hash(state);
        self.implementation.hash(state);
    }
}

/// Available implementations to benchmark
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Implementation {
    Sequential,
    Parallel,
}

impl std::fmt::Display for Implementation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Implementation::Sequential => write!(f, "Sequential"),
            Implementation::Parallel => write!(f, "Parallel"),
        }
    }
}

/// Summary statistics for multiple benchmark runs
#[derive(Debug, Clone)]
pub struct BenchmarkStats {
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub median: Duration,
    pub std_dev: Duration,
    pub success_rate: f64,
    pub runs: usize,
    pub phase_metrics: Option<HashMap<String, PhaseStats>>,
}

/// Statistics for a specific phase
#[derive(Debug, Clone)]
pub struct PhaseStats {
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_percentage: f64,
}

impl BenchmarkStats {
    pub fn new(times: &[Duration], successes: usize, runs: usize) -> Self {
        if times.is_empty() {
            return BenchmarkStats {
                min: Duration::new(0, 0),
                max: Duration::new(0, 0),
                avg: Duration::new(0, 0),
                median: Duration::new(0, 0),
                std_dev: Duration::new(0, 0),
                success_rate: 0.0,
                runs: 0,
                phase_metrics: None,
            };
        }

        let mut sorted_times = times.to_vec();
        sorted_times.sort();

        let min = *sorted_times.first().unwrap();
        let max = *sorted_times.last().unwrap();
        
        let total_nanos: u128 = times.iter().map(|d| d.as_nanos()).sum();
        let avg = Duration::from_nanos((total_nanos / times.len() as u128) as u64);
        
        let median = if times.len() % 2 == 0 {
            let mid_idx = times.len() / 2;
            let mid_nanos = (sorted_times[mid_idx - 1].as_nanos() + sorted_times[mid_idx].as_nanos()) / 2;
            Duration::from_nanos(mid_nanos as u64)
        } else {
            sorted_times[times.len() / 2]
        };
        
        let variance: u128 = times
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as i128 - avg.as_nanos() as i128;
                (diff * diff) as u128
            })
            .sum::<u128>() / times.len() as u128;
        
        let std_dev = Duration::from_nanos((variance as f64).sqrt() as u64);
        
        BenchmarkStats {
            min,
            max,
            avg,
            median,
            std_dev,
            success_rate: successes as f64 / runs as f64,
            runs,
            phase_metrics: None,
        }
    }
    
    pub fn with_phase_metrics(mut self, deal_metrics: &[Option<DealMetrics>], reconstruct_metrics: &[Option<ReconstructMetrics>]) -> Self {
        let mut phase_stats = HashMap::new();
        
        if !deal_metrics.is_empty() {
            // Process deal metrics
            let metrics: Vec<&DealMetrics> = deal_metrics.iter().filter_map(|m| m.as_ref()).collect();
            if !metrics.is_empty() {
                // Random vector generation
                phase_stats.insert(String::from("Random vector generation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.rand_vec_generation.duration).collect(),
                    metrics.iter().map(|m| m.rand_vec_generation.percentage).collect(),
                ));
                
                // Dot product calculation
                phase_stats.insert(String::from("Dot product calculation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.dot_product.duration).collect(),
                    metrics.iter().map(|m| m.dot_product.percentage).collect(),
                ));
                
                // Message matrix creation
                phase_stats.insert(String::from("Message matrix creation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.matrix_creation.duration).collect(),
                    metrics.iter().map(|m| m.matrix_creation.percentage).collect(),
                ));
                
                // Encoding phase
                phase_stats.insert(String::from("Encoding phase"), calculate_phase_stats(
                    metrics.iter().map(|m| m.encoding.duration).collect(),
                    metrics.iter().map(|m| m.encoding.percentage).collect(),
                ));
                
                // Share creation
                phase_stats.insert(String::from("Share creation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.share_creation.duration).collect(),
                    metrics.iter().map(|m| m.share_creation.percentage).collect(),
                ));
            }
        }
        
        if !reconstruct_metrics.is_empty() {
            // Process reconstruct metrics
            let metrics: Vec<&ReconstructMetrics> = reconstruct_metrics.iter().filter_map(|m| m.as_ref()).collect();
            if !metrics.is_empty() {
                // Matrix setup
                phase_stats.insert(String::from("Matrix setup"), calculate_phase_stats(
                    metrics.iter().map(|m| m.matrix_setup.duration).collect(),
                    metrics.iter().map(|m| m.matrix_setup.percentage).collect(),
                ));
                
                // Row decoding
                phase_stats.insert(String::from("Row decoding"), calculate_phase_stats(
                    metrics.iter().map(|m| m.row_decoding.duration).collect(),
                    metrics.iter().map(|m| m.row_decoding.percentage).collect(),
                ));
                
                // Field element reconstruction
                phase_stats.insert(String::from("Field element reconstruction"), calculate_phase_stats(
                    metrics.iter().map(|m| m.field_reconstruction.duration).collect(),
                    metrics.iter().map(|m| m.field_reconstruction.percentage).collect(),
                ));
                
                // Final computation
                phase_stats.insert(String::from("Final computation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.final_computation.duration).collect(),
                    metrics.iter().map(|m| m.final_computation.percentage).collect(),
                ));
            }
        }
        
        if !phase_stats.is_empty() {
            self.phase_metrics = Some(phase_stats);
        }
        
        self
    }
}

/// Calculate statistics for a phase
fn calculate_phase_stats(durations: Vec<Duration>, percentages: Vec<f64>) -> PhaseStats {
    if durations.is_empty() {
        return PhaseStats {
            avg_duration: Duration::new(0, 0),
            min_duration: Duration::new(0, 0),
            max_duration: Duration::new(0, 0),
            avg_percentage: 0.0,
        };
    }

    let mut sorted_durations = durations.clone();
    sorted_durations.sort();

    let min_duration = *sorted_durations.first().unwrap();
    let max_duration = *sorted_durations.last().unwrap();
    
    let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
    let avg_duration = Duration::from_nanos((total_nanos / durations.len() as u128) as u64);
    
    let avg_percentage = percentages.iter().sum::<f64>() / percentages.len() as f64;
    
    PhaseStats {
        avg_duration,
        min_duration,
        max_duration,
        avg_percentage,
    }
}

/// Aggregated benchmark results for different parameter combinations
pub struct BenchmarkSummary {
    pub setup_stats: HashMap<BenchmarkParams, BenchmarkStats>,
    pub deal_stats: HashMap<BenchmarkParams, BenchmarkStats>,
    pub reconstruct_stats: HashMap<BenchmarkParams, BenchmarkStats>,
    pub total_stats: HashMap<BenchmarkParams, BenchmarkStats>,
}

/// Removes random shares from the vector
fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: usize) {
    let mut rng = thread_rng();
    shares.shuffle(&mut rng);
    if num_to_remove <= shares.len() {
        shares.drain(0..num_to_remove);
    }
}

/// Run a single benchmark with the given parameters
pub fn run_single_benchmark<F: PrimeField<BigInt = BigInt<4>> + Debug>(
    params: &BenchmarkParams, 
    progress: Option<&ProgressBar>
) -> BenchmarkResult {
    let secret = F::from(params.secret_value);
    
    let code_params = CodeInitParams {
        decoder_type: Some(params.decoder_type),
        ldpc_rate: Some(params.ldpc_rate),
        ldpc_info_size: Some(params.ldpc_info_size),
        max_iterations: Some(params.max_iterations),
        llr_value: Some(f64::from_bits(params.llr_bits)),
    };

    if let Some(pb) = progress {
        pb.set_message("Setting up...");
    }

    let (setup_duration, deal_duration, reconstruct_duration, reconstructed_secret, deal_metrics, reconstruct_metrics) = 
        match params.implementation {
            Implementation::Sequential => {
                // Setup phase
                let setup_start = Instant::now();
                let mut pp = aos::setup::<F>(code_params, params.c_value as u32);
                let setup_duration = setup_start.elapsed();

                if let Some(pb) = progress {
                    pb.set_message("Dealing shares...");
                }
                
                // Deal phase
                let deal_start = Instant::now();
                let mut shares = aos::deal(&pp, secret);
                let deal_duration = deal_start.elapsed();
                let deal_metrics = shares.metrics.clone();
                
                if let Some(pb) = progress {
                    pb.set_message("Removing shares...");
                }
                
                // Remove shares
                remove_random_shares(&mut shares.shares, params.shares_to_remove);
                
                if let Some(pb) = progress {
                    pb.set_message("Reconstructing...");
                }
                
                // Reconstruct phase
                let reconstruct_start = Instant::now();
                let (reconstructed_secret, reconstruct_metrics) = aos::reconstruct(&mut pp, &shares);
                let reconstruct_duration = reconstruct_start.elapsed();
                
                (setup_duration, deal_duration, reconstruct_duration, reconstructed_secret, deal_metrics, reconstruct_metrics)
            },
            Implementation::Parallel => {
                // Setup phase
                let setup_start = Instant::now();
                let pp = aos_parallel::setup::<F>(code_params, params.c_value as u32);
                let setup_duration = setup_start.elapsed();

                if let Some(pb) = progress {
                    pb.set_message("Dealing shares...");
                }
                
                // Deal phase
                let deal_start = Instant::now();
                let mut shares = aos_parallel::deal(&pp, secret);
                let deal_duration = deal_start.elapsed();
                let deal_metrics = shares.metrics.clone();
                
                if let Some(pb) = progress {
                    pb.set_message("Removing shares...");
                }
                
                // Remove shares
                remove_random_shares(&mut shares.shares, params.shares_to_remove);
                
                if let Some(pb) = progress {
                    pb.set_message("Reconstructing...");
                }
                
                // Reconstruct phase
                let reconstruct_start = Instant::now();
                let (reconstructed_secret, reconstruct_metrics) = aos_parallel::reconstruct(&pp, &shares);
                let reconstruct_duration = reconstruct_start.elapsed();
                
                (setup_duration, deal_duration, reconstruct_duration, reconstructed_secret, deal_metrics, reconstruct_metrics)
            }
        };

    if let Some(pb) = progress {
        pb.set_message("Done!");
    }

    let total_time = setup_duration + deal_duration + reconstruct_duration;
    let success = secret == reconstructed_secret;

    BenchmarkResult {
        setup_time: setup_duration,
        deal_time: deal_duration,
        reconstruct_time: reconstruct_duration,
        total_time,
        params: params.clone(),
        success,
        deal_metrics,
        reconstruct_metrics,
    }
}

/// Run multiple benchmarks with the same parameters to gather statistics
pub fn run_multiple_benchmarks<F: PrimeField<BigInt = BigInt<4>> + Debug>(
    params: &BenchmarkParams,
    num_runs: usize,
    multi_progress: &MultiProgress,
) -> Vec<BenchmarkResult> {
    let pb = multi_progress.add(ProgressBar::new(num_runs as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len} runs ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );
    
    pb.set_message(format!(
        "Benchmarking {} (c={}, rate={:?}, info_size={:?}, decoder={:?})", 
        params.implementation,
        params.c_value,
        params.ldpc_rate,
        params.ldpc_info_size,
        params.decoder_type
    ));

    let mut results = Vec::with_capacity(num_runs);
    
    for i in 0..num_runs {
        pb.set_message(format!(
            "Run {}/{} - {} (c={}, rate={:?}, info_size={:?}, decoder={:?})", 
            i + 1, 
            num_runs,
            params.implementation,
            params.c_value,
            params.ldpc_rate,
            params.ldpc_info_size,
            params.decoder_type
        ));
        
        let run_progress = multi_progress.add(ProgressBar::new(4));
        run_progress.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.bold.dim} {msg}")
                .unwrap(),
        );
        run_progress.set_prefix(format!("[Run {}/{}]", i + 1, num_runs));
        
        let result = run_single_benchmark::<F>(params, Some(&run_progress));
        results.push(result);
        
        run_progress.finish_and_clear();
        pb.inc(1);
    }
    
    pb.finish_with_message(format!(
        "Completed {} runs for {} (c={}, rate={:?}, info_size={:?}, decoder={:?})",
        num_runs,
        params.implementation,
        params.c_value,
        params.ldpc_rate,
        params.ldpc_info_size,
        params.decoder_type
    ));
    
    results
}

/// Generate all parameter combinations to benchmark
pub fn generate_benchmark_params(
    c_values: &[usize],
    shares_to_remove_values: &[usize],
    decoder_types: &[DecoderImplementation],
    ldpc_rates: &[AR4JARate],
    ldpc_info_sizes: &[AR4JAInfoSize],
    implementations: &[Implementation],
) -> Vec<BenchmarkParams> {
    let mut params = Vec::new();
    
    for &c in c_values {
        for &shares_to_remove in shares_to_remove_values {
            for &decoder_type in decoder_types {
                for &rate in ldpc_rates {
                    for &info_size in ldpc_info_sizes {
                        for &implementation in implementations {
                            params.push(BenchmarkParams {
                                c_value: c,
                                secret_value: 42u128, // Fixed secret value for consistency
                                shares_to_remove,
                                decoder_type,
                                ldpc_rate: rate,
                                ldpc_info_size: info_size,
                                max_iterations: 500,  // Default
                                llr_bits: 100_f64.to_bits(), // Default LLR value stored as bits
                                implementation,
                            });
                        }
                    }
                }
            }
        }
    }
    
    params
}

/// Calculate statistics from multiple benchmark runs
pub fn calculate_stats(results: &[BenchmarkResult]) -> BenchmarkSummary {
    let mut setup_times = HashMap::new();
    let mut deal_times = HashMap::new();
    let mut reconstruct_times = HashMap::new();
    let mut total_times = HashMap::new();
    let mut success_counts = HashMap::new();
    let mut params_set = HashMap::new();
    
    // Maps to store metrics for each parameter set
    let mut deal_metrics: HashMap<BenchmarkParams, Vec<Option<DealMetrics>>> = HashMap::new();
    let mut reconstruct_metrics: HashMap<BenchmarkParams, Vec<Option<ReconstructMetrics>>> = HashMap::new();
    
    for result in results {
        let params = result.params.clone();
        
        setup_times.entry(params.clone()).or_insert_with(Vec::new).push(result.setup_time);
        deal_times.entry(params.clone()).or_insert_with(Vec::new).push(result.deal_time);
        reconstruct_times.entry(params.clone()).or_insert_with(Vec::new).push(result.reconstruct_time);
        total_times.entry(params.clone()).or_insert_with(Vec::new).push(result.total_time);
        
        // Collect operation metrics
        deal_metrics.entry(params.clone()).or_insert_with(Vec::new).push(result.deal_metrics.clone());
        reconstruct_metrics.entry(params.clone()).or_insert_with(Vec::new).push(result.reconstruct_metrics.clone());
        
        *success_counts.entry(params.clone()).or_insert(0) += if result.success { 1 } else { 0 };
        params_set.insert(params.clone(), params_set.get(&params).unwrap_or(&0) + 1);
    }
    
    let mut setup_stats = HashMap::new();
    let mut deal_stats = HashMap::new();
    let mut reconstruct_stats = HashMap::new();
    let mut total_stats = HashMap::new();
    
    for (params, count) in params_set {
        // Calculate basic stats
        let mut setup_stat = BenchmarkStats::new(
            &setup_times[&params],
            success_counts[&params],
            count,
        );
        
        let mut deal_stat = BenchmarkStats::new(
            &deal_times[&params],
            success_counts[&params],
            count,
        );
        
        let mut reconstruct_stat = BenchmarkStats::new(
            &reconstruct_times[&params],
            success_counts[&params],
            count,
        );
        
        let total_stat = BenchmarkStats::new(
            &total_times[&params],
            success_counts[&params],
            count,
        );
        
        // Add phase statistics if available
        deal_stat = deal_stat.with_phase_metrics(&deal_metrics[&params], &[]);
        reconstruct_stat = reconstruct_stat.with_phase_metrics(&[], &reconstruct_metrics[&params]);
        
        setup_stats.insert(params.clone(), setup_stat);
        deal_stats.insert(params.clone(), deal_stat);
        reconstruct_stats.insert(params.clone(), reconstruct_stat);
        total_stats.insert(params.clone(), total_stat);
    }
    
    BenchmarkSummary {
        setup_stats,
        deal_stats,
        reconstruct_stats,
        total_stats,
    }
}

/// Format a duration for display
pub fn format_duration_ms(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    let micros = duration.subsec_micros() % 1000;
    
    if secs > 0 {
        if millis > 0 {
            format!("{}.{:03}s", secs, millis)
        } else {
            format!("{}s", secs)
        }
    } else if millis > 0 {
        if micros > 0 {
            format!("{}.{:03}ms", millis, micros)
        } else {
            format!("{}ms", millis)
        }
    } else {
        format!("{}µs", duration.subsec_micros())
    }
}

/// Save benchmark results to a CSV file
pub fn save_benchmark_results_to_csv(summary: &BenchmarkSummary, file_path: &str) -> io::Result<()> {
    // Save main summary
    {
        let path = format!("{}_summary.csv", file_path);
        let mut file = File::create(path)?;
        
        // Write header
        writeln!(file, "Implementation,C,InfoSize,Rate,Decoder,Phase,Avg_ms,Min_ms,Max_ms,Median_ms,StdDev_ms,SuccessRate")?;
        
        // Write total stats
        for (params, stats) in &summary.total_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Total,{},{},{},{},{},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_millis(),
                stats.min.as_millis(),
                stats.max.as_millis(),
                stats.median.as_millis(),
                stats.std_dev.as_millis(),
                stats.success_rate
            )?;
        }
        
        // Write setup stats
        for (params, stats) in &summary.setup_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Setup,{},{},{},{},{},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_millis(),
                stats.min.as_millis(),
                stats.max.as_millis(),
                stats.median.as_millis(),
                stats.std_dev.as_millis(),
                stats.success_rate
            )?;
        }
        
        // Write deal stats
        for (params, stats) in &summary.deal_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Deal,{},{},{},{},{},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_millis(),
                stats.min.as_millis(),
                stats.max.as_millis(),
                stats.median.as_millis(),
                stats.std_dev.as_millis(),
                stats.success_rate
            )?;
        }
        
        // Write reconstruct stats
        for (params, stats) in &summary.reconstruct_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Reconstruct,{},{},{},{},{},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_millis(),
                stats.min.as_millis(),
                stats.max.as_millis(),
                stats.median.as_millis(),
                stats.std_dev.as_millis(),
                stats.success_rate
            )?;
        }
    }
    
    // Save detailed phase stats
    {
        let path = format!("{}_phases.csv", file_path);
        let mut file = File::create(path)?;
        
        // Write header
        writeln!(file, "Implementation,C,InfoSize,Rate,Decoder,Operation,Phase,Avg_ms,Min_ms,Max_ms,Percentage")?;
        
        // Write deal phase stats
        for (params, stats) in &summary.deal_stats {
            if let Some(phase_metrics) = &stats.phase_metrics {
                for (name, phase_stat) in phase_metrics {
                    writeln!(file, "{},{},{:?},{:?},{:?},Deal,\"{}\",{},{},{},{}",
                        params.implementation,
                        params.c_value,
                        params.ldpc_info_size,
                        params.ldpc_rate,
                        params.decoder_type,
                        name,
                        phase_stat.avg_duration.as_micros() as f64 / 1000.0,
                        phase_stat.min_duration.as_micros() as f64 / 1000.0,
                        phase_stat.max_duration.as_micros() as f64 / 1000.0,
                        phase_stat.avg_percentage
                    )?;
                }
            }
        }
        
        // Write reconstruct phase stats
        for (params, stats) in &summary.reconstruct_stats {
            if let Some(phase_metrics) = &stats.phase_metrics {
                for (name, phase_stat) in phase_metrics {
                    writeln!(file, "{},{},{:?},{:?},{:?},Reconstruct,\"{}\",{},{},{},{}",
                        params.implementation,
                        params.c_value,
                        params.ldpc_info_size,
                        params.ldpc_rate,
                        params.decoder_type,
                        name,
                        phase_stat.avg_duration.as_micros() as f64 / 1000.0,
                        phase_stat.min_duration.as_micros() as f64 / 1000.0,
                        phase_stat.max_duration.as_micros() as f64 / 1000.0,
                        phase_stat.avg_percentage
                    )?;
                }
            }
        }
    }
    
    println!("Benchmark results saved to {}_summary.csv and {}_phases.csv", file_path, file_path);
    Ok(())
}

/// Print benchmark results in a table format
pub fn print_benchmark_results(summary: &BenchmarkSummary, show_detail: bool) {
    println!("\n{:-^80}", " BENCHMARK RESULTS SUMMARY ");
    
    println!("\n{:-^80}", " TOTAL EXECUTION TIME ");
    println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<8}", 
        "Parameters", "Avg", "Min", "Max", "Median", "StdDev", "Success");
    println!("{:-^110}", "");
    
    for (params, stats) in &summary.total_stats {
        println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12} | {:<8}", 
            format!("{}:c{}:{:?}:{:?}:{:?}", 
                params.implementation, 
                params.c_value, 
                params.ldpc_info_size, 
                params.ldpc_rate,
                params.decoder_type),
            format_duration_ms(stats.avg),
            format_duration_ms(stats.min),
            format_duration_ms(stats.max),
            format_duration_ms(stats.median),
            format_duration_ms(stats.std_dev),
            format!("{:.0}%", stats.success_rate * 100.0));
    }
    
    if show_detail {
        // Setup time details
        println!("\n{:-^80}", " SETUP TIME ");
        println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}", 
            "Parameters", "Avg", "Min", "Max", "Median", "StdDev");
        println!("{:-^100}", "");
        
        for (params, stats) in &summary.setup_stats {
            println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}", 
                format!("{}:c{}:{:?}:{:?}:{:?}", 
                    params.implementation, 
                    params.c_value, 
                    params.ldpc_info_size, 
                    params.ldpc_rate,
                    params.decoder_type),
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max),
                format_duration_ms(stats.median),
                format_duration_ms(stats.std_dev));
        }
        
        // Deal time details
        println!("\n{:-^80}", " DEAL TIME ");
        println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}", 
            "Parameters", "Avg", "Min", "Max", "Median", "StdDev");
        println!("{:-^100}", "");
        
        for (params, stats) in &summary.deal_stats {
            println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}", 
                format!("{}:c{}:{:?}:{:?}:{:?}", 
                    params.implementation, 
                    params.c_value, 
                    params.ldpc_info_size, 
                    params.ldpc_rate,
                    params.decoder_type),
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max),
                format_duration_ms(stats.median),
                format_duration_ms(stats.std_dev));
                
            // Print phase details if available
            if let Some(phase_metrics) = &stats.phase_metrics {
                println!("  {:<28} | {:<12} | {:<12} | {:<12} | {:<12}", 
                    "Phase", "Avg", "Min", "Max", "% of Total");
                println!("  {:-^80}", "");
                
                // Sort phases by percentage (descending)
                let mut phases: Vec<(&String, &PhaseStats)> = phase_metrics.iter().collect();
                phases.sort_by(|(_, a), (_, b)| 
                    b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());
                
                for (name, phase_stat) in phases {
                    println!("  {:<28} | {:<12} | {:<12} | {:<12} | {:<12}", 
                        name,
                        format_duration_ms(phase_stat.avg_duration),
                        format_duration_ms(phase_stat.min_duration),
                        format_duration_ms(phase_stat.max_duration),
                        format!("{:.2}%", phase_stat.avg_percentage));
                }
                println!("");
            }
        }
        
        // Reconstruct time details
        println!("\n{:-^80}", " RECONSTRUCT TIME ");
        println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}", 
            "Parameters", "Avg", "Min", "Max", "Median", "StdDev");
        println!("{:-^100}", "");
        
        for (params, stats) in &summary.reconstruct_stats {
            println!("{:<40} | {:<12} | {:<12} | {:<12} | {:<12} | {:<12}", 
                format!("{}:c{}:{:?}:{:?}:{:?}", 
                    params.implementation, 
                    params.c_value, 
                    params.ldpc_info_size, 
                    params.ldpc_rate,
                    params.decoder_type),
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max),
                format_duration_ms(stats.median),
                format_duration_ms(stats.std_dev));
                
            // Print phase details if available
            if let Some(phase_metrics) = &stats.phase_metrics {
                println!("  {:<28} | {:<12} | {:<12} | {:<12} | {:<12}", 
                    "Phase", "Avg", "Min", "Max", "% of Total");
                println!("  {:-^80}", "");
                
                // Sort phases by percentage (descending)
                let mut phases: Vec<(&String, &PhaseStats)> = phase_metrics.iter().collect();
                phases.sort_by(|(_, a), (_, b)| 
                    b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());
                
                for (name, phase_stat) in phases {
                    println!("  {:<28} | {:<12} | {:<12} | {:<12} | {:<12}", 
                        name,
                        format_duration_ms(phase_stat.avg_duration),
                        format_duration_ms(phase_stat.min_duration),
                        format_duration_ms(phase_stat.max_duration),
                        format!("{:.2}%", phase_stat.avg_percentage));
                }
                println!("");
            }
        }
    }
}

/// Run a comprehensive benchmark with multiple parameter combinations
pub fn run_comprehensive_benchmark<F: PrimeField<BigInt = BigInt<4>> + Debug>(
    c_values: &[usize],
    shares_to_remove_values: &[usize],
    decoder_types: &[DecoderImplementation],
    ldpc_rates: &[AR4JARate],
    ldpc_info_sizes: &[AR4JAInfoSize],
    implementations: &[Implementation],
    runs_per_config: usize,
    show_detail: bool,
    output_file: Option<&str>,
) {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    println!("Starting comprehensive benchmark at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    let params = generate_benchmark_params(
        c_values,
        shares_to_remove_values,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
    );
    
    println!("Will run {} parameter combinations with {} runs each ({} total runs)",
        params.len(),
        runs_per_config,
        params.len() * runs_per_config);
    
    let multi_progress = Arc::new(MultiProgress::new());
    let mp = Arc::clone(&multi_progress);
    
    let mut all_results = Vec::new();
    
    for param in params {
        let results = run_multiple_benchmarks::<F>(&param, runs_per_config, &mp);
        all_results.extend(results);
    }
    
    let summary = calculate_stats(&all_results);
    print_benchmark_results(&summary, show_detail);
    
    // Save results to CSV if output file is specified
    if let Some(file_path) = output_file {
        let output_path = if file_path.is_empty() {
            // Create descriptive filename
            let implementation_str = if implementations.contains(&Implementation::Sequential) && 
                                        implementations.contains(&Implementation::Parallel) {
                "both"
            } else if implementations.contains(&Implementation::Sequential) {
                "seq"
            } else {
                "par"
            };
            
            let c_values_str = c_values.iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("_");
            
            let rates_str = ldpc_rates.iter()
                .map(|r| format!("{:?}", r))
                .collect::<Vec<String>>()
                .join("_");
            
            let info_sizes_str = ldpc_info_sizes.iter()
                .map(|s| format!("{:?}", s))
                .collect::<Vec<String>>()
                .join("_");
                
            // Include decoder type in filename if only one is used
            let decoder_str = if decoder_types.len() == 1 {
                format!("_{:?}", decoder_types[0])
            } else {
                String::from("_multi_decoder")
            };
                
            format!("benchmark_{}_c{}_{}_{}_{}{}",
                timestamp,
                c_values_str,
                implementation_str,
                rates_str,
                info_sizes_str,
                decoder_str)
        } else {
            file_path.to_string()
        };
        
        if let Err(e) = save_benchmark_results_to_csv(&summary, &output_path) {
            println!("Error saving benchmark results to CSV: {}", e);
        }
    }
    
    println!("\nBenchmark completed at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
}