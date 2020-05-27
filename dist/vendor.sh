#!/usr/bin/env sh

CARGO_CONFIG=".cargo/config.toml"

# Don't overwrite any existing config
[ ! -f "${CARGO_CONFIG}" ] || {
  echo ERROR ${CARGO_CONFIG} exists.
  exit 1
}

set -xueo pipefail

mkdir -p "$(dirname ${CARGO_CONFIG})"
cargo vendor > "${CARGO_CONFIG}"

# some vendored files aren't world readable which has lead to issues in container builds when trying out the workflow
find ./vendor -type f -exec chmod a+r {} \;

git add --force .cargo vendor
git commit -m "Vendor dependencies and update cargo to use them" -- .cargo vendor
