#!/usr/bin/env bash
#
# Script to run all upstream PR checks in one go, stopping on the first failure.
#

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="${ABSOLUTE_PATH}/../dist/"

"${DIST_DIR}/cargo_test.sh"
"${DIST_DIR}/prow_yaml_lint.sh"