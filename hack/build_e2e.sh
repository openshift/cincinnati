#!/usr/bin/env bash

set -euo pipefail

type -f jq || {
  echo ERROR: jq is not available.
  exit 1
}

set -ex
mkdir -p /opt/cincinnati/bin

cd e2e
mapfile -t tests < <(
  RUST_BACKTRACE=full cargo build --tests --features test-e2e-prom-query --verbose --message-format=json |
    jq -r 'select(.profile.test == true) | .executable'
  )

for f in ${tests[@]}; do
  cp -rvf ${f} /opt/cincinnati/bin/
done

for f in /opt/cincinnati/bin/*; do
  mv ${f} ${f%-*}
done
