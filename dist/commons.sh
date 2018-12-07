#!/usr/bin/env bash

IMAGE_BUILD="${IMAGE_BUILD:-local/muslrust:stable_custom}"
PROJECT_PARENT_DIR="${ABSOLUTE_PATH:?need ABSOLUTE_PATH set}/../"
GIT_REV="$(git rev-parse --short=7 HEAD)"
BUILD_VOLUME="build_${GIT_REV}"
if ! 2>&1 > /dev/null docker volume inspect "${BUILD_VOLUME}"; then
    docker volume create "${BUILD_VOLUME}" -d local --opt type=tmpfs --opt=device=tmpfs --opt o=uid=$UID
fi

function docker_cargo () {
    docker run -t --rm \
        --user "$UID" \
        --env "HOME=/root" \
        -v "${BUILD_VOLUME}":/root/.cargo/registry:z \
        -v $PROJECT_PARENT_DIR:/volume:Z \
        $IMAGE_BUILD cargo ${@}
}

function ensure_build_container() {
    docker build -t "${IMAGE_BUILD}" dist/build/
}