# Retromount development commands
#
# Run `just --list` to see all available commands.
#
# These recipes provide a consistent developer workflow and mirror the
# commands used in CI so contributors can easily run the same checks locally.

# Ensure Windows uses PowerShell as the execution shell.
# Linux/macOS will continue using their default shell.
set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

# ------------------------------------------------
# Build tasks
# ------------------------------------------------

# Build the project in debug mode
build:
    cargo build --all-targets --all-features --verbose

# Build the project in release mode
release:
    cargo build --release --all-targets --all-features --verbose

# Remove build artifacts
clean:
    cargo clean


# ------------------------------------------------
# Code quality
# ------------------------------------------------

# Apply automatic formatting fixes
fix:
    cargo fmt

# Format code
fmt:
    cargo fmt

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints and fail on warnings
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
    cargo test --all-targets --all-features --verbose

# Run all local quality checks (recommended before committing)
check:
    just fmt-check
    just lint
    just test


# ------------------------------------------------
# Running Retromount
# ------------------------------------------------

# Run the application
run:
    cargo run

# Run with a specific configuration file
run-config CONFIG="retromount.yaml":
    cargo run -- --config {{CONFIG}}

# Run with debug logging enabled
debug:
    RUST_LOG=debug cargo run

# Run with very verbose logging
trace:
    RUST_LOG=trace cargo run


# ------------------------------------------------
# Packaging
# ------------------------------------------------

# Build a Debian package (requires cargo-deb)
deb:
    cargo deb

# Build release binary and Debian package
release-deb:
    just release
    cargo deb --no-build


# ------------------------------------------------
# CI parity
# ------------------------------------------------

# Run the same checks that CI performs
ci:
    just check
    just build
    just release
    