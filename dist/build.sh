#!/usr/bin/env bash
#
# Script to test the build the image without pushing it.
#

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

unset QUAY_USER
unset QUAY_TOKEN

"${ABSOLUTE_PATH}/build_deploy.sh"
