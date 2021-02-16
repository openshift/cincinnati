#!/usr/bin/env bash

set -xeuo pipefail

pushd ./graph-builder/tests/images/

export REPO_BASE="registry.ci.openshift.org/cincinnati-ci/cincinnati"
AUTHFILE="${CINCINNATI_CI_DOCKERJSON_PATH}" ./build-n-push-buildah.sh ./*private*

export REPO_BASE="registry.ci.openshift.org/cincinnati-ci-public/cincinnati"
AUTHFILE="${CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH}" ./build-n-push-buildah.sh ./*public*
