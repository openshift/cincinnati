#!/usr/bin/env bash
#
# This script takes the raw CI credentials and transforms them into a docker.json compatible file.

set -xeuo pipefail

test -f "${CINCINNATI_CI_DOCKERCFG_PATH}"
test -f "${CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH}"

touch "${CINCINNATI_CI_DOCKERJSON_PATH}"
touch "${CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH}"

# consolidate the quay credentials with the private CI registry credentials to allow a seamless transition to the latter
jq -s '{ "auths": .[0] } * .[1]' "${CINCINNATI_CI_DOCKERCFG_PATH}" "${CINCINNATI_TEST_CREDENTIALS_PATH}" > "${CINCINNATI_CI_DOCKERJSON_PATH}"
jq <"${CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH}" '{ "auths": . }' > "${CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH}"
