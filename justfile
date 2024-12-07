alias t := test
alias b := bench
alias bd := bench-divan
alias bl := bench-launchers
alias d := develop
alias e := example
alias f := format
alias l := lint

# COMMANDS -----------------------------------------------------------------------------------------

# List commands
default:
    @just --list

# Format
format:
    cargo +nightly fmt

lint:
    cargo +nightly clippy --all -- -D warnings 

# Build
build: format
    cargo build --release

# Test
test: format
    cargo test --all

# Benchmark
bench BENCH=("main"):
    cargo bench {{ BENCH }}

# Benchmark - per launcher
bench-launchers: (bench "per_launcher")

# Benchmark - divan
bench-divan: (bench "divan")

# Run example
example EXAMPLE=("basic"):
    cargo run --example {{ EXAMPLE }}

# Run test suite whenever any change is made
develop:
    cargo watch -s "just test"
