#!/usr/bin/env bash
#
# This script helps to set up a local development environment with secrets that are stored on the CI cluster.
# It fetches these secrets and populates local files with them.
# The script needs to be `source`d to work: `source hack/get_ci_secrets.sh`.

CINCINNATI_CI_DOCKERCFG_PATH="$(mktemp)"
CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH="$(mktemp)"
CINCINNATI_CI_DOCKERJSON_PATH=$(mktemp)
CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH=$(mktemp)

if (
    set -xeuo pipefail

    oc get secrets --namespace=cincinnati-ci ci-image-sa-dockercfg-vjdrw -o 'go-template={{index .data ".dockercfg"}}' | base64 -d > "${CINCINNATI_CI_DOCKERCFG_PATH}"
    oc get secrets --namespace=cincinnati-ci-public ci-image-sa-dockercfg-cwj4w -o 'go-template={{index .data ".dockercfg"}}' | base64 -d > "${CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH}"

    export CINCINNATI_CI_DOCKERCFG_PATH
    export CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH
    export CINCINNATI_CI_DOCKERJSON_PATH
    export CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH

    dist/prepare_ci_credentials.sh
  ); then
  export CINCINNATI_CI_DOCKERCFG_PATH
  export CINCINNATI_CI_PUBLIC_DOCKERCFG_PATH
  export CINCINNATI_CI_DOCKERJSON_PATH
  export CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH
  export CINCINNATI_TEST_CREDENTIALS_PATH="${CINCINNATI_CI_DOCKERJSON_PATH}"
else
  echo Failed.
fi
