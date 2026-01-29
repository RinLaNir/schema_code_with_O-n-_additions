use ark_bls12_381::Fr;
use ark_ff::PrimeField;
use chrono::Local;
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::env;
use std::process;
use std::time::Instant;

mod types;
mod code;
mod aos_core;
mod aos;
mod aos_parallel;
mod benchmark;
mod ui;

use benchmark::{Implementation, run_comprehensive_benchmark};
use crate::types::{CodeInitParams, Share};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    
    if args.len() > 1 {
        match args[1].as_str() {
            "benchmark" => {
                run_benchmarks(&args[2..]);
                return;
            }
            "cli" => {
                run_single_test();
                return;
            }
            "help" | "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                if args[1] != "ui" {
                    println!("Unknown command: {}", args[1]);
                    print_help();
                    process::exit(1);
                }
            }
        }
    }
    
    
    run_ui();
}

    
fn run_ui() {
    match ui::launch_ui() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Error running UI: {}", e);
            process::exit(1);
        }
    }
}

    
fn print_help() {
    println!("Usage: {} [COMMAND] [OPTIONS]", env::args().next().unwrap_or_else(|| String::from("schema_code")));
    println!("Commands:");
    println!("  benchmark [OPTIONS]  Run comprehensive benchmarks");
    println!("  cli                  Run single test via CLI");
    println!("  ui                   Run graphical user interface");
    println!("  help                 Print this help message");
    println!("");
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
    println!("  --output            Save results to JSON file with auto-generated name");
    println!("  --output=FILE       Save results to JSON file (FILE.json)");
    println!("");
    println!("Example:");
    println!("  {} benchmark --runs=5 --warmup=1 --c=10,20 --detail --decoders=Aminstarf32,Phif64 --output", env::args().next().unwrap_or_else(|| String::from("schema_code")));
}

/// Parse command line arguments for benchmarking
fn parse_benchmark_args(args: &[String]) -> (
    Vec<usize>,                    // c_values
    Vec<isize>,                    // shares_to_remove_values
    Vec<DecoderImplementation>,    // decoder_types
    Vec<AR4JARate>,                // ldpc_rates
    Vec<AR4JAInfoSize>,            // ldpc_info_sizes
    Vec<Implementation>,           // implementations
    usize,                         // runs_per_config
    usize,                         // warmup_runs
    bool,                          // show_detail
    Option<String>,                // output_file
    u128,                          // secret_value
    usize,                         // max_iterations
    f64,                           // llr_value
) {
    
    let mut c_values = vec![10, 20];
    let shares_to_remove_values = vec![100];
    
    let mut decoder_types = vec![
        DecoderImplementation::Phif64,
        DecoderImplementation::Phif32,
        DecoderImplementation::Tanhf64,
        DecoderImplementation::Tanhf32,
        DecoderImplementation::Minstarapproxf64,
        DecoderImplementation::Minstarapproxf32,
        DecoderImplementation::Minstarapproxi8,
        DecoderImplementation::Minstarapproxi8Jones,
        DecoderImplementation::Minstarapproxi8PartialHardLimit,
        DecoderImplementation::Minstarapproxi8JonesPartialHardLimit,
        DecoderImplementation::Minstarapproxi8Deg1Clip,
        DecoderImplementation::Minstarapproxi8JonesDeg1Clip,
        DecoderImplementation::Minstarapproxi8PartialHardLimitDeg1Clip,
        DecoderImplementation::Minstarapproxi8JonesPartialHardLimitDeg1Clip,
        DecoderImplementation::Aminstarf64,
        DecoderImplementation::Aminstarf32,
        DecoderImplementation::Aminstari8,
        DecoderImplementation::Aminstari8Jones,
        DecoderImplementation::Aminstari8PartialHardLimit,
        DecoderImplementation::Aminstari8JonesPartialHardLimit,
        DecoderImplementation::Aminstari8Deg1Clip,
        DecoderImplementation::Aminstari8JonesDeg1Clip,
        DecoderImplementation::Aminstari8PartialHardLimitDeg1Clip,
        DecoderImplementation::Aminstari8JonesPartialHardLimitDeg1Clip,
        DecoderImplementation::HLPhif64,
        DecoderImplementation::HLPhif32,
        DecoderImplementation::HLTanhf64,
        DecoderImplementation::HLTanhf32,
        DecoderImplementation::HLMinstarapproxf64,
        DecoderImplementation::HLMinstarapproxf32,
        DecoderImplementation::HLMinstarapproxi8,
        DecoderImplementation::HLMinstarapproxi8PartialHardLimit,
        DecoderImplementation::HLAminstarf64,
        DecoderImplementation::HLAminstarf32,
        DecoderImplementation::HLAminstari8,
        DecoderImplementation::HLAminstari8PartialHardLimit,
    ];
    let mut ldpc_rates = vec![AR4JARate::R1_2, AR4JARate::R4_5];
    let mut ldpc_info_sizes = vec![AR4JAInfoSize::K1024];
    let mut implementations = vec![Implementation::Sequential, Implementation::Parallel];
    let mut runs_per_config = 3;
    let mut warmup_runs = 1;
    let mut show_detail = false;
    let mut output_file = None;
    let mut secret_value: u128 = 42;
    let mut max_iterations: usize = 500;
    let mut llr_value: f64 = 10.0;

    for arg in args {
        if arg.starts_with("--runs=") {
            if let Some(num_str) = arg.strip_prefix("--runs=") {
                if let Ok(num) = num_str.parse::<usize>() {
                    runs_per_config = num;
                }
            }
        } else if arg.starts_with("--warmup=") {
            if let Some(num_str) = arg.strip_prefix("--warmup=") {
                if let Ok(num) = num_str.parse::<usize>() {
                    warmup_runs = num;
                }
            }
        } else if arg.starts_with("--secret=") {
            if let Some(secret_str) = arg.strip_prefix("--secret=") {
                if secret_str.starts_with("random") {
                    
                    if let Some(seed_str) = secret_str.strip_prefix("random:") {
                        if let Ok(seed) = seed_str.parse::<u64>() {
                            use std::collections::hash_map::DefaultHasher;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = DefaultHasher::new();
                            seed.hash(&mut hasher);
                            secret_value = hasher.finish() as u128;
                        }
                    } else {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                        secret_value = duration.as_nanos() as u128;
                    }
                } else if secret_str.starts_with("0x") {
                    
                    if let Ok(val) = u128::from_str_radix(secret_str.trim_start_matches("0x"), 16) {
                        secret_value = val;
                    }
                } else if let Ok(val) = secret_str.parse::<u128>() {
                    secret_value = val;
                }
            }
        } else if arg.starts_with("--max-iterations=") {
            if let Some(iter_str) = arg.strip_prefix("--max-iterations=") {
                if let Ok(iter) = iter_str.parse::<usize>() {
                    max_iterations = iter;
                }
            }
        } else if arg.starts_with("--llr=") {
            if let Some(llr_str) = arg.strip_prefix("--llr=") {
                if let Ok(llr) = llr_str.parse::<f64>() {
                    llr_value = llr;
                }
            }
        } else if arg == "--detail" {
            show_detail = true;
        } else if arg == "--sequential" {
            implementations = vec![Implementation::Sequential];
        } else if arg == "--parallel" {
            implementations = vec![Implementation::Parallel];
        } else if arg == "--output" {
            
            output_file = Some(String::new());
        } else if arg.starts_with("--output=") {
            if let Some(file_path) = arg.strip_prefix("--output=") {
                output_file = Some(file_path.to_string());
            }
        } else if arg.starts_with("--c=") {
            if let Some(values_str) = arg.strip_prefix("--c=") {
                c_values = values_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<usize>().ok())
                    .collect();
                
                if c_values.is_empty() {
                    c_values = vec![10, 20];
                }
            }
        } else if arg.starts_with("--rates=") {
            if let Some(values_str) = arg.strip_prefix("--rates=") {
                ldpc_rates = values_str
                    .split(',')
                    .filter_map(|s| {
                        match s.trim() {
                            "1_2" => Some(AR4JARate::R1_2),
                            "2_3" => Some(AR4JARate::R2_3),
                            "4_5" => Some(AR4JARate::R4_5),
                            _ => None,
                        }
                    })
                    .collect();
                
                if ldpc_rates.is_empty() {
                    ldpc_rates = vec![AR4JARate::R1_2, AR4JARate::R4_5];
                }
            }
        } else if arg.starts_with("--sizes=") {
            if let Some(values_str) = arg.strip_prefix("--sizes=") {
                ldpc_info_sizes = values_str
                    .split(',')
                    .filter_map(|s| {
                        match s.trim() {
                            "K1024" => Some(AR4JAInfoSize::K1024),
                            "K4096" => Some(AR4JAInfoSize::K4096),
                            "K16384" => Some(AR4JAInfoSize::K16384),
                            _ => None,
                        }
                    })
                    .collect();
                
                if ldpc_info_sizes.is_empty() {
                    ldpc_info_sizes = vec![AR4JAInfoSize::K1024];
                }
            }
        } else if arg.starts_with("--decoders=") {
            if let Some(values_str) = arg.strip_prefix("--decoders=") {
                
                if values_str.trim() == "all" {
                    
                } else {
                    
                    let specified_decoders = values_str
                        .split(',')
                        .filter_map(|s| {
                            match s.trim() {
                                "Phif64" => Some(DecoderImplementation::Phif64),
                                "Phif32" => Some(DecoderImplementation::Phif32),
                                "Tanhf64" => Some(DecoderImplementation::Tanhf64),
                                "Tanhf32" => Some(DecoderImplementation::Tanhf32),
                                "Minstarapproxf64" => Some(DecoderImplementation::Minstarapproxf64),
                                "Minstarapproxf32" => Some(DecoderImplementation::Minstarapproxf32),
                                "Minstarapproxi8" => Some(DecoderImplementation::Minstarapproxi8),
                                "Minstarapproxi8Jones" => Some(DecoderImplementation::Minstarapproxi8Jones),
                                "Minstarapproxi8PartialHardLimit" => Some(DecoderImplementation::Minstarapproxi8PartialHardLimit),
                                "Minstarapproxi8JonesPartialHardLimit" => Some(DecoderImplementation::Minstarapproxi8JonesPartialHardLimit),
                                "Minstarapproxi8Deg1Clip" => Some(DecoderImplementation::Minstarapproxi8Deg1Clip),
                                "Minstarapproxi8JonesDeg1Clip" => Some(DecoderImplementation::Minstarapproxi8JonesDeg1Clip),
                                "Minstarapproxi8PartialHardLimitDeg1Clip" => Some(DecoderImplementation::Minstarapproxi8PartialHardLimitDeg1Clip),
                                "Minstarapproxi8JonesPartialHardLimitDeg1Clip" => Some(DecoderImplementation::Minstarapproxi8JonesPartialHardLimitDeg1Clip),
                                "Aminstarf64" => Some(DecoderImplementation::Aminstarf64),
                                "Aminstarf32" => Some(DecoderImplementation::Aminstarf32),
                                "Aminstari8" => Some(DecoderImplementation::Aminstari8),
                                "Aminstari8Jones" => Some(DecoderImplementation::Aminstari8Jones),
                                "Aminstari8PartialHardLimit" => Some(DecoderImplementation::Aminstari8PartialHardLimit),
                                "Aminstari8JonesPartialHardLimit" => Some(DecoderImplementation::Aminstari8JonesPartialHardLimit),
                                "Aminstari8Deg1Clip" => Some(DecoderImplementation::Aminstari8Deg1Clip),
                                "Aminstari8JonesDeg1Clip" => Some(DecoderImplementation::Aminstari8JonesDeg1Clip),
                                "Aminstari8PartialHardLimitDeg1Clip" => Some(DecoderImplementation::Aminstari8PartialHardLimitDeg1Clip),
                                "Aminstari8JonesPartialHardLimitDeg1Clip" => Some(DecoderImplementation::Aminstari8JonesPartialHardLimitDeg1Clip),
                                "HLPhif64" => Some(DecoderImplementation::HLPhif64),
                                "HLPhif32" => Some(DecoderImplementation::HLPhif32),
                                "HLTanhf64" => Some(DecoderImplementation::HLTanhf64),
                                "HLTanhf32" => Some(DecoderImplementation::HLTanhf32),
                                "HLMinstarapproxf64" => Some(DecoderImplementation::HLMinstarapproxf64),
                                "HLMinstarapproxf32" => Some(DecoderImplementation::HLMinstarapproxf32),
                                "HLMinstarapproxi8" => Some(DecoderImplementation::HLMinstarapproxi8),
                                "HLMinstarapproxi8PartialHardLimit" => Some(DecoderImplementation::HLMinstarapproxi8PartialHardLimit),
                                "HLAminstarf64" => Some(DecoderImplementation::HLAminstarf64),
                                "HLAminstarf32" => Some(DecoderImplementation::HLAminstarf32),
                                "HLAminstari8" => Some(DecoderImplementation::HLAminstari8),
                                "HLAminstari8PartialHardLimit" => Some(DecoderImplementation::HLAminstari8PartialHardLimit),
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>();
                    
                    
                    if !specified_decoders.is_empty() {
                        decoder_types = specified_decoders;
                    }
                }
            }
        }
    }

    (
        c_values,
        shares_to_remove_values,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
        runs_per_config,
        warmup_runs,
        show_detail,
        output_file,
        secret_value,
        max_iterations,
        llr_value,
    )
}

    
fn run_benchmarks(args: &[String]) {
    let (
        c_values,
        shares_to_remove_values,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
        runs_per_config,
        warmup_runs,
        show_detail,
        output_file,
        secret_value,
        max_iterations,
        llr_value,
    ) = parse_benchmark_args(args);

    run_comprehensive_benchmark::<Fr>(
        &c_values,
        &shares_to_remove_values,
        &decoder_types,
        &ldpc_rates,
        &ldpc_info_sizes,
        &implementations,
        runs_per_config,
        warmup_runs,
        show_detail,
        output_file.as_deref(),
        secret_value,
        max_iterations,
        llr_value,
    );
}

    
fn run_single_test() {
    println!("Starting secret sharing scheme at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    
    let use_parallel = false;  
    
    let secret = Fr::from(42u128); 
    let c = 10;
    let code_params = CodeInitParams {
        decoder_type: Some(DecoderImplementation::Aminstarf32),
        ldpc_rate: Some(AR4JARate::R4_5),
        ldpc_info_size: Some(AR4JAInfoSize::K1024),
        max_iterations: Some(500),
        llr_value: Some(1.5),
    };

    let (setup_duration, deal_duration, reconstruct_duration, reconstructed_secret) = 
        if use_parallel {
            let setup_start = Instant::now();
            let pp = aos_parallel::setup::<Fr>(code_params, c);
            let setup_duration = setup_start.elapsed();
            let deal_start = Instant::now();
            let mut shares = aos_parallel::deal(&pp, secret);
            let deal_duration = deal_start.elapsed();
            println!("Total shares: {}", shares.shares.len());
            let shares_to_remove = 100;
            println!("Removing {} random shares...", shares_to_remove);
            remove_random_shares(&mut shares.shares, shares_to_remove);
            println!("Remaining shares: {}", shares.shares.len());
            let reconstruct_start = Instant::now();
            let (reconstructed_value, _) = aos_parallel::reconstruct(&pp, &shares);
            let reconstruct_duration = reconstruct_start.elapsed();
            (setup_duration, deal_duration, reconstruct_duration, reconstructed_value)
        } else {
            let setup_start = Instant::now();
            let pp = aos::setup::<Fr>(code_params, c);
            let setup_duration = setup_start.elapsed();
            let deal_start = Instant::now();
            let mut shares = aos::deal(&pp, secret);
            let deal_duration = deal_start.elapsed();
            println!("Total shares: {}", shares.shares.len());
            let shares_to_remove = 100;
            println!("Removing {} random shares...", shares_to_remove);
            remove_random_shares(&mut shares.shares, shares_to_remove);
            println!("Remaining shares: {}", shares.shares.len());
            let reconstruct_start = Instant::now();
            let (reconstructed_value, _) = aos::reconstruct(&pp, &shares);
            let reconstruct_duration = reconstruct_start.elapsed();
            (setup_duration, deal_duration, reconstruct_duration, reconstructed_value)
        };

    println!("Original Secret: {:?}", secret.into_bigint());
    println!("Reconstructed Secret: {:?}", reconstructed_secret.into_bigint());
    
    if secret == reconstructed_secret {
        println!("✅ Secret reconstructed successfully!");
    } else {
        println!("❌ Secret reconstruction failed!");
    }
    
    
    let total_time = setup_duration + deal_duration + reconstruct_duration;
    println!("\n--- Performance Summary ---");
    println!("Setup: {:.2?} ({:.2}%)", 
             setup_duration, (setup_duration.as_secs_f64() / total_time.as_secs_f64()) * 100.0);
    println!("Deal: {:.2?} ({:.2}%)", 
             deal_duration, (deal_duration.as_secs_f64() / total_time.as_secs_f64()) * 100.0);
    println!("Reconstruction: {:.2?} ({:.2}%)", 
             reconstruct_duration, (reconstruct_duration.as_secs_f64() / total_time.as_secs_f64()) * 100.0);
    println!("Total execution time: {:.2?}", total_time);
}

    
fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: usize) {
    let mut rng = thread_rng();
    shares.shuffle(&mut rng);
    if num_to_remove <= shares.len() {
        shares.drain(0..num_to_remove);
    }
}