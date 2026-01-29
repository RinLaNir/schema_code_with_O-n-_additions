use crate::aos;
use crate::aos_parallel;
use crate::types::{CodeInitParams, Share, DealMetrics, ReconstructMetrics, DecodingStats, ThroughputMetrics, ParallelMetrics};
use crate::{log_info, log_success, log_error, log_warning, log_verbose};
use ark_ff::{BigInt, PrimeField};
use chrono::Local;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle, ProgressDrawTarget};
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::Debug;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::hash::{Hash, Hasher};
use std::fs::File;
use std::io::{self, Write};

fn serialize_duration_as_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
}

fn deserialize_duration_from_ms<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ms = f64::deserialize(deserializer)?;
    Ok(Duration::from_secs_f64(ms / 1000.0))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkResult {
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
    pub setup_time: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
    pub deal_time: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
    pub reconstruct_time: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms", deserialize_with = "deserialize_duration_from_ms", default)]
    pub total_time: Duration,
    #[serde(skip)]
    pub params: BenchmarkParams,
    #[serde(default)]
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deal_metrics: Option<DealMetrics>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reconstruct_metrics: Option<ReconstructMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    pub c_value: usize,
    pub secret_value: u128,
    pub shares_to_remove: isize,
    pub decoder_type: DecoderImplementation,
    pub ldpc_rate: AR4JARate,
    pub ldpc_info_size: AR4JAInfoSize,
    pub max_iterations: usize,
    pub llr_bits: u64,
    pub implementation: Implementation,
}

impl Default for BenchmarkParams {
    fn default() -> Self {
        Self {
            c_value: 10,
            secret_value: 0,
            shares_to_remove: 0,
            decoder_type: DecoderImplementation::Aminstarf32,
            ldpc_rate: AR4JARate::R4_5,
            ldpc_info_size: AR4JAInfoSize::K1024,
            max_iterations: 300,
            llr_bits: 0,
            implementation: Implementation::Sequential,
        }
    }
}

impl Hash for BenchmarkParams {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.c_value.hash(state);
        self.secret_value.hash(state);
        self.shares_to_remove.hash(state);
        
        std::mem::discriminant(&self.decoder_type).hash(state);
        std::mem::discriminant(&self.ldpc_rate).hash(state);
        std::mem::discriminant(&self.ldpc_info_size).hash(state);
        
        self.max_iterations.hash(state);
        self.llr_bits.hash(state);
        self.implementation.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Default)]
pub enum Implementation {
    #[default]
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

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkStats {
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub min: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub max: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub avg: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub median: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub std_dev: Duration,
    pub success_rate: f64,
    #[allow(dead_code)]
    pub runs: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_metrics: Option<HashMap<String, PhaseStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoding_stats: Option<DecodingStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput: Option<ThroughputMetrics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhaseStats {
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub avg_duration: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms")]
    pub min_duration: Duration,
    #[serde(serialize_with = "serialize_duration_as_ms")]
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
                decoding_stats: None,
                throughput: None,
            };
        }

        let mut sorted_times = times.to_vec();
        sorted_times.sort();

        let min = *sorted_times.first().expect("times vector should not be empty");
        let max = *sorted_times.last().expect("times vector should not be empty");
        
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
            decoding_stats: None,
            throughput: None,
        }
    }
    
    pub fn with_phase_metrics(mut self, deal_metrics: &[Option<DealMetrics>], reconstruct_metrics: &[Option<ReconstructMetrics>]) -> Self {
        let mut phase_stats = HashMap::new();
        
        if !deal_metrics.is_empty() {
            let metrics: Vec<&DealMetrics> = deal_metrics.iter().filter_map(|m| m.as_ref()).collect();
            if !metrics.is_empty() {
                phase_stats.insert(String::from("Random vector generation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.rand_vec_generation.duration).collect(),
                    metrics.iter().map(|m| m.rand_vec_generation.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Dot product calculation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.dot_product.duration).collect(),
                    metrics.iter().map(|m| m.dot_product.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Message matrix creation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.matrix_creation.duration).collect(),
                    metrics.iter().map(|m| m.matrix_creation.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Encoding phase"), calculate_phase_stats(
                    metrics.iter().map(|m| m.encoding.duration).collect(),
                    metrics.iter().map(|m| m.encoding.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Share creation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.share_creation.duration).collect(),
                    metrics.iter().map(|m| m.share_creation.percentage).collect(),
                ));
            }
        }
        
        if !reconstruct_metrics.is_empty() {
            let metrics: Vec<&ReconstructMetrics> = reconstruct_metrics.iter().filter_map(|m| m.as_ref()).collect();
            if !metrics.is_empty() {
                phase_stats.insert(String::from("Matrix setup"), calculate_phase_stats(
                    metrics.iter().map(|m| m.matrix_setup.duration).collect(),
                    metrics.iter().map(|m| m.matrix_setup.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Row decoding"), calculate_phase_stats(
                    metrics.iter().map(|m| m.row_decoding.duration).collect(),
                    metrics.iter().map(|m| m.row_decoding.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Field element reconstruction"), calculate_phase_stats(
                    metrics.iter().map(|m| m.field_reconstruction.duration).collect(),
                    metrics.iter().map(|m| m.field_reconstruction.percentage).collect(),
                ));
                
                phase_stats.insert(String::from("Final computation"), calculate_phase_stats(
                    metrics.iter().map(|m| m.final_computation.duration).collect(),
                    metrics.iter().map(|m| m.final_computation.percentage).collect(),
                ));
            }
        }
        
        if !phase_stats.is_empty() {
            self.phase_metrics = Some(phase_stats);
        }
        
        if !reconstruct_metrics.is_empty() {
            let decoding_stats_list: Vec<&DecodingStats> = reconstruct_metrics
                .iter()
                .filter_map(|m| m.as_ref())
                .filter_map(|m| m.decoding_stats.as_ref())
                .collect();
            
            if !decoding_stats_list.is_empty() {
                let total_rows: usize = decoding_stats_list.iter().map(|d| d.total_rows).sum();
                let successful_rows: usize = decoding_stats_list.iter().map(|d| d.successful_rows).sum();
                let failed_rows: usize = decoding_stats_list.iter().map(|d| d.failed_rows).sum();
                let total_iterations: usize = decoding_stats_list.iter().map(|d| d.total_iterations).sum();
                let max_iterations_hit: usize = decoding_stats_list.iter().map(|d| d.max_iterations_hit).sum();
                
                let avg_iterations = if successful_rows > 0 {
                    total_iterations as f64 / successful_rows as f64
                } else {
                    0.0
                };
                
                self.decoding_stats = Some(DecodingStats {
                    total_rows,
                    successful_rows,
                    failed_rows,
                    total_iterations,
                    avg_iterations,
                    max_iterations_hit,
                });
            }
        }
        
        self
    }
    
    pub fn with_throughput(mut self, c_value: usize, info_bits: usize) -> Self {
        if self.avg.as_nanos() > 0 {
            let shares_per_second = c_value as f64 / self.avg.as_secs_f64();
            let bits_per_second = (c_value * info_bits) as f64 / self.avg.as_secs_f64();
            
            self.throughput = Some(ThroughputMetrics {
                shares_per_second,
                bits_per_second,
            });
        }
        self
    }
}

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

fn get_info_bits(info_size: AR4JAInfoSize) -> usize {
    match info_size {
        AR4JAInfoSize::K1024 => 1024,
        AR4JAInfoSize::K4096 => 4096,
        AR4JAInfoSize::K16384 => 16384,
    }
}

#[derive(Clone)]
pub struct BenchmarkSummary {
    pub setup_stats: HashMap<BenchmarkParams, BenchmarkStats>, 
    pub deal_stats: HashMap<BenchmarkParams, BenchmarkStats>,
    pub reconstruct_stats: HashMap<BenchmarkParams, BenchmarkStats>,
    pub total_stats: HashMap<BenchmarkParams, BenchmarkStats>,
}

fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: isize) {
    let mut rng = thread_rng();
    shares.shuffle(&mut rng);
    
    let count_to_remove = if num_to_remove < 0 {
        let percentage = (-num_to_remove) as f64;
        let count = (shares.len() as f64 * percentage / 100.0).round() as usize;
        count
    } else {
        num_to_remove as usize
    };
    
    if count_to_remove <= shares.len() {
        shares.drain(0..count_to_remove);
    }
}

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
                let setup_start = Instant::now();
                let pp = aos::setup::<F>(code_params, params.c_value as u32);
                let setup_duration = setup_start.elapsed();

                if let Some(pb) = progress {
                    pb.set_message("Dealing shares...");
                }
                
                let deal_start = Instant::now();
                let mut shares = aos::deal(&pp, secret);
                let deal_duration = deal_start.elapsed();
                let deal_metrics = shares.metrics.clone();
                
                if let Some(pb) = progress {
                    pb.set_message("Removing shares...");
                }
                
                remove_random_shares(&mut shares.shares, params.shares_to_remove as isize);
                
                if let Some(pb) = progress {
                    pb.set_message("Reconstructing...");
                }
                
                let reconstruct_start = Instant::now();
                let (reconstructed_secret, reconstruct_metrics) = aos::reconstruct(&pp, &shares);
                let reconstruct_duration = reconstruct_start.elapsed();
                
                (setup_duration, deal_duration, reconstruct_duration, reconstructed_secret, deal_metrics, reconstruct_metrics)
            },
            Implementation::Parallel => {
                let setup_start = Instant::now();
                let pp = aos_parallel::setup::<F>(code_params, params.c_value as u32);
                let setup_duration = setup_start.elapsed();

                if let Some(pb) = progress {
                    pb.set_message("Dealing shares...");
                }
                
                let deal_start = Instant::now();
                let mut shares = aos_parallel::deal(&pp, secret);
                let deal_duration = deal_start.elapsed();
                let deal_metrics = shares.metrics.clone();
                
                if let Some(pb) = progress {
                    pb.set_message("Removing shares...");
                }
                
                remove_random_shares(&mut shares.shares, params.shares_to_remove as isize);
                
                if let Some(pb) = progress {
                    pb.set_message("Reconstructing...");
                }
                
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
    
    log_info!("Benchmarking {} (c={}, rate={:?}, info_size={:?}, decoder={:?})", 
        params.implementation,
        params.c_value,
        params.ldpc_rate,
        params.ldpc_info_size,
        params.decoder_type
    );

    let mut results = Vec::with_capacity(num_runs);
    
    for i in 0..num_runs {
        log_info!("Run {}/{} - {} (c={}, rate={:?}, info_size={:?}, decoder={:?})", 
            i + 1, 
            num_runs,
            params.implementation,
            params.c_value,
            params.ldpc_rate,
            params.ldpc_info_size,
            params.decoder_type
        );
        
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

pub fn generate_benchmark_params(
    c_values: &[usize],
    shares_to_remove_values: &[isize],
    decoder_types: &[DecoderImplementation],
    ldpc_rates: &[AR4JARate],
    ldpc_info_sizes: &[AR4JAInfoSize],
    implementations: &[Implementation],
    secret_value: u128,
    max_iterations: usize,
    llr_value: f64,
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
                                secret_value,
                                shares_to_remove,
                                decoder_type,
                                ldpc_rate: rate,
                                ldpc_info_size: info_size,
                                max_iterations,
                                llr_bits: llr_value.to_bits(),
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

pub fn calculate_stats(results: &[BenchmarkResult]) -> BenchmarkSummary {
    let mut setup_times = HashMap::new();
    let mut deal_times = HashMap::new();
    let mut reconstruct_times = HashMap::new();
    let mut total_times = HashMap::new();
    let mut success_counts = HashMap::new();
    let mut params_set = HashMap::new();
    
    let mut deal_metrics: HashMap<BenchmarkParams, Vec<Option<DealMetrics>>> = HashMap::new();
    let mut reconstruct_metrics: HashMap<BenchmarkParams, Vec<Option<ReconstructMetrics>>> = HashMap::new();
    
    for result in results {
        let params = result.params.clone();
        
        setup_times.entry(params.clone()).or_insert_with(Vec::new).push(result.setup_time);
        deal_times.entry(params.clone()).or_insert_with(Vec::new).push(result.deal_time);
        reconstruct_times.entry(params.clone()).or_insert_with(Vec::new).push(result.reconstruct_time);
        total_times.entry(params.clone()).or_insert_with(Vec::new).push(result.total_time);
        
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
        let setup_stat = BenchmarkStats::new(
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
        
        deal_stat = deal_stat.with_phase_metrics(&deal_metrics[&params], &[]);
        reconstruct_stat = reconstruct_stat.with_phase_metrics(&[], &reconstruct_metrics[&params]);
        
        let info_bits = get_info_bits(params.ldpc_info_size);
        deal_stat = deal_stat.with_throughput(params.c_value, info_bits);
        
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

pub fn save_benchmark_results_to_csv(summary: &BenchmarkSummary, file_path: &str) -> io::Result<()> {
    {
        let path = format!("{}_summary.csv", file_path);
        let mut file = File::create(path)?;
        
        writeln!(file, "Implementation,C,InfoSize,Rate,Decoder,Phase,Avg_ms,Min_ms,Max_ms,Median_ms,StdDev_ms,SuccessRate")?;
        
        for (params, stats) in &summary.total_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Total,{:.3},{:.3},{:.3},{:.3},{:.3},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_secs_f64() * 1000.0,
                stats.min.as_secs_f64() * 1000.0,
                stats.max.as_secs_f64() * 1000.0,
                stats.median.as_secs_f64() * 1000.0,
                stats.std_dev.as_secs_f64() * 1000.0,
                stats.success_rate
            )?;
        }
        
        for (params, stats) in &summary.setup_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Setup,{:.3},{:.3},{:.3},{:.3},{:.3},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_secs_f64() * 1000.0,
                stats.min.as_secs_f64() * 1000.0,
                stats.max.as_secs_f64() * 1000.0,
                stats.median.as_secs_f64() * 1000.0,
                stats.std_dev.as_secs_f64() * 1000.0,
                stats.success_rate
            )?;
        }
        
        for (params, stats) in &summary.deal_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Deal,{:.3},{:.3},{:.3},{:.3},{:.3},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_secs_f64() * 1000.0,
                stats.min.as_secs_f64() * 1000.0,
                stats.max.as_secs_f64() * 1000.0,
                stats.median.as_secs_f64() * 1000.0,
                stats.std_dev.as_secs_f64() * 1000.0,
                stats.success_rate
            )?;
        }
        
        for (params, stats) in &summary.reconstruct_stats {
            writeln!(file, "{},{},{:?},{:?},{:?},Reconstruct,{:.3},{:.3},{:.3},{:.3},{:.3},{}",
                params.implementation,
                params.c_value,
                params.ldpc_info_size,
                params.ldpc_rate,
                params.decoder_type,
                stats.avg.as_secs_f64() * 1000.0,
                stats.min.as_secs_f64() * 1000.0,
                stats.max.as_secs_f64() * 1000.0,
                stats.median.as_secs_f64() * 1000.0,
                stats.std_dev.as_secs_f64() * 1000.0,
                stats.success_rate
            )?;
        }
    }
    
    {
        let path = format!("{}_phases.csv", file_path);
        let mut file = File::create(path)?;
        
        writeln!(file, "Implementation,C,InfoSize,Rate,Decoder,Operation,Phase,Avg_ms,Min_ms,Max_ms,Percentage")?;
        
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
    
    log_success!("Benchmark results saved to {}_summary.csv and {}_phases.csv", file_path, file_path);
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub metadata: ReportMetadata,
    pub configurations: Vec<ConfigurationResult>,
} 

#[derive(Serialize, Deserialize)]
pub struct ReportMetadata {
    pub timestamp: String,
    pub version: String,
    pub total_runs: usize,
    pub warmup_runs: usize,
    pub total_configurations: usize,
} 

#[derive(Serialize, Deserialize, Clone)]
pub struct SerializableParams {
    pub implementation: String,
    pub c_value: usize,
    pub secret_value: u128,
    pub shares_to_remove: isize,
    pub decoder_type: String,
    pub ldpc_rate: String,
    pub ldpc_info_size: String,
    pub max_iterations: usize,
    pub llr_value: f64,
} 

impl From<&BenchmarkParams> for SerializableParams {
    fn from(params: &BenchmarkParams) -> Self {
        SerializableParams {
            implementation: format!("{}", params.implementation),
            c_value: params.c_value,
            secret_value: params.secret_value,
            shares_to_remove: params.shares_to_remove,
            decoder_type: format!("{:?}", params.decoder_type),
            ldpc_rate: format!("{:?}", params.ldpc_rate),
            ldpc_info_size: format!("{:?}", params.ldpc_info_size),
            max_iterations: params.max_iterations,
            llr_value: f64::from_bits(params.llr_bits),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimingStatsJson {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub median_ms: f64,
    pub std_dev_ms: f64,
    pub success_rate: f64,
} 

impl From<&BenchmarkStats> for TimingStatsJson {
    fn from(stats: &BenchmarkStats) -> Self {
        TimingStatsJson {
            avg_ms: stats.avg.as_secs_f64() * 1000.0,
            min_ms: stats.min.as_secs_f64() * 1000.0,
            max_ms: stats.max.as_secs_f64() * 1000.0,
            median_ms: stats.median.as_secs_f64() * 1000.0,
            std_dev_ms: stats.std_dev.as_secs_f64() * 1000.0,
            success_rate: stats.success_rate,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PhaseStatsJson {
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub percentage: f64,
}

impl From<&PhaseStats> for PhaseStatsJson {
    fn from(stats: &PhaseStats) -> Self {
        PhaseStatsJson {
            avg_ms: stats.avg_duration.as_secs_f64() * 1000.0,
            min_ms: stats.min_duration.as_secs_f64() * 1000.0,
            max_ms: stats.max_duration.as_secs_f64() * 1000.0,
            percentage: stats.avg_percentage,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PhaseSummaries {
    pub total: TimingStatsJson,
    pub setup: TimingStatsJson,
    pub deal: TimingStatsJson,
    pub reconstruct: TimingStatsJson,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PhaseBreakdown {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deal: Option<HashMap<String, PhaseStatsJson>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reconstruct: Option<HashMap<String, PhaseStatsJson>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigurationResult {
    pub params: SerializableParams,
    pub summary: PhaseSummaries,
    pub phases: PhaseBreakdown,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decoding_stats: Option<AggregatedDecodingStats>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parallel_metrics: Option<ParallelMetrics>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub throughput: Option<ThroughputMetrics>,
    #[serde(default)]
    pub individual_runs: Vec<BenchmarkResult>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AggregatedDecodingStats {
    pub total_rows: usize,
    pub avg_successful_rows: f64,
    pub avg_failed_rows: f64,
    pub avg_iterations: f64,
    pub avg_max_iterations_hit: f64,
    pub avg_success_rate: f64,
}

pub fn save_benchmark_results_to_json(
    summary: &BenchmarkSummary,
    results: &[BenchmarkResult],
    warmup_runs: usize,
    file_path: &str,
) -> io::Result<()> {
    let path = if file_path.ends_with(".json") {
        file_path.to_string()
    } else {
        format!("{}.json", file_path)
    };
    
    let mut results_by_params: HashMap<BenchmarkParams, Vec<&BenchmarkResult>> = HashMap::new();
    for result in results {
        results_by_params.entry(result.params.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }
    
    let mut configurations = Vec::new();
    
    for (params, param_results) in results_by_params {
        let total_stats = summary.total_stats.get(&params);
        let setup_stats = summary.setup_stats.get(&params);
        let deal_stats = summary.deal_stats.get(&params);
        let reconstruct_stats = summary.reconstruct_stats.get(&params);
        
        if total_stats.is_none() || setup_stats.is_none() || deal_stats.is_none() || reconstruct_stats.is_none() {
            continue;
        }
        
        let total_stats = total_stats.unwrap();
        let setup_stats = setup_stats.unwrap();
        let deal_stats = deal_stats.unwrap();
        let reconstruct_stats = reconstruct_stats.unwrap();
        
        let deal_phases = deal_stats.phase_metrics.as_ref().map(|pm| {
            pm.iter().map(|(k, v)| (k.clone(), PhaseStatsJson::from(v))).collect()
        });
        
        let reconstruct_phases = reconstruct_stats.phase_metrics.as_ref().map(|pm| {
            pm.iter().map(|(k, v)| (k.clone(), PhaseStatsJson::from(v))).collect()
        });
        
        let decoding_stats = aggregate_decoding_stats(&param_results);
        
        let throughput = calculate_throughput(&params, total_stats);
        
        let parallel_metrics = if params.implementation == Implementation::Parallel {
            Some(ParallelMetrics {
                thread_count: rayon::current_num_threads(),
                speedup: None,
                efficiency: None,
            })
        } else {
            None
        };
        
        let config_result = ConfigurationResult {
            params: SerializableParams::from(&params),
            summary: PhaseSummaries {
                total: TimingStatsJson::from(total_stats),
                setup: TimingStatsJson::from(setup_stats),
                deal: TimingStatsJson::from(deal_stats),
                reconstruct: TimingStatsJson::from(reconstruct_stats),
            },
            phases: PhaseBreakdown {
                deal: deal_phases,
                reconstruct: reconstruct_phases,
            },
            decoding_stats,
            parallel_metrics,
            throughput: Some(throughput),
            individual_runs: param_results.iter().map(|r| (*r).clone()).collect(),
        };
        
        configurations.push(config_result);
    }
    
    let report = BenchmarkReport {
        metadata: ReportMetadata {
            timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            total_runs: results.len(),
            warmup_runs,
            total_configurations: configurations.len(),
        },
        configurations,
    };
    
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    let mut file = File::create(&path)?;
    file.write_all(json.as_bytes())?;
    
    log_success!("Benchmark results saved to {}", path);
    Ok(())
}

pub fn import_from_json(path: &std::path::Path) -> Result<BenchmarkSummary, String> {
    use std::fs;
    
    let json_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let report: BenchmarkReport = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let mut setup_stats = HashMap::new();
    let mut deal_stats = HashMap::new();
    let mut reconstruct_stats = HashMap::new();
    let mut total_stats = HashMap::new();
    
    for config in report.configurations {
        let implementation = parse_implementation(&config.params.implementation)?;
        let decoder_type = parse_decoder_type(&config.params.decoder_type)?;
        let ldpc_rate = parse_ldpc_rate(&config.params.ldpc_rate)?;
        let ldpc_info_size = parse_ldpc_info_size(&config.params.ldpc_info_size)?;
        
        let params = BenchmarkParams {
            c_value: config.params.c_value,
            secret_value: config.params.secret_value,
            shares_to_remove: config.params.shares_to_remove,
            decoder_type,
            ldpc_rate,
            ldpc_info_size,
            max_iterations: config.params.max_iterations,
            llr_bits: config.params.llr_value.to_bits(),
            implementation,
        };
        
        let setup_stat = timing_stats_from_json(&config.summary.setup);
        let mut deal_stat = timing_stats_from_json(&config.summary.deal);
        let mut reconstruct_stat = timing_stats_from_json(&config.summary.reconstruct);
        let total_stat = timing_stats_from_json(&config.summary.total);
        
        if let Some(deal_phases) = &config.phases.deal {
            deal_stat.phase_metrics = Some(phase_stats_from_json(deal_phases));
        }
        if let Some(reconstruct_phases) = &config.phases.reconstruct {
            reconstruct_stat.phase_metrics = Some(phase_stats_from_json(reconstruct_phases));
        }
        
        if let Some(dec_stats) = &config.decoding_stats {
            reconstruct_stat.decoding_stats = Some(DecodingStats {
                total_rows: dec_stats.total_rows,
                successful_rows: dec_stats.avg_successful_rows as usize,
                failed_rows: dec_stats.avg_failed_rows as usize,
                total_iterations: 0,
                avg_iterations: dec_stats.avg_iterations,
                max_iterations_hit: dec_stats.avg_max_iterations_hit as usize,
            });
        }
        
        if let Some(throughput) = &config.throughput {
            deal_stat.throughput = Some(throughput.clone());
        }
        
        setup_stats.insert(params.clone(), setup_stat);
        deal_stats.insert(params.clone(), deal_stat);
        reconstruct_stats.insert(params.clone(), reconstruct_stat);
        total_stats.insert(params, total_stat);
    }
    
    Ok(BenchmarkSummary {
        setup_stats,
        deal_stats,
        reconstruct_stats,
        total_stats,
    })
}

fn parse_implementation(s: &str) -> Result<Implementation, String> {
    match s.to_lowercase().as_str() {
        "sequential" => Ok(Implementation::Sequential),
        "parallel" => Ok(Implementation::Parallel),
        _ => Err(format!("Unknown implementation: {}", s)),
    }
}

fn parse_decoder_type(s: &str) -> Result<DecoderImplementation, String> {
    match s {
        "Phif64" => Ok(DecoderImplementation::Phif64),
        "Phif32" => Ok(DecoderImplementation::Phif32),
        "Tanhf64" => Ok(DecoderImplementation::Tanhf64),
        "Tanhf32" => Ok(DecoderImplementation::Tanhf32),
        "Minstarapproxf64" => Ok(DecoderImplementation::Minstarapproxf64),
        "Minstarapproxf32" => Ok(DecoderImplementation::Minstarapproxf32),
        "Minstarapproxi8" => Ok(DecoderImplementation::Minstarapproxi8),
        "Minstarapproxi8Jones" => Ok(DecoderImplementation::Minstarapproxi8Jones),
        "Minstarapproxi8PartialHardLimit" => Ok(DecoderImplementation::Minstarapproxi8PartialHardLimit),
        "Minstarapproxi8JonesPartialHardLimit" => Ok(DecoderImplementation::Minstarapproxi8JonesPartialHardLimit),
        "Minstarapproxi8Deg1Clip" => Ok(DecoderImplementation::Minstarapproxi8Deg1Clip),
        "Minstarapproxi8JonesDeg1Clip" => Ok(DecoderImplementation::Minstarapproxi8JonesDeg1Clip),
        "Minstarapproxi8PartialHardLimitDeg1Clip" => Ok(DecoderImplementation::Minstarapproxi8PartialHardLimitDeg1Clip),
        "Minstarapproxi8JonesPartialHardLimitDeg1Clip" => Ok(DecoderImplementation::Minstarapproxi8JonesPartialHardLimitDeg1Clip),
        "Aminstarf64" => Ok(DecoderImplementation::Aminstarf64),
        "Aminstarf32" => Ok(DecoderImplementation::Aminstarf32),
        "Aminstari8" => Ok(DecoderImplementation::Aminstari8),
        "Aminstari8Jones" => Ok(DecoderImplementation::Aminstari8Jones),
        "Aminstari8PartialHardLimit" => Ok(DecoderImplementation::Aminstari8PartialHardLimit),
        "Aminstari8JonesPartialHardLimit" => Ok(DecoderImplementation::Aminstari8JonesPartialHardLimit),
        "Aminstari8Deg1Clip" => Ok(DecoderImplementation::Aminstari8Deg1Clip),
        "Aminstari8JonesDeg1Clip" => Ok(DecoderImplementation::Aminstari8JonesDeg1Clip),
        "Aminstari8PartialHardLimitDeg1Clip" => Ok(DecoderImplementation::Aminstari8PartialHardLimitDeg1Clip),
        "Aminstari8JonesPartialHardLimitDeg1Clip" => Ok(DecoderImplementation::Aminstari8JonesPartialHardLimitDeg1Clip),
        "HLPhif64" => Ok(DecoderImplementation::HLPhif64),
        "HLPhif32" => Ok(DecoderImplementation::HLPhif32),
        "HLTanhf64" => Ok(DecoderImplementation::HLTanhf64),
        "HLTanhf32" => Ok(DecoderImplementation::HLTanhf32),
        "HLMinstarapproxf64" => Ok(DecoderImplementation::HLMinstarapproxf64),
        "HLMinstarapproxf32" => Ok(DecoderImplementation::HLMinstarapproxf32),
        "HLMinstarapproxi8" => Ok(DecoderImplementation::HLMinstarapproxi8),
        "HLMinstarapproxi8PartialHardLimit" => Ok(DecoderImplementation::HLMinstarapproxi8PartialHardLimit),
        "HLAminstarf64" => Ok(DecoderImplementation::HLAminstarf64),
        "HLAminstarf32" => Ok(DecoderImplementation::HLAminstarf32),
        "HLAminstari8" => Ok(DecoderImplementation::HLAminstari8),
        "HLAminstari8PartialHardLimit" => Ok(DecoderImplementation::HLAminstari8PartialHardLimit),
        _ => Err(format!("Unknown decoder type: {}", s)),
    }
}

fn parse_ldpc_rate(s: &str) -> Result<AR4JARate, String> {
    match s {
        "R1_2" => Ok(AR4JARate::R1_2),
        "R2_3" => Ok(AR4JARate::R2_3),
        "R4_5" => Ok(AR4JARate::R4_5),
        _ => Err(format!("Unknown LDPC rate: {}", s)),
    }
}

fn parse_ldpc_info_size(s: &str) -> Result<AR4JAInfoSize, String> {
    match s {
        "K1024" => Ok(AR4JAInfoSize::K1024),
        "K4096" => Ok(AR4JAInfoSize::K4096),
        "K16384" => Ok(AR4JAInfoSize::K16384),
        _ => Err(format!("Unknown LDPC info size: {}", s)),
    }
}

fn timing_stats_from_json(json: &TimingStatsJson) -> BenchmarkStats {
    BenchmarkStats {
        min: Duration::from_secs_f64(json.min_ms / 1000.0),
        max: Duration::from_secs_f64(json.max_ms / 1000.0),
        avg: Duration::from_secs_f64(json.avg_ms / 1000.0),
        median: Duration::from_secs_f64(json.median_ms / 1000.0),
        std_dev: Duration::from_secs_f64(json.std_dev_ms / 1000.0),
        success_rate: json.success_rate,
        runs: 0,
        phase_metrics: None,
        decoding_stats: None,
        throughput: None,
    }
}

fn phase_stats_from_json(json: &HashMap<String, PhaseStatsJson>) -> HashMap<String, PhaseStats> {
    json.iter()
        .map(|(name, stats)| {
            (name.clone(), PhaseStats {
                avg_duration: Duration::from_secs_f64(stats.avg_ms / 1000.0),
                min_duration: Duration::from_secs_f64(stats.min_ms / 1000.0),
                max_duration: Duration::from_secs_f64(stats.max_ms / 1000.0),
                avg_percentage: stats.percentage,
            })
        })
        .collect()
}

fn aggregate_decoding_stats(results: &[&BenchmarkResult]) -> Option<AggregatedDecodingStats> {
    let stats: Vec<&DecodingStats> = results.iter()
        .filter_map(|r| r.reconstruct_metrics.as_ref())
        .filter_map(|m| m.decoding_stats.as_ref())
        .collect();
    
    if stats.is_empty() {
        return None;
    }
    
    let total_rows = stats[0].total_rows;
    let avg_successful = stats.iter().map(|s| s.successful_rows as f64).sum::<f64>() / stats.len() as f64;
    let avg_failed = stats.iter().map(|s| s.failed_rows as f64).sum::<f64>() / stats.len() as f64;
    let avg_iterations = stats.iter().map(|s| s.avg_iterations).sum::<f64>() / stats.len() as f64;
    let avg_max_hit = stats.iter().map(|s| s.max_iterations_hit as f64).sum::<f64>() / stats.len() as f64;
    let avg_success_rate = stats.iter().map(|s| s.success_rate()).sum::<f64>() / stats.len() as f64;
    
    Some(AggregatedDecodingStats {
        total_rows,
        avg_successful_rows: avg_successful,
        avg_failed_rows: avg_failed,
        avg_iterations,
        avg_max_iterations_hit: avg_max_hit,
        avg_success_rate,
    })
}

fn calculate_throughput(params: &BenchmarkParams, total_stats: &BenchmarkStats) -> ThroughputMetrics {
    let total_time_secs = total_stats.avg.as_secs_f64();
    
    let info_bits = match params.ldpc_info_size {
        AR4JAInfoSize::K1024 => 1024,
        AR4JAInfoSize::K4096 => 4096,
        AR4JAInfoSize::K16384 => 16384,
    }; 
    
    let shares_per_second = if total_time_secs > 0.0 {
        params.c_value as f64 / total_time_secs
    } else {
        0.0
    };
    
    let bits_per_second = if total_time_secs > 0.0 {
        (info_bits as f64 * 256.0) / total_time_secs
    } else {
        0.0
    }; 
    
    ThroughputMetrics {
        shares_per_second,
        bits_per_second,
    }
}

pub fn print_benchmark_results(summary: &BenchmarkSummary, show_detail: bool) {
    log_info!("--- Benchmark Results ---");
    for (params, stats) in &summary.total_stats {
        log_info!("{} c={} {:?}: avg={}, success={:.0}%", 
            params.implementation, 
            params.c_value, 
            params.decoder_type,
            format_duration_ms(stats.avg),
            stats.success_rate * 100.0);
    }
    
    if show_detail {
        log_verbose!("");
        log_verbose!("=== TOTAL EXECUTION TIME ===");
        for (params, stats) in &summary.total_stats {
            log_verbose!("[{}] c={} {:?}", params.implementation, params.c_value, params.decoder_type);
            log_verbose!("  Avg: {}  Min: {}  Max: {}", 
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max));
            log_verbose!("  Median: {}  StdDev: {}  Success: {:.0}%", 
                format_duration_ms(stats.median),
                format_duration_ms(stats.std_dev),
                stats.success_rate * 100.0);
        }
        
        log_verbose!("");
        log_verbose!("=== SETUP TIME ===");
        for (params, stats) in &summary.setup_stats {
            log_verbose!("[{}] c={} {:?}", params.implementation, params.c_value, params.decoder_type);
            log_verbose!("  Avg: {}  Min: {}  Max: {}", 
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max));
        }
        
        log_verbose!("");
        log_verbose!("=== DEAL TIME ===");
        for (params, stats) in &summary.deal_stats {
            log_verbose!("[{}] c={} {:?}", params.implementation, params.c_value, params.decoder_type);
            log_verbose!("  Avg: {}  Min: {}  Max: {}  Median: {}", 
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max),
                format_duration_ms(stats.median));
                
            if let Some(phase_metrics) = &stats.phase_metrics {
                let mut phases: Vec<(&String, &PhaseStats)> = phase_metrics.iter().collect();
                phases.sort_by(|(_, a), (_, b)| 
                    b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());
                
                for (name, phase_stat) in phases {
                    log_verbose!("    {} - {} ({:.1}%)", 
                        name,
                        format_duration_ms(phase_stat.avg_duration),
                        phase_stat.avg_percentage);
                }
            }
        }
        
        log_verbose!("");
        log_verbose!("=== RECONSTRUCT TIME ===");
        for (params, stats) in &summary.reconstruct_stats {
            log_verbose!("[{}] c={} {:?}", params.implementation, params.c_value, params.decoder_type);
            log_verbose!("  Avg: {}  Min: {}  Max: {}  Median: {}", 
                format_duration_ms(stats.avg),
                format_duration_ms(stats.min),
                format_duration_ms(stats.max),
                format_duration_ms(stats.median));
                
            if let Some(phase_metrics) = &stats.phase_metrics {
                let mut phases: Vec<(&String, &PhaseStats)> = phase_metrics.iter().collect();
                phases.sort_by(|(_, a), (_, b)| 
                    b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());
                
                for (name, phase_stat) in phases {
                    log_verbose!("    {} - {} ({:.1}%)", 
                        name,
                        format_duration_ms(phase_stat.avg_duration),
                        phase_stat.avg_percentage);
                }
            }
        }
    }
}

pub fn run_comprehensive_benchmark<F: PrimeField<BigInt = BigInt<4>> + Debug>(
    c_values: &[usize],
    shares_to_remove_values: &[isize],
    decoder_types: &[DecoderImplementation],
    ldpc_rates: &[AR4JARate],
    ldpc_info_sizes: &[AR4JAInfoSize],
    implementations: &[Implementation],
    runs_per_config: usize,
    warmup_runs: usize,
    show_detail: bool,
    output_file: Option<&str>,
    secret_value: u128,
    max_iterations: usize,
    llr_value: f64,
) {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    log_info!("Starting comprehensive benchmark at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    let params = generate_benchmark_params(
        c_values,
        shares_to_remove_values,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
        secret_value,
        max_iterations,
        llr_value,
    );
    
    log_info!("Will run {} parameter combinations with {} runs each ({} total runs, {} warmup each)",
        params.len(),
        runs_per_config,
        params.len() * runs_per_config,
        warmup_runs);
    
    let multi_progress = Arc::new(MultiProgress::with_draw_target(ProgressDrawTarget::hidden()));
    let mp = Arc::clone(&multi_progress);
    
    let mut all_results = Vec::new();
    
    for param in params {
        if warmup_runs > 0 {
            log_verbose!("Running {} warmup iteration(s) for {} c={}", warmup_runs, param.implementation, param.c_value);
            for _ in 0..warmup_runs {
                let _ = run_single_benchmark::<F>(&param, None);
            }
        }
        
        let results = run_multiple_benchmarks::<F>(&param, runs_per_config, &mp);
        all_results.extend(results);
    }
    
    let summary = calculate_stats(&all_results);
    print_benchmark_results(&summary, show_detail);
    
    if let Some(file_path) = output_file {
        let output_path = if file_path.is_empty() {
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
        
        if let Err(e) = save_benchmark_results_to_json(&summary, &all_results, warmup_runs, &output_path) {
            log_error!("Error saving benchmark results to JSON: {}", e);
        }
        
        if let Err(e) = save_benchmark_results_to_csv(&summary, &output_path) {
            log_error!("Error saving benchmark results to CSV: {}", e);
        }
    }
    
    log_success!("\nBenchmark completed at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
}

pub fn run_comprehensive_benchmark_for_ui<F: PrimeField<BigInt = BigInt<4>> + Debug>(
    c_values: &[usize],
    shares_to_remove_values: &[isize],
    decoder_types: &[DecoderImplementation],
    ldpc_rates: &[AR4JARate],
    ldpc_info_sizes: &[AR4JAInfoSize],
    implementations: &[Implementation],
    runs_per_config: usize,
    show_detail: bool,
    output_file: Option<&str>,
    status_callback: impl Fn(String),
    secret_value: u128,
    max_iterations: usize,
    llr_value: f64,
    cancel_flag: Arc<AtomicBool>,
) -> BenchmarkSummary {
    let processed_shares: Vec<isize> = shares_to_remove_values.iter().cloned().collect();
    
    let params = generate_benchmark_params(
        c_values,
        &processed_shares,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
        secret_value,
        max_iterations,
        llr_value,
    );
    
    status_callback(format!("Starting benchmark with {} parameter combinations", params.len()));
    
    let multi_progress = Arc::new(MultiProgress::with_draw_target(ProgressDrawTarget::hidden()));
    let mp = Arc::clone(&multi_progress);
    
    let mut all_results = Vec::new();
    let mut was_cancelled = false;
    
    for (i, param) in params.iter().enumerate() {
        if cancel_flag.load(Ordering::SeqCst) {
            log_warning!("Benchmark stopped by user after {} configurations", i);
            status_callback(format!("Benchmark stopped after {} configurations", i));
            was_cancelled = true;
            break;
        }
        
        status_callback(format!(
            "Running config {}/{}: {} (c={}, rate={:?}, info_size={:?}, decoder={:?})",
            i + 1, 
            params.len(),
            param.implementation,
            param.c_value,
            param.ldpc_rate,
            param.ldpc_info_size,
            param.decoder_type
        ));
        
        let results = run_multiple_benchmarks::<F>(param, runs_per_config, &mp);
        all_results.extend(results);
    }
    
    if was_cancelled {
        status_callback("Generating partial results...".to_string());
    } else {
        status_callback("Generating summary statistics...".to_string());
    }
    let summary = calculate_stats(&all_results);
    
    if show_detail {
        print_benchmark_results(&summary, true);
    }

    if let Some(file_path) = output_file {
        let output_path = if file_path.is_empty() {
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

            let decoder_str = if decoder_types.len() == 1 {
                format!("_{:?}", decoder_types[0])
            } else {
                String::from("_multi_decoder")
            };
            
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            
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
        
        status_callback(format!("Saving results to {}", output_path));

        if let Err(e) = save_benchmark_results_to_json(&summary, &all_results, 0, &output_path) {
            log_error!("Error saving benchmark results to JSON: {}", e);
            status_callback(format!("Error saving results to JSON: {}", e));
        }

        // if let Err(e) = save_benchmark_results_to_csv(&summary, &output_path) {
        //     log_error!("Error saving benchmark results to CSV: {}", e);
        //     status_callback(format!("Error saving results to CSV: {}", e));
        // }
    }
    
    if was_cancelled {
        status_callback("Benchmark stopped by user".to_string());
    } else {
        status_callback("Benchmark complete!".to_string());
    }

    summary
}