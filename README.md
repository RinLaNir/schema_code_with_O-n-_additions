# Schema Code - LDPC Secret Sharing Scheme

Rust implementation of an Additive Only Secret Sharing (AOS) scheme built on CCSDS AR4JA LDPC codes. The current implementation uses a bit model of `F_{2^ell}` with packed bytes, XOR masking, sequential and parallel execution backends, and a desktop GUI for benchmarking and result inspection.

## What Changed

The scheme no longer uses `ark-ff`, `ark-bls12-381`, or a prime field secret type such as `Fr`.

The core secret type is now `F2PowElement`:

- internal representation: packed little-endian `Vec<u8>`
- external representation: standard big-endian hex string
- operations used by the scheme: random bit generation, bit access, bit mutation, XOR

The old `c` parameter was removed from `setup` and from all CLI/UI benchmark flows. During setup the implementation now samples a binary mask `a <-R F_2^k`, where `k` is the LDPC information length.

## Scheme Overview

Given:

- secret `s in F_{2^ell}`
- LDPC information length `k`
- LDPC codeword length `n`
- binary mask `a = (a_1, ..., a_k) in F_2^k`
- random columns `r_1, ..., r_k in F_{2^ell}`

the deal phase computes:

`z0 = s XOR XOR_{i : a_i = 1} r_i`

Then each `r_i` is viewed as a column of `ell` bits. These columns form a message matrix `M in GF(2)^(ell x k)`. Every row is encoded independently with the LDPC code, yielding an encoded matrix `M_enc in GF(2)^(ell x n)`. Each share is one encoded column together with its index.

Reconstruction decodes every row from the available shares. If at least one row fails to decode, reconstruction returns `None`. Otherwise the original columns `r_i` are rebuilt and the secret is recovered as:

`s = z0 XOR XOR_{i : a_i = 1} r_i`

## Prerequisites

### Install Rust

1. Install Rust with [rustup](https://rustup.rs/).

   Windows (PowerShell):

   ```powershell
   winget install Rustlang.Rustup
   ```

2. Restart the terminal and verify the toolchain:

   ```bash
   rustc --version
   cargo --version
   ```

## Building

```bash
git clone <repository-url>
cd schema_code

# Release build
cargo build --release

# Debug build
cargo build
```

The binary is produced as:

- `target/release/schema_code.exe` on Windows
- `target/release/schema_code` on Linux/macOS

## Running

### GUI Mode

Launch the GUI:

```bash
cargo run --release
```

or explicitly:

```bash
cargo run --release -- ui
```

The GUI includes:

- `Configuration` tab for benchmark parameters, secret length, fixed or random secret input, decoder settings, erasure settings, and output options
- `Results` tab for summaries, phase timing breakdowns, throughput, and decoding statistics
- `Console` tab for live logs
- `About` tab for quick usage notes
- English and Ukrainian localization

### Benchmark CLI

Run the benchmark driver:

```bash
# Default benchmark matrix
cargo run --release -- benchmark

# Fixed 128-bit secret in hex
cargo run --release -- benchmark --secret-bits=128 --secret=0x2a --rates=4_5 --sizes=K1024

# Deterministic random secret and deterministic share removal
cargo run --release -- benchmark --secret-bits=256 --secret=random:7 --shares=100,-10 --seed=42 --output
```

#### Benchmark Options

| Option | Description |
| --- | --- |
| `--runs=N` | Number of measured runs per configuration |
| `--warmup=N` | Number of warmup runs before measurement |
| `--sequential` | Run only the sequential backend |
| `--parallel` | Run only the parallel backend |
| `--detail` | Print phase-level timing details |
| `--rates=R1,R2,...` | LDPC rates: `1_2`, `2_3`, `4_5` |
| `--sizes=S1,S2,...` | LDPC information sizes: `K1024`, `K4096`, `K16384` |
| `--decoders=D1,D2,...` | Decoder implementations, or `all` |
| `--shares=N1,N2,...` | Shares to remove before reconstruction; positive values are absolute counts, negative values are percentages |
| `--seed=N` | Seed for deterministic share removal |
| `--secret-bits=ELL` | Secret bit length `ell` |
| `--secret=HEX` | Secret as hex, with optional `0x` prefix |
| `--secret=random` | Random secret of length `ell` |
| `--secret=random:SEED` | Deterministic random secret |
| `--max-iterations=N` | LDPC decoder iteration limit |
| `--llr=X` | LLR magnitude for known bits during decoding |
| `--output` | Save JSON report with an auto-generated filename |
| `--output=FILE` | Save JSON report to `FILE.json` |
| `--no-cache` | Disable setup caching between benchmark runs |
| `--terminal-log` | Mirror log output to the terminal |

### Help

```bash
cargo run -- help
```

## Secret Representation

`F2PowElement` is the only secret representation used by the public API.

- `bit_len` stores `ell`
- `bytes` stores the value in packed little-endian order
- `to_hex()` returns a fixed-width big-endian hex string
- `from_hex(hex, ell)` validates that the input fits into `ell` bits

Input rules:

- shorter hex values are left-padded with zeros to match the selected bit length
- values that do not fit into `ell` bits are rejected
- random secret generation produces exactly `ell` bits

The setup phase also requires `k >= ell`, where `k` is the LDPC information length chosen by the AR4JA configuration.

## Library Example

```rust
use schema_code::aos;
use schema_code::types::{CodeInitParams, F2PowElement};
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use ldpc_toolbox::decoder::factory::DecoderImplementation;

let params = CodeInitParams {
    decoder_type: Some(DecoderImplementation::Aminstarf32),
    ldpc_rate: Some(AR4JARate::R4_5),
    ldpc_info_size: Some(AR4JAInfoSize::K1024),
    max_iterations: Some(300),
    llr_value: Some(1.3863),
    secret_bits: Some(128),
};

let pp = aos::setup(params);
let secret = F2PowElement::from_hex("0x2a", 128).unwrap();
let shares = aos::deal(&pp, &secret);
let (reconstructed, metrics) = aos::reconstruct(&pp, &shares);

assert_eq!(Some(secret), reconstructed);
assert!(metrics.is_some());
```

The same API shape is available through `schema_code::aos_parallel`.

## Running Tests

```bash
cargo test
```

The tests cover:

- `F2PowElement` parsing, padding, XOR, and bit access
- setup invariants such as `a_bits.len() == k`
- round-trip `deal -> reconstruct` in sequential and parallel modes
- reconstruction with erasures
- failure handling when row decoding does not fully succeed

## Benchmark Output

JSON exports include:

- benchmark metadata
- one entry per configuration
- `secret_hex` and `secret_bits`
- setup, deal, reconstruct, and total timing summaries
- optional phase breakdowns
- optional decoding statistics
- optional throughput and parallel metrics
- individual run data

## Project Structure

```text
src/
  main.rs                 CLI entry point and benchmark argument parsing
  lib.rs                  Library exports
  types.rs                Core types, secret representation, metrics
  utils.rs                Helpers such as share removal
  aos_core/               Shared scheme logic and execution strategy trait
  aos/                    Sequential backend
  aos_parallel/           Parallel backend using Rayon
  code/                   LDPC code abstraction and AR4JA implementation
  benchmark/              Benchmark orchestration, stats, import/export
  ui/                     egui desktop application
tests/
  integration_tests.rs    End-to-end scheme tests
```

## Dependencies

Key dependencies:

- `ldpc-toolbox` for CCSDS AR4JA codes, encoding, and BP decoding
- `sparse-bin-mat` for sparse binary matrix support used by the LDPC stack
- `ndarray` for matrix operations
- `rand` for secret and mask generation
- `rayon` for the parallel backend
- `serde` and `serde_json` for result export/import
- `eframe`, `egui_plot`, `egui_extras`, `rfd` for the GUI

See `Cargo.toml` for the full dependency list.
