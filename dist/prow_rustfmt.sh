#!/usr/bin/env bash

set -e

# Install and run cargofmt. Input: rust version. Defaults to current default if unset
CARGO="cargo"
test -z ${1} || TOOLCHAIN_ARG="--toolchain ${1}"
test -z ${1} || CARGO="rustup run ${1} cargo"
rustup component add rustfmt ${TOOLCHAIN_ARG}
${CARGO} fmt --all -- --check

# Run the test build
cargo build
