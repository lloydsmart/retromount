# Retromount development commands
# Run `just` to list available commands

set shell := ["bash", "-cu"]

# Default task
default:
    just --list

# ------------------------------------------------
# Build tasks
# ------------------------------------------------

build:
    cargo build

release:
    cargo build --release

clean:
    cargo clean

# ------------------------------------------------
# Code quality
# ------------------------------------------------

fmt:
    cargo fmt

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test

check:
    just fmt-check
    just lint
    just test

# ------------------------------------------------
# Run / development
# ------------------------------------------------

run:
    cargo run

run-config CONFIG="retromount.yaml":
    cargo run -- --config {{CONFIG}}

# ------------------------------------------------
# FUSE debugging helpers (future use)
# ------------------------------------------------

# Run with debug logging
debug:
    RUST_LOG=debug cargo run

# Run with trace logging
trace:
    RUST_LOG=trace cargo run

# ------------------------------------------------
# Packaging
# ------------------------------------------------

deb:
    cargo deb

release-deb:
    cargo build --release
    cargo deb

# ------------------------------------------------
# CI parity (run exactly what CI runs)
# ------------------------------------------------

ci:
    just fmt-check
    just lint
    just test
    cargo build --release