#!/usr/bin/env sh

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

set -xueo pipefail

# generate the rh-manifest.txt
cargo run --bin rh-manifest-generator

mkdir -p "$(dirname ${CARGO_CONFIG})"
cargo vendor > "${CARGO_CONFIG}"

# some vendored files aren't world readable which has lead to issues in container builds when trying out the workflow
find ./vendor -type f -exec chmod a+r {} \;


git add --force "${FILES_TO_COMMIT[@]}"
git commit -m "Vendor dependencies and update cargo to use them, and add rh-manifest.txt" -- "${FILES_TO_COMMIT[@]}"
