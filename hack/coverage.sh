#!/usr/bin/env bash
#
# This script runs the testsuite and collects the code coverage report

### Global variables

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"


### function declaration

function check_deps() {
  declare -A deps
  declare all_deps_available=true

  deps["type -f kcov"]="Please install kcov as instructed by https://github.com/SimonKagstrom/kcov/blob/master/INSTALL.md"
  deps["cargo kcov --version"]="Please install cargo-kcov using 'cargo install cargo-kcov'"

  echo Checking dependencies..

  for dep in "${!deps[@]}"; do
    ${dep} || {
      echo "${deps[${dep}]}"
      all_deps_available=false
    }
  done

  if [[ ${all_deps_available} == false ]]; then exit 1; fi

  echo All dependencies available!
}

function build_tests() {
  # we to clean all tests up because cargo-kcov will pick any left-voer test binary up
  cargo clean

  # we want to prevent completely untested function to be stripped
  RUSTFLAGS='-C link-dead-code' \
    CARGO_ARGS="build --tests" "${ABSOLUTE_PATH}"/../dist/cargo_test.sh
}

function run_coverage() {
	cargo kcov --verbose --all --no-clean-rebuild --open
}

function main() {
  check_deps
  build_tests
  run_coverage
}


### Main
set -ex
main