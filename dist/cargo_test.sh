#!/usr/bin/env bash

set -e

declare -A cargo_test_flags
cargo_test_flags["cincinnati"]="--features test-net"
cargo_test_flags["commons"]=""
cargo_test_flags["graph-builder"]="--features test-net"
cargo_test_flags["policy-engine"]=""
cargo_test_flags["quay"]="--features test-net"

if [[ -n "${CINCINNATI_TEST_CREDENTIALS_PATH}" && -n "${CINCINNATI_TEST_QUAY_API_TOKEN_PATH}" ]]; then
    echo Secrets available, enabling private tests...
    cargo_test_flags["cincinnati"]+=",test-net-private"
    cargo_test_flags["graph-builder"]+=",test-net-private"
    cargo_test_flags["quay"]+=",test-net-private"

    export CINCINNATI_TEST_QUAY_API_TOKEN="$(cat ${CINCINNATI_TEST_QUAY_API_TOKEN_PATH})"
fi

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
  DOCKERFILE_BUILD="$ABSOLUTE_PATH/Dockerfile.build/"

  ensure_build_container "${DOCKERFILE_BUILD}"

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
