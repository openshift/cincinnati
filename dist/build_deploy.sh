#!/usr/bin/env bash

# TODO [vrutkovs]: rework this as Prow CI doesn't use it anymore 

set -ex

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"

DOCKERFILE_BUILD="$ABSOLUTE_PATH/Dockerfile.build/"
ensure_build_container "${DOCKERFILE_BUILD}"

DOCKERFILE_DEPLOY="$ABSOLUTE_PATH/Dockerfile.deploy/Dockerfile"
RELEASE_DIR="${PROJECT_PARENT_DIR}/target/release"
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

docker build -t "${IMAGE}:${IMAGE_TAG}" $RELEASE_OUTPUT_DIR

if [[ -n "$QUAY_USER" && -n "$QUAY_TOKEN" ]]; then
    DOCKER_CONF="$PWD/.docker"
    mkdir -p "$DOCKER_CONF"
    docker --config="$DOCKER_CONF" login -u="$QUAY_USER" -p="$QUAY_TOKEN" quay.io
    docker --config="$DOCKER_CONF" push "${IMAGE}:${IMAGE_TAG}"
fi
