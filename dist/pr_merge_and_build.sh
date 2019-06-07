#!/usr/bin/env bash
#
# This script tries to merge the current branch with master and then runs dist/build_deploy.sh
#

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rev_to_merge="$(git rev-parse --abbrev-ref HEAD)"
tmp_branch="${rev_to_merge}_with_master"

function cleanup() {
  set +e -x
  git checkout "${rev_to_merge}"
  git branch -D "${tmp_branch}"
}

git checkout -B "${tmp_branch}" master
trap cleanup EXIT

git merge --no-edit "${rev_to_merge}"

unset QUAY_USER
unset QUAY_TOKEN

"${ABSOLUTE_PATH}"/build_deploy.sh
