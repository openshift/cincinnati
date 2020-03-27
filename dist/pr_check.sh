#!/usr/bin/env bash
#
# Script to run all upstream PR checks in one go, stopping on the first failure.
#

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${ABSOLUTE_PATH}/cargo_test.sh"
"${ABSOLUTE_PATH}/prow_yaml_lint.sh"
