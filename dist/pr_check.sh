#!/usr/bin/env bash

set -ex

declare -A cargo_test_flags
cargo_test_flags["cincinnati"]=""
cargo_test_flags["graph-builder"]="--features test-net"
cargo_test_flags["policy-engine"]=""
cargo_test_flags["quay"]="--features test-net"

declare -A executors
executors["cargo"]="execute_native"
executors["docker"]="execute_docker"

function run_tests() {
  for directory in ${!cargo_test_flags[*]}; do
    (${1} /usr/bin/env bash -c "\
      cd ${directory} && \
      export CARGO_TARGET_DIR="../target" && \
      cargo test --release ${cargo_test_flags[${directory}]} && \
      :
    ")
  done
}

function execute_native() {
  run_tests
}

function execute_docker() {
  ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  source "${ABSOLUTE_PATH}/commons.sh"

  ensure_build_container

  function cleanup() {
      set +e
      docker_cargo_stop_all
      if [[ ! -n "$KEEP_CARGO_OUTPUT" ]]; then
          docker_cargo cargo clean --release
      fi
  }
  trap cleanup EXIT

  run_tests "docker_cargo"
}

for executor in ${!executors[@]}; do
  if type -f $executor; then
    ${executors[${executor}]}
    exit 0
  fi
done

echo error: could not find any of "${executors[@]}" in PATH
exit 1
