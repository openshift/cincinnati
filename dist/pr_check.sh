#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"

ensure_build_container

function cleanup() {
    set +e
    docker_cargo_stop_all
    if [[ ! -n "$KEEP_CARGO_OUTPUT" ]]; then
        docker_cargo clean
    fi
}
trap cleanup EXIT

docker_cargo test
