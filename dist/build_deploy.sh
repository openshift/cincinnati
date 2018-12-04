#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_BUILD="${IMAGE_BUILD:-ekidd/rust-musl-builder:1.30.1}"
IMAGE="quay.io/app-sre/cincinnati"
IMAGE_TAG=$(git rev-parse --short=7 HEAD)
PROJECT_PARENT_DIR=$ABSOLUTE_PATH/../
DOCKERFILE_DEPLOY="$ABSOLUTE_PATH/Dockerfile"

docker run --rm -v $PROJECT_PARENT_DIR:/home/rust/src $IMAGE_BUILD cargo build --release

docker build -f $DOCKERFILE_DEPLOY -t "${IMAGE}:${IMAGE_TAG}" $PROJECT_PARENT_DIR

if [[ -n "$QUAY_USER" && -n "$QUAY_TOKEN" ]]; then
    DOCKER_CONF="$PWD/.docker"
    mkdir -p "$DOCKER_CONF"
    docker --config="$DOCKER_CONF" login -u="$QUAY_USER" -p="$QUAY_TOKEN" quay.io
    docker --config="$DOCKER_CONF" push "${IMAGE}:${IMAGE_TAG}"
fi
