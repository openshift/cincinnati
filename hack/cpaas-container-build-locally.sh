#!/usr/bin/env bash
set -ex

DOCKERFILE="dist/Dockerfile.cpaas/Dockerfile"
TAG="$(git rev-parse --short HEAD)"

sed -i -e 's,FROM ,FROM registry.redhat.io/,' "${DOCKERFILE}"
trap "sed -i -e 's,FROM registry.redhat.io/,FROM ,' "${DOCKERFILE}"" EXIT

docker build . -t cincinnati-"${TAG}"
