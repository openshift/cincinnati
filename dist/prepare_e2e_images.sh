#!/usr/bin/env bash

set -xeuo pipefail

skopeo sync --src docker --dest docker --src-no-creds --dest-authfile "${CINCINNATI_CI_PUBLIC_DOCKERJSON_PATH}" \
  quay.io/openshift-release-dev/ocp-release \
  registry.ci.openshift.org/cincinnati-ci-public
