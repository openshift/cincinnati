#!/usr/bin/env bash

IMAGE_BUILD="${IMAGE_BUILD:-quay.io/app-sre/cincinnati:builder}"
PROJECT_PARENT_DIR="${ABSOLUTE_PATH:?need ABSOLUTE_PATH set}/../"
GIT_REV="$(git rev-parse --short=7 HEAD)"
BUILD_VOLUME_CARGO_GIT="build_cargo_git_${GIT_REV}"
BUILD_VOLUME_CARGO_REGISTRY="build_cargo_registry_${GIT_REV}"
if ! 2>&1 > /dev/null docker volume inspect "${BUILD_VOLUME_GIT}"; then
    docker volume create "${BUILD_VOLUME_CARGO_GIT}" -d local --opt type=tmpfs --opt=device=tmpfs --opt o=uid=$UID
fi
if ! 2>&1 > /dev/null docker volume inspect "${BUILD_VOLUME_REGISTRY}"; then
    docker volume create "${BUILD_VOLUME_CARGO_REGISTRY}" -d local --opt type=tmpfs --opt=device=tmpfs --opt o=uid=$UID
fi

IMAGE="${IMAGE:-quay.io/app-sre/cincinnati}"
IMAGE_TAG="${IMAGE_TAG:-${GIT_REV}}"

DOCKER_CARGO_LABEL="docker-cargo"
function docker_cargo () {
    docker run -t --rm \
        --label "${DOCKER_CARGO_LABEL}" \
        --user "$UID" \
        --env "HOME=/root" \
        -v "${BUILD_VOLUME_CARGO_GIT}":/root/.cargo/git:Z \
        -v "${BUILD_VOLUME_CARGO_REGISTRY}":/root/.cargo/registry:Z \
        -v $PROJECT_PARENT_DIR:/volume:Z \
        --workdir /volume \
        $IMAGE_BUILD "${@}"
}

function docker_cargo_stop_all() {
    containers=$(docker ps --quiet --filter "label=${DOCKER_CARGO_LABEL}")
    if test "${containers}"; then
        docker rm -f ${containers} 2>&1 >/dev/null
    fi
}

function ensure_build_container() {
    docker build -t "${IMAGE_BUILD}" "${1:?need Dockerfile for the builder}"
}
