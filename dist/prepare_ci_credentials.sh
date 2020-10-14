#!/usr/bin/env bash
#
# This script takes the raw CI credentials and transforms them into a docker.json compatible file.

set -xeuo pipefail

CINCINNATI_CI_DOCKERCFG_PATH="${CINCINNATI_CI_DOCKERCFG_PATH:-/dev/null}"
CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH="${CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH:-/dev/null}"

if test -f "${CINCINNATI_CI_DOCKERCFG_PATH}"; then
    # consolidate the quay credentials with the private CI registry credentials to allow a seamless transition to the latter
    jq -s '{ "auths": .[0] } * .[1]' "${CINCINNATI_CI_DOCKERCFG_PATH}" "${CINCINNATI_TEST_CREDENTIALS_PATH}" > "${CINCINNATI_CI_DOCKERJSON_PATH}"
fi

if test -f "${CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH}"; then
    jq <"${CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH}" '{ "auths": . }' > "${CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH}"
fi
