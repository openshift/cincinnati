#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_BUILD="${IMAGE_BUILD:-ekidd/rust-musl-builder:1.30.1}"
PROJECT_PARENT_DIR=$ABSOLUTE_PATH/../

docker run --rm -v $PROJECT_PARENT_DIR:/home/rust/src $IMAGE_BUILD cargo test
