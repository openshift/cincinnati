#!/usr/bin/env bash

set -ue

CARGO_CONFIG=".cargo/config.toml"
FILES_TO_COMMIT=(
  "${CARGO_CONFIG}"
  "vendor"
  "rh-manifest.txt"
)

# Don't overwrite any existing config
[ ! -f "${CARGO_CONFIG}" ] || {
  echo WARNING ${CARGO_CONFIG} exists, moving away..
  mv --backup=existing ${CARGO_CONFIG}{,.prev}
}

set -x

# generate the rh-manifest.txt
cargo run --bin rh-manifest-generator

mkdir -p "$(dirname ${CARGO_CONFIG})"
cargo vendor > "${CARGO_CONFIG}"

# some vendored files aren't world readable which has lead to issues in container builds when trying out the workflow
find ./vendor -type f -exec chmod a+r {} \;

# check the a variable as we don't want to commit new files on CI
# it can also be set by developers which want to skip the git operations
if [ "${OPENSHIFT_CI}" != "true" ]; then
  git add "${FILES_TO_COMMIT[@]}"
  git diff --quiet --staged -- "${FILES_TO_COMMIT[@]}" || {
    echo Files changed, committing...
    git commit -m "Update vendored dependencies and rh-manifest.txt" -- "${FILES_TO_COMMIT[@]}"
  }
fi
