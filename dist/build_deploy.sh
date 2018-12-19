#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"

ensure_build_container

DOCKERFILE_DEPLOY="$ABSOLUTE_PATH/Dockerfile"
RELEASE_DIR="${PROJECT_PARENT_DIR}/target/x86_64-unknown-linux-musl/release"
RELEASE_OUTPUT_DIR="${PROJECT_PARENT_DIR}/release-$(date +'%Y%m%d.%H%M%S')"

function cleanup() {
    set +e
    if [[ ! -n "$KEEP_CARGO_OUTPUT" ]]; then
        docker_cargo cargo clean
    fi
    docker_cargo_stop_all
    if [[ ! -n "$KEEP_RELEASE_OUTPUT" ]]; then
        rm -f ${RELEASE_OUTPUT_DIR}/{graph-builder,policy-engine}
        rm -f ${RELEASE_OUTPUT_DIR}/$(basename ${DOCKERFILE_DEPLOY})
        rmdir ${RELEASE_OUTPUT_DIR}
    fi
}
trap cleanup EXIT

docker_cargo cargo build --release
mkdir $RELEASE_OUTPUT_DIR
cp ${RELEASE_DIR}/{graph-builder,policy-engine} $DOCKERFILE_DEPLOY  $RELEASE_OUTPUT_DIR/

docker build -t "${IMAGE}:${IMAGE_TAG}" $RELEASE_OUTPUT_DIR

if [[ -n "$QUAY_USER" && -n "$QUAY_TOKEN" ]]; then
    DOCKER_CONF="$PWD/.docker"
    mkdir -p "$DOCKER_CONF"
    docker --config="$DOCKER_CONF" login -u="$QUAY_USER" -p="$QUAY_TOKEN" quay.io
    docker --config="$DOCKER_CONF" push "${IMAGE}:${IMAGE_TAG}"
fi
