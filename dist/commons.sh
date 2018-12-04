#!/usr/bin/env bash

IMAGE_BUILD="${IMAGE_BUILD:-clux/muslrust:1.30.0-stable}"
PROJECT_PARENT_DIR="${ABSOLUTE_PATH:?need ABSOLUTE_PATH set}/../"

function docker_cargo () {
    docker run -t --rm \
        --user "$UID:$GID" \
        --tmpfs "/tmp/cargo:rw" \
        --env "CARGO_HOME=/tmp/cargo" \
        -v $PROJECT_PARENT_DIR:/volume:Z \
        $IMAGE_BUILD cargo ${@}
}
