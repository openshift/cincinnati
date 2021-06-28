#!/usr/bin/env bash

# TODO [vrutkovs]: rework this as Prow CI doesn't use it anymore 

set -ex

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"

DOCKERFILE_DEPLOY="$ABSOLUTE_PATH/Dockerfile.deploy/Dockerfile"

docker build -t "${IMAGE}:${IMAGE_TAG}" -f $DOCKERFILE_DEPLOY .

if [[ -n "$QUAY_USER" && -n "$QUAY_TOKEN" ]]; then
    DOCKER_CONF="$PWD/.docker"
    mkdir -p "$DOCKER_CONF"
    docker --config="$DOCKER_CONF" login -u="$QUAY_USER" -p="$QUAY_TOKEN" quay.io
    docker --config="$DOCKER_CONF" push "${IMAGE}:${IMAGE_TAG}"
fi
