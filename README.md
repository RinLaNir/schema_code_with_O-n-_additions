# Schema Code - LDPC Secret Sharing Scheme

A Rust implementation of an Additive Only Secret Sharing (AOS) scheme using CCSDS AR4JA LDPC codes. Features both sequential and parallel implementations with a GUI application for benchmarking and visualization.

## Prerequisites

### Install Rust

1. **Download and install Rust via [rustup](https://rustup.rs/):**

   **Windows (PowerShell):**
   ```powershell
   winget install Rustlang.Rustup
   ```
   Or download the installer directly from https://rustup.rs/

2. **Restart your terminal** and verify the installation:
   ```bash
   rustc --version
   cargo --version
   ```

## Building

```bash
# Clone the repository
git clone <repository-url>
cd schema_code

# Build in release mode (recommended for performance)
cargo build --release

# Or debug mode for development
cargo build
```

The compiled binary will be located at:
- **Release**: `target/release/schema_code.exe` (Windows) or `target/release/schema_code` (Linux/macOS)
- **Debug**: `target/debug/schema_code.exe` (Windows) or `target/debug/schema_code` (Linux/macOS)

## Running the Application

### GUI Mode (Default)

Launch the graphical user interface:

```bash
cargo run --release
```

Or run the compiled executable directly:
```bash
# Windows
.\target\release\schema_code.exe

# Linux/macOS
./target/release/schema_code
```

The GUI provides:
- **Configure Tab**: Set benchmark parameters (C values, decoder types, LDPC rates, implementations)
- **Results Tab**: View benchmark results with summary, phases breakdown, and detailed metrics
- **Console Tab**: Real-time logging output
- **About Tab**: Application information
- **Language Selection**: English and Ukrainian localization

### CLI Single Test Mode

Run a single test with default parameters:

```bash
cargo run --release -- cli
```

### Benchmark Mode

Run comprehensive benchmarks with various configurations:

```bash
# Basic benchmark with default settings
cargo run --release -- benchmark

# Benchmark with custom parameters
cargo run --release -- benchmark --runs=5 --c=10,20 --detail --output

# Full example with all options
cargo run --release -- benchmark --runs=5 --c=10,20 --rates=1_2,4_5 --sizes=K1024 --decoders=Aminstarf32,Phif64 --sequential --output=results
```

#### Benchmark Options

| Option | Description |
|--------|-------------|
| `--runs=N` | Number of runs per configuration (default: 3) |
| `--sequential` | Run only sequential implementation |
| `--parallel` | Run only parallel implementation |
| `--detail` | Show detailed results for each phase |
| `--c=C1,C2,...` | C values to test (e.g., `--c=10,20,50`) |
| `--rates=R1,R2,...` | LDPC rates: `1_2`, `2_3`, `4_5` |
| `--sizes=S1,S2,...` | Info sizes: `K1024`, `K4096`, `K16384` |
| `--decoders=D1,D2,...` | Decoder types or `all` for all available decoders |
| `--output` | Auto-generate timestamped output CSV filename |
| `--output=FILE` | Save results to specific file (creates `FILE_summary.csv` and `FILE_phases.csv`) |

### Help

Display all available commands and options:

```bash
cargo run -- help
```

## Running Tests

Execute the test suite:

```bash
cargo test
```

Tests verify:
- Deal/reconstruct round-trip with no erasures
- Deal/reconstruct with erasures (error correction capability)
- Various secret values
- Both sequential and parallel implementations

## Features

### Core Functionality
- **Secret Sharing**: Split secrets into shares and reconstruct them using LDPC codes
- **Error Correction**: Reconstruct secrets even with missing/erased shares
- **BLS12-381**: Uses cryptographic prime field elements for security

### LDPC Configurations
- **Rates**: 1/2, 2/3, 4/5
- **Info Sizes**: K1024, K4096, K16384
- **36+ Decoder Types**: Phi, Tanh, MinStar, Aminstar variants, and more

### Performance
- **Parallel Processing**: Multi-threaded implementation using Rayon
- **Benchmarking**: Comprehensive timing for setup, deal, and reconstruct phases
- **Statistical Analysis**: Min, max, average, median, standard deviation

### User Interface
- **GUI Application**: Full-featured desktop application using egui
- **Results Visualization**: Charts and tables for benchmark results
- **CSV Export**: Save results for external analysis
- **Localization**: English and Ukrainian language support

## Project Structure

```
src/
├── main.rs           # Entry point, CLI argument parsing
├── lib.rs            # Library exports
├── types.rs          # Core data types (Share, Metrics, etc.)
├── benchmark.rs      # Benchmark framework
├── code/
│   ├── mod.rs        # AdditiveCode trait definition
│   └── ldpc_impl.rs  # CCSDS AR4JA LDPC implementation
├── aos_core/
│   └── mod.rs        # Shared logic, ExecutionStrategy trait
├── aos/
│   ├── mod.rs        # Sequential AOS implementation
│   └── utils.rs      # Sequential utilities
├── aos_parallel/
│   ├── mod.rs        # Parallel AOS implementation (Rayon)
│   └── utils.rs      # Parallel utilities
└── ui/
    ├── mod.rs        # UI module entry point
    ├── app.rs        # Main application state
    ├── localization.rs
    ├── benchmark_config.rs
    ├── logging.rs
    ├── components/   # Reusable UI components
    ├── tabs/         # Tab implementations
    └── results/      # Results display components
```

## Output Files

Benchmark results are saved as CSV files:
- `*_summary.csv`: Aggregated statistics per configuration
- `*_phases.csv`: Detailed phase-level timing data

## Dependencies

Key dependencies include:
- **Cryptography**: `ark-ff`, `ark-bls12-381` (finite field arithmetic)
- **LDPC**: `ldpc-toolbox`, `sparse-bin-mat`
- **Parallelization**: `rayon`
- **GUI**: `eframe`, `egui_plot`, `rfd`
- **Utilities**: `chrono`, `indicatif`, `ndarray`

See `Cargo.toml` for the complete list.
