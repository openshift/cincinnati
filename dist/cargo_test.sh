#!/usr/bin/env bash

set -e

declare -A cargo_test_flags
cargo_test_flags["cincinnati"]="--features test-net"
cargo_test_flags["commons"]=""
cargo_test_flags["graph-builder"]="--features test-net"
cargo_test_flags["policy-engine"]=""
cargo_test_flags["metadata-helper"]=""
cargo_test_flags["rh-manifest-generator"]=""
cargo_test_flags["prometheus-query"]=""
cargo_test_flags["quay"]="--features test-net"

declare -A cargo_nextest_flags
cargo_nextest_flags["cincinnati"]=""
cargo_nextest_flags["commons"]=""
cargo_nextest_flags["graph-builder"]=""
cargo_nextest_flags["policy-engine"]=""
cargo_nextest_flags["metadata-helper"]=""
cargo_nextest_flags["rh-manifest-generator"]="--no-tests=pass"
cargo_nextest_flags["prometheus-query"]=""
cargo_nextest_flags["quay"]=""

if [[ -n "${CINCINNATI_TEST_CREDENTIALS_PATH}" && -n "${CINCINNATI_TEST_QUAY_API_TOKEN_PATH}" ]]; then
    echo Secrets available, enabling private tests...
    cargo_test_flags["cincinnati"]+=",test-net-private"
    cargo_test_flags["graph-builder"]+=",test-net-private"
    cargo_test_flags["quay"]+=",test-net-private"

    export CINCINNATI_TEST_QUAY_API_TOKEN="$(cat ${CINCINNATI_TEST_QUAY_API_TOKEN_PATH})"
fi

declare -A executors
executors["cargo"]="execute_native"

# Spurious dead code warning
# shellcheck disable=SC2317
function run_tests() {
  set -x
  export ARTIFACT_DIR=${ARTIFACT_DIR:-.}
  export CARGO_TARGET_DIR="$PWD/target"

  RUNNER="nextest"
  if ! cargo nextest --version; then
    echo "Preferred runner nextest not found"
    if [ -n "$CI" ]; then
      echo "Fallback to standard test runner not desirable in CI: exiting"
      exit 1
    else
      echo "Falling back from $RUNNER to standard Rust runner"
      RUNNER="test"
    fi
  fi

  has_failed=false
  for directory in ${!cargo_test_flags[*]}; do
    # intentional, flags need to expand to multiple strings
    # shellcheck disable=SC2086
    if [ "$RUNNER" == "nextest" ]; then
      cargo nextest run                     \
        --profile ci                        \
        ${cargo_test_flags[$directory]}     \
        ${cargo_nextest_flags[$directory]}  \
        --package "${directory}"            \
        || has_failed=true
      cp -r "${CARGO_TARGET_DIR}/nextest/ci/junit.xml" "${ARTIFACT_DIR}/junit_${directory}.xml"
    else
      cargo test ${cargo_test_flags[$directory]} --package "${directory}" || has_failed=true
    fi
  done

  if [ "${has_failed}" == true ]; then
    echo "at least one error has occurred while running tests; check the output for more information"
    exit 1
  fi

  set +x
}

function execute_native() {
  run_tests
}

for executor in "${!executors[@]}"; do
  if type -f $executor; then
    ${executors[${executor}]}
    exit 0
  fi
done

echo error: could not find any of "${executors[@]}" in PATH
exit 1
