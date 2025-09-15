_list:
    @just --list

# Check project.
check: clippy
    just --unstable --fmt --check
    nixpkgs-fmt --check .
    fd --hidden --extension=md --extension=yml --exec-batch prettier --check
    fd --hidden --extension=toml --exec-batch taplo format --check
    fd --hidden --extension=toml --exec-batch taplo lint
    cargo +nightly fmt -- --check

# Format project.
fmt:
    just --unstable --fmt
    nixpkgs-fmt .
    fd --hidden --extension=md --extension=yml --exec-batch prettier --write
    fd --hidden --extension=toml --exec-batch taplo format
    cargo +nightly fmt

# Lint workspace with Clippy.
clippy:
    cargo clippy --workspace --no-default-features
    cargo clippy --workspace --all-features

# Downgrade dependencies required to testing using MSRV.
downgrade-msrv:
    @ echo "No downgrades currently necessary."

# Test workspace.
test: test-no-coverage build-no-std

# Test workspace (without generating coverage output).
test-no-coverage:
    cargo nextest run --workspace --all-targets --all-features

# Test docs.
test-docs:
    cargo test --doc --workspace --all-features

# Test workspace (without generating coverage output).
build-no-std:
    cargo build --target=thumbv6m-none-eabi --manifest-path=./ensure-no-std/Cargo.toml

# Build rustdoc.
doc:
    rm "$(cargo metadata --format-version=1 | jq -r '.target_directory')/doc/crates.js"
    RUSTDOCFLAGS="--cfg=docsrs -Dwarnings" cargo +nightly doc --workspace --no-deps --all-features
