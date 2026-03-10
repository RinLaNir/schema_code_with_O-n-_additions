use ark_bls12_381::Fr;
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

mod types;
mod utils;
mod code;
mod aos_core;
mod aos;
mod aos_parallel;
mod benchmark;
mod ui;

use benchmark::{CliConfig, Implementation, run_comprehensive_benchmark};
use crate::types::{all_decoder_types, parse_decoder_type, parse_ldpc_rate, parse_ldpc_info_size};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "benchmark" => {
                run_benchmarks(&args[2..]);
                return;
            }
            "help" | "--help" | "-h" => {
                print_help();
                return;
            }
            "ui" => {}
            _ => {
                println!("Unknown command: {}", args[1]);
                print_help();
                process::exit(1);
            }
        }
    }

    run_ui();
}


fn run_ui() {
    if let Err(e) = ui::launch_ui() {
        eprintln!("Error running UI: {}", e);
        process::exit(1);
    }
}


fn print_help() {
    let bin = env::args().next().unwrap_or_else(|| String::from("schema_code"));
    println!("Usage: {} [COMMAND] [OPTIONS]", bin);
    println!("Commands:");
    println!("  benchmark [OPTIONS]  Run comprehensive benchmarks");
    println!("  ui                   Run graphical user interface");
    println!("  help                 Print this help message");
    println!();
    println!("Benchmark Options:");
    println!("  --runs=N            Number of runs per parameter combination (default: 3)");
    println!("  --warmup=N          Number of warmup runs before measurement (default: 1)");
    println!("  --sequential        Run only sequential implementation");
    println!("  --parallel          Run only parallel implementation");
    println!("  --detail            Show detailed results for each phase");
    println!("  --c=N1,N2,...       Comma-separated list of c values to test");
    println!("  --rates=R1,R2,...   Comma-separated list of rates to test (1_2, 2_3, etc.)");
    println!("  --sizes=S1,S2,...   Comma-separated list of info sizes to test (K1024, etc.)");
    println!("  --decoders=D1,D2,...Comma-separated list of decoder types to test");
    println!("                      (Aminstarf32, Phif64, Tanhf32, etc. or 'all' for all types)");
    println!("  --shares=N1,N2,...   Comma-separated list of shares_to_remove values");
    println!("                      (positive = absolute count, negative = percentage)");
    println!("  --seed=N            Seed for deterministic share removal (reproducible erasure pattern)");
    println!("  --output            Save results to JSON file with auto-generated name");
    println!("  --output=FILE       Save results to JSON file (FILE.json)");
    println!("  --no-cache          Disable setup caching (run setup per each iteration)");
    println!("  --terminal-log      Print log messages to terminal (disabled by default)");
    println!();
    println!("Example:");
    println!("  {} benchmark --runs=5 --warmup=1 --c=10,20 --detail --decoders=Aminstarf32,Phif64 --output", bin);
}

/// Parse command line arguments for benchmarking.
fn parse_benchmark_args(args: &[String]) -> CliConfig {
    let mut c_values = vec![10, 20];
    let mut shares_to_remove_values: Vec<isize> = vec![100]; // remove 100 shares (absolute count); overridden by --shares=
    
    let mut decoder_types = all_decoder_types();
    let mut ldpc_rates = vec![AR4JARate::R1_2, AR4JARate::R4_5];
    let mut ldpc_info_sizes = vec![AR4JAInfoSize::K1024];
    let mut implementations = vec![Implementation::Sequential, Implementation::Parallel];
    let mut runs_per_config = 3;
    let mut warmup_runs = 1;
    let mut show_detail = false;
    let mut output_file = None;
    let mut cache_setup = true;
    let mut secret_value: u128 = 42;
    let mut max_iterations: usize = 500;
    let mut llr_value: f64 = 10.0;
    let mut removal_seed: Option<u64> = None;

    for arg in args {
        if let Some(val) = arg.strip_prefix("--runs=") {
            if let Ok(num) = val.parse::<usize>() {
                runs_per_config = num;
            }
        } else if let Some(val) = arg.strip_prefix("--warmup=") {
            if let Ok(num) = val.parse::<usize>() {
                warmup_runs = num;
            }
        } else if let Some(secret_str) = arg.strip_prefix("--secret=") {
            if secret_str.starts_with("random") {
                if let Some(seed_str) = secret_str.strip_prefix("random:") {
                    if let Ok(seed) = seed_str.parse::<u64>() {
                        let mut hasher = DefaultHasher::new();
                        seed.hash(&mut hasher);
                        secret_value = hasher.finish() as u128;
                    }
                } else {
                    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                    secret_value = duration.as_nanos();
                }
            } else if secret_str.starts_with("0x") {
                if let Ok(val) = u128::from_str_radix(secret_str.trim_start_matches("0x"), 16) {
                    secret_value = val;
                }
            } else if let Ok(val) = secret_str.parse::<u128>() {
                secret_value = val;
            }
        } else if let Some(val) = arg.strip_prefix("--max-iterations=") {
            if let Ok(iter) = val.parse::<usize>() {
                max_iterations = iter;
            }
        } else if let Some(val) = arg.strip_prefix("--llr=") {
            if let Ok(llr) = val.parse::<f64>() {
                llr_value = llr;
            }
        } else if arg == "--detail" {
            show_detail = true;
        } else if arg == "--sequential" {
            implementations = vec![Implementation::Sequential];
        } else if arg == "--parallel" {
            implementations = vec![Implementation::Parallel];
        } else if arg == "--output" {
            output_file = Some(String::new());
        } else if let Some(val) = arg.strip_prefix("--output=") {
            output_file = Some(val.to_string());
        } else if let Some(val) = arg.strip_prefix("--c=") {
            let parsed: Vec<usize> = val.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            c_values = if parsed.is_empty() { vec![10, 20] } else { parsed };
        } else if let Some(val) = arg.strip_prefix("--rates=") {
            let parsed: Vec<_> = val.split(',').filter_map(|s| parse_ldpc_rate(s.trim()).ok()).collect();
            ldpc_rates = if parsed.is_empty() { vec![AR4JARate::R1_2, AR4JARate::R4_5] } else { parsed };
        } else if let Some(val) = arg.strip_prefix("--sizes=") {
            let parsed: Vec<_> = val.split(',').filter_map(|s| parse_ldpc_info_size(s.trim()).ok()).collect();
            ldpc_info_sizes = if parsed.is_empty() { vec![AR4JAInfoSize::K1024] } else { parsed };
        } else if let Some(val) = arg.strip_prefix("--shares=") {
            let parsed: Vec<isize> = val.split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if !parsed.is_empty() {
                shares_to_remove_values = parsed;
            }
        } else if arg == "--terminal-log" {
            crate::ui::logging::set_terminal_log(true); // applies global logger state directly
        } else if arg == "--no-cache" {
            cache_setup = false;
        } else if let Some(val) = arg.strip_prefix("--decoders=") {
            if val.trim() != "all" {
                let specified: Vec<_> = val.split(',').filter_map(|s| parse_decoder_type(s.trim()).ok()).collect();
                if !specified.is_empty() {
                    decoder_types = specified;
                }
            }
        } else if let Some(val) = arg.strip_prefix("--seed=") {
            if let Ok(seed) = val.parse::<u64>() {
                removal_seed = Some(seed);
            }
        }
    }

    CliConfig {
        c_values,
        shares_to_remove_values,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
        runs_per_config,
        warmup_runs,
        cache_setup,
        show_detail,
        output_file,
        secret_value,
        max_iterations,
        llr_value,
        removal_seed,
    }
}

    
fn run_benchmarks(args: &[String]) {
    let cfg = parse_benchmark_args(args);

    run_comprehensive_benchmark::<Fr>(
        &cfg.c_values,
        &cfg.shares_to_remove_values,
        &cfg.decoder_types,
        &cfg.ldpc_rates,
        &cfg.ldpc_info_sizes,
        &cfg.implementations,
        cfg.runs_per_config,
        cfg.warmup_runs,
        cfg.cache_setup,
        cfg.show_detail,
        cfg.output_file.as_deref(),
        cfg.secret_value,
        cfg.max_iterations,
        cfg.llr_value,
        cfg.removal_seed,
    );
}