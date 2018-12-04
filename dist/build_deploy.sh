#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"

IMAGE="quay.io/app-sre/cincinnati"
IMAGE_TAG=$(git rev-parse --short=7 HEAD)
DOCKERFILE_DEPLOY="$ABSOLUTE_PATH/Dockerfile"
RELEASE_DIR="${PROJECT_PARENT_DIR}/target/x86_64-unknown-linux-musl/release"
RELEASE_OUTPUT_DIR="${PROJECT_PARENT_DIR}/release-$(date +'%Y%m%d.%H%M%S')"

function cleanup() {
    set +e
    if [[ ! -n "$KEEP_RELEASE_OUTPUT" ]]; then
        rm -f ${RELEASE_OUTPUT_DIR}/{graph-builder,policy-engine}
        rmdir ${RELEASE_OUTPUT_DIR}
    fi
    docker_cargo clean
}
trap cleanup EXIT

docker_cargo build --release
mkdir $RELEASE_OUTPUT_DIR
cp ${RELEASE_DIR}/{graph-builder,policy-engine} $RELEASE_OUTPUT_DIR/

docker build -f $DOCKERFILE_DEPLOY -t "${IMAGE}:${IMAGE_TAG}" $RELEASE_OUTPUT_DIR

if [[ -n "$QUAY_USER" && -n "$QUAY_TOKEN" ]]; then
    DOCKER_CONF="$PWD/.docker"
    mkdir -p "$DOCKER_CONF"
    docker --config="$DOCKER_CONF" login -u="$QUAY_USER" -p="$QUAY_TOKEN" quay.io
    docker --config="$DOCKER_CONF" push "${IMAGE}:${IMAGE_TAG}"
fi
