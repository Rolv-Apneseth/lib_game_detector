alias t := test
alias b := bench
alias bd := bench-divan
alias bl := bench-launchers
alias d := develop
alias dr := develop-readme
alias e := example
alias f := format
alias l := lint
alias p := publish

# COMMANDS -----------------------------------------------------------------------------------------

# List commands
default:
    @just --list

# Format
format:
    cargo +nightly fmt

# Lint
lint:
    cargo clippy --all -- -D warnings 

# Build
build: format
    cargo build --release

# Doc
doc:
    cargo doc

# MSRV
msrv:
    cargo msrv verify

# Test
test: format doc msrv
    cargo test --all

# Publish
publish: test
    cargo publish

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
    bacon test

# Re-generate the README whenever a change is made to `lib.rs`
develop-readme:
    bacon rdme
