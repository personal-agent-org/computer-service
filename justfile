# Personal Agent Computer Service.
build:
    cargo build --release -p computer-service

fmt:
    cargo fmt

lint:
    cargo clippy --workspace --all-targets

check: fmt lint
    cargo test --workspace
    cargo build --release -p computer-service
