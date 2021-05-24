#!/usr/bin/env bash

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
