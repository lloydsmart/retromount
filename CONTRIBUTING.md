# Contributing to RetroMount

Thank you for your interest in contributing!

This document describes the basic development workflow used by this
repository.

---

# Development setup

RetroMount uses Rust stable.

The repository includes a `rust-toolchain.toml`, so Rustup will
automatically install the required toolchain.

Clone the repository:

```bash
git clone https://github.com/lloydsmart/retromount
cd retromount
```

Build the project:

```bash
cargo build
```

---

# Running tests

```bash
cargo test
```

---

# Formatting

All code must pass `rustfmt`.

```bash
cargo fmt
```

CI enforces this via:

```
cargo fmt --all -- --check
```

---

# Linting

Clippy warnings are treated as errors.

```bash
cargo clippy --all-targets --all-features
```

---

# CI checks

Pull requests must pass:

- formatting
- clippy
- build
- tests

across the supported CI platforms.

---

# Submitting changes

1. Create a feature branch
2. Ensure CI passes
3. Open a pull request against `develop`

Please include clear commit messages describing the purpose of the
change.