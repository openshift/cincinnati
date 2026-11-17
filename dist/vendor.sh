#!/usr/bin/env bash

set -ue

command -v jq >/dev/null || {
  echo "ERROR: jq is required to prune the vendored lockfiles" >&2
  exit 1
}

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

# Drop the lockfiles that upstream crates ship inside their own tarballs.
# They are those projects' development lockfiles and play no part in building
# cincinnati, but the SBOM scanner of the downstream build reads every Cargo.lock
# in the source tree as a manifest and reports its whole dependency graph as if
# it were shipped in our image. That adds thousands of components we don't build
# and generates CVE trackers against crates we don't use, e.g. OCPBUGS-135334 for
# webpki-roots, which only appears in vendor/{actix-http,h2,hyper-rustls}/Cargo.lock.
# The matching entry has to go from .cargo-checksum.json too, otherwise cargo
# aborts with "failed to open file .../Cargo.lock" when building from vendor/.
while IFS= read -r -d '' lock; do
  crate_dir="./vendor/$(printf '%s' "${lock#./vendor/}" | cut -d/ -f1)"
  checksums="${crate_dir}/.cargo-checksum.json"
  rm -f "${lock}"
  [ -f "${checksums}" ] || continue
  jq --arg f "${lock#"${crate_dir}"/}" '.files |= del(.[$f])' "${checksums}" > "${checksums}.tmp"
  mv "${checksums}.tmp" "${checksums}"
done < <(find ./vendor -mindepth 2 -name Cargo.lock -print0)

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
