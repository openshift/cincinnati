#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"

function cleanup() {
    set +e
    docker_cargo clean
}
trap cleanup EXIT

docker_cargo test
