# Retromount development commands

build:
    cargo build

release:
    cargo build --release

clean:
    cargo clean

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

run:
    cargo run

run-config CONFIG="retromount.yaml":
    cargo run -- --config {{CONFIG}}

debug:
    cargo run

deb:
    cargo deb

release-deb:
    cargo build --release
    cargo deb

ci:
    just fmt-check
    just lint
    just test
    cargo build --release