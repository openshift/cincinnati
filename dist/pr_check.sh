#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_BUILD="${IMAGE_BUILD:-clux/muslrust:1.30.0-stable}"
PROJECT_PARENT_DIR=$ABSOLUTE_PATH/../

docker run -t --rm -v $PROJECT_PARENT_DIR:/volume:Z $IMAGE_BUILD cargo test
