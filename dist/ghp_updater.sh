#!/usr/bin/env bash

set -e

ABSOLUTE_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${ABSOLUTE_PATH}/commons.sh"
DOC_OUTPUT_DIR="${PROJECT_PARENT_DIR}/docs"
GH_REPO="shiywang/cincinnati"
FULL_REPO="git@github.com:${GH_REPO}.git"


if [[ -z "${CINCINNATI_GITHUB_PUSH_TOKEN_PATH}" ]]; then
	echo 'No CINCINNATI_GITHUB_PUSH_TOKEN_PATH found, do not push to the remote repo'
else
	ssh-add ${CINCINNATI_GITHUB_PUSH_TOKEN_PATH}
	cargo doc --no-deps --all --all-features --target-dir=${DOC_OUTPUT_DIR} --release
	echo '<meta http-equiv=refresh content=0;url=doc/cincinnati/index.html>' > ${DOC_OUTPUT_DIR}/index.html
	# commit and push changes
	git add -A
	git commit -m "GH-Pages update by openshift-merge-robot"
	git push origin master
fi
