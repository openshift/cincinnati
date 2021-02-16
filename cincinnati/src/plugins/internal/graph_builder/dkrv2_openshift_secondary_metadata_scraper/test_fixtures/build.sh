#!/usr/bin/env bash
set -xu
export REPO_BASE="registry.ci.openshift.org/cincinnati-ci-public/cincinnati"
export AUTHFILE="${CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH}"
../../../../../../../graph-builder/tests/images/build-n-push-buildah.sh graph-data-6420f7fbf3724e1e5e329ae8d1e2985973f60c14
