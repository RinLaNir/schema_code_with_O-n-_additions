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
mod aos;
mod aos_parallel;
mod benchmark;

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
            "help" | "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                println!("Unknown command: {}", args[1]);
                print_help();
                process::exit(1);
            }
        }
    }
    
    // Default to running a single test if no command is provided
    run_single_test();
}

/// Print help information
fn print_help() {
    println!("Usage: {} [COMMAND] [OPTIONS]", env::args().next().unwrap_or_else(|| String::from("schema_code")));
    println!("Commands:");
    println!("  benchmark [OPTIONS]  Run comprehensive benchmarks");
    println!("  help                 Print this help message");
    println!("");
    println!("Benchmark Options:");
    println!("  --runs=N            Number of runs per parameter combination (default: 3)");
    println!("  --sequential        Run only sequential implementation");
    println!("  --parallel          Run only parallel implementation");
    println!("  --detail            Show detailed results for each phase");
    println!("  --c=N1,N2,...       Comma-separated list of c values to test");
    println!("  --rates=R1,R2,...   Comma-separated list of rates to test (1_2, 2_3, etc.)");
    println!("  --sizes=S1,S2,...   Comma-separated list of info sizes to test (K1024, etc.)");
    println!("  --decoders=D1,D2,...Comma-separated list of decoder types to test");
    println!("                      (Aminstarf32, Phif64, Tanhf32, etc. or 'all' for all types)");
    println!("  --output            Save results to CSV files with auto-generated names");
    println!("  --output=FILE       Save results to CSV files (FILE_summary.csv and FILE_phases.csv)");
    println!("");
    println!("Example:");
    println!("  {} benchmark --runs=5 --c=10,20 --detail --decoders=Aminstarf32,Phif64 --output", env::args().next().unwrap_or_else(|| String::from("schema_code")));
}

/// Parse command line arguments for benchmarking
fn parse_benchmark_args(args: &[String]) -> (
    Vec<usize>,                    // c_values
    Vec<usize>,                    // shares_to_remove_values
    Vec<DecoderImplementation>,    // decoder_types
    Vec<AR4JARate>,                // ldpc_rates
    Vec<AR4JAInfoSize>,            // ldpc_info_sizes
    Vec<Implementation>,           // implementations
    usize,                         // runs_per_config
    bool,                          // show_detail
    Option<String>,                // output_file
) {
    // Default values
    let mut c_values = vec![10, 20];
    let shares_to_remove_values = vec![250];
    // By default, use all decoder types if not specified
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
    let mut show_detail = false;
    let mut output_file = None;

    for arg in args {
        if arg.starts_with("--runs=") {
            if let Some(num_str) = arg.strip_prefix("--runs=") {
                if let Ok(num) = num_str.parse::<usize>() {
                    runs_per_config = num;
                }
            }
        } else if arg == "--detail" {
            show_detail = true;
        } else if arg == "--sequential" {
            implementations = vec![Implementation::Sequential];
        } else if arg == "--parallel" {
            implementations = vec![Implementation::Parallel];
        } else if arg == "--output" {
            // Auto-generate output filename (will be handled in the benchmark function)
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
                // Check if "all" was explicitly specified
                if values_str.trim() == "all" {
                    // Keep the default (all decoders)
                } else {
                    // Parse specific decoder types
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
                    
                    // Only update decoder_types if valid decoder types were specified
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
        show_detail,
        output_file,
    )
}

/// Run benchmarks with parsed command line arguments
fn run_benchmarks(args: &[String]) {
    let (
        c_values,
        shares_to_remove_values,
        decoder_types,
        ldpc_rates,
        ldpc_info_sizes,
        implementations,
        runs_per_config,
        show_detail,
        output_file,
    ) = parse_benchmark_args(args);

    run_comprehensive_benchmark::<Fr>(
        &c_values,
        &shares_to_remove_values,
        &decoder_types,
        &ldpc_rates,
        &ldpc_info_sizes,
        &implementations,
        runs_per_config,
        show_detail,
        output_file.as_deref(),
    );
}

/// Run a single test (original functionality)
fn run_single_test() {
    println!("Starting secret sharing scheme at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    // Choose which implementation to use
    let use_parallel = false;  // Set to true to use parallel implementation
    
    let secret = Fr::from(42u128); // Secret as a field element
    let c = 10;
    let code_params = CodeInitParams {
        decoder_type: Some(DecoderImplementation::Aminstarf32),  // Explicit decoder type
        ldpc_rate: Some(AR4JARate::R4_5),           // Explicit rate
        ldpc_info_size: Some(AR4JAInfoSize::K1024), // Explicit info size
        max_iterations: Some(500),                  // Custom max iterations
        llr_value: Some(1.5),                       // Custom LLR value
    };

    let (setup_duration, deal_duration, reconstruct_duration, reconstructed_secret) = 
        if use_parallel {
            // Using parallel implementation
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
            // Using sequential implementation
            let setup_start = Instant::now();
            let mut pp = aos::setup::<Fr>(code_params, c);
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
            let (reconstructed_value, _) = aos::reconstruct(&mut pp, &shares);
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
    
    // Print overall performance summary
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

/// Removes random shares from the vector
fn remove_random_shares(shares: &mut Vec<Share>, num_to_remove: usize) {
    let mut rng = thread_rng();
    shares.shuffle(&mut rng);
    if num_to_remove <= shares.len() {
        shares.drain(0..num_to_remove);
    }
}