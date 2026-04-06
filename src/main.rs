use rand::rngs::StdRng;
use rand::SeedableRng;
use std::env;
use std::process;

mod aos;
mod aos_core;
mod aos_parallel;
mod benchmark;
mod code;
mod types;
mod ui;
mod utils;

use crate::types::{
    all_decoder_types, parse_decoder_type, parse_ldpc_info_size, parse_ldpc_rate, F2PowElement,
};
use benchmark::{run_comprehensive_benchmark, CliConfig, Implementation};

enum SecretSpec {
    Hex(String),
    Random(Option<u64>),
}

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
    let bin = env::args()
        .next()
        .unwrap_or_else(|| String::from("schema_code"));
    println!("Usage: {} [COMMAND] [OPTIONS]", bin);
    println!("Commands:");
    println!("  benchmark [OPTIONS]  Run comprehensive benchmarks");
    println!("  ui                   Run graphical user interface");
    println!("  help                 Print this help message");
    println!();
    println!("Benchmark Options:");
    println!("  --runs=N             Number of runs per parameter combination (default: 3)");
    println!("  --warmup=N           Number of warmup runs before measurement (default: 1)");
    println!("  --sequential         Run only sequential implementation");
    println!("  --parallel           Run only parallel implementation");
    println!("  --detail             Show detailed results for each phase");
    println!("  --rates=R1,R2,...    Comma-separated list of rates to test (1_2, 2_3, etc.)");
    println!("  --sizes=S1,S2,...    Comma-separated list of info sizes to test (K1024, etc.)");
    println!("  --decoders=D1,D2,... Comma-separated list of decoder types to test");
    println!("  --shares=N1,N2,...   Comma-separated list of shares_to_remove values");
    println!("                       (positive = absolute count, negative = percentage)");
    println!("  --seed=N             Seed for deterministic share removal");
    println!("  --secret-bits=ELL    Secret length in bits (default: 128)");
    println!("  --secret=HEX         Secret as hex string (accepts optional 0x prefix)");
    println!("  --secret=random      Generate a random secret");
    println!("  --secret=random:SEED Generate a deterministic random secret");
    println!("  --output             Save results to JSON file with auto-generated name");
    println!("  --output=FILE        Save results to JSON file (FILE.json)");
    println!("  --no-cache           Disable setup caching");
    println!("  --terminal-log       Print log messages to terminal");
    println!();
    println!("Example:");
    println!(
        "  {} benchmark --runs=5 --warmup=1 --rates=4_5 --sizes=K1024 --secret-bits=128 --secret=0x2a --detail --output",
        bin
    );
}

fn parse_secret(spec: &SecretSpec, secret_bits: usize) -> Result<F2PowElement, String> {
    match spec {
        SecretSpec::Hex(hex) => F2PowElement::from_hex(hex, secret_bits),
        SecretSpec::Random(Some(seed)) => {
            let mut rng = StdRng::seed_from_u64(*seed);
            Ok(F2PowElement::random(secret_bits, &mut rng))
        }
        SecretSpec::Random(None) => {
            let mut rng = rand::rng();
            Ok(F2PowElement::random(secret_bits, &mut rng))
        }
    }
}

fn parse_benchmark_args(args: &[String]) -> CliConfig {
    let mut shares_to_remove_values: Vec<isize> = vec![100];
    let mut decoder_types = all_decoder_types();
    let mut ldpc_rates = vec![
        crate::types::parse_ldpc_rate("1_2").unwrap(),
        crate::types::parse_ldpc_rate("4_5").unwrap(),
    ];
    let mut ldpc_info_sizes = vec![crate::types::parse_ldpc_info_size("K1024").unwrap()];
    let mut implementations = vec![Implementation::Sequential, Implementation::Parallel];
    let mut runs_per_config = 3;
    let mut warmup_runs = 1;
    let mut show_detail = false;
    let mut output_file = None;
    let mut cache_setup = true;
    let mut secret_bits: usize = 128;
    let mut secret_spec = SecretSpec::Hex(String::from("2a"));
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
            if secret_str.eq_ignore_ascii_case("random") {
                secret_spec = SecretSpec::Random(None);
            } else if let Some(seed_str) = secret_str.strip_prefix("random:") {
                match seed_str.parse::<u64>() {
                    Ok(seed) => secret_spec = SecretSpec::Random(Some(seed)),
                    Err(_) => {
                        eprintln!("Invalid secret seed: {}", seed_str);
                        process::exit(1);
                    }
                }
            } else {
                secret_spec = SecretSpec::Hex(secret_str.to_string());
            }
        } else if let Some(val) = arg.strip_prefix("--secret-bits=") {
            match val.parse::<usize>() {
                Ok(bits) if bits > 0 => secret_bits = bits,
                _ => {
                    eprintln!("Invalid --secret-bits value: {}", val);
                    process::exit(1);
                }
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
        } else if let Some(val) = arg.strip_prefix("--rates=") {
            let parsed: Vec<_> = val
                .split(',')
                .filter_map(|s| parse_ldpc_rate(s.trim()).ok())
                .collect();
            if !parsed.is_empty() {
                ldpc_rates = parsed;
            }
        } else if let Some(val) = arg.strip_prefix("--sizes=") {
            let parsed: Vec<_> = val
                .split(',')
                .filter_map(|s| parse_ldpc_info_size(s.trim()).ok())
                .collect();
            if !parsed.is_empty() {
                ldpc_info_sizes = parsed;
            }
        } else if let Some(val) = arg.strip_prefix("--shares=") {
            let parsed: Vec<isize> = val
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if !parsed.is_empty() {
                shares_to_remove_values = parsed;
            }
        } else if arg == "--terminal-log" {
            crate::ui::logging::set_terminal_log(true);
        } else if arg == "--no-cache" {
            cache_setup = false;
        } else if let Some(val) = arg.strip_prefix("--decoders=") {
            if val.trim() != "all" {
                let specified: Vec<_> = val
                    .split(',')
                    .filter_map(|s| parse_decoder_type(s.trim()).ok())
                    .collect();
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

    let secret = match parse_secret(&secret_spec, secret_bits) {
        Ok(secret) => secret,
        Err(err) => {
            eprintln!("Invalid secret: {}", err);
            process::exit(1);
        }
    };

    CliConfig {
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
        secret,
        max_iterations,
        llr_value,
        removal_seed,
    }
}

fn run_benchmarks(args: &[String]) {
    let cfg = parse_benchmark_args(args);

    run_comprehensive_benchmark(
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
        &cfg.secret,
        cfg.max_iterations,
        cfg.llr_value,
        cfg.removal_seed,
    );
}
