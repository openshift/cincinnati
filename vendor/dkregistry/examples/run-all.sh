#!/usr/bin/env sh
set -e
for path in examples/*.rs; do
  file="${path##*/}"
  example="${file%%.*}"
  cargo run --example "$example" || {
    echo Example $example failed.
    exit 1
  }
done
