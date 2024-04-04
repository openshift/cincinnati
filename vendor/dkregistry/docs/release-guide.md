# Release process

This project uses [cargo-release][cargo-release] in order to prepare new releases, to tag and sign relevant git commit and to publish the resulting artifacts to [crates.io][crates-io].

The release process follows the usual PR-and-review flow, allowing an external reviewer to have a final check before publishing.

This document gives an high-level overview as well as a step-by-step guide on how to perform a release.

## Overview

Most of the process is automated with the help of cargo-release and its metadata entries in Cargo manifest.
This helper is in charge of bumping the `version` field in the manifest in a dedicated commit, attaching the correspoding signed tag to it, and then producing another commit to prepare the project for the next development cycle.

The two resulting commits can be then submitted in a dedicated PR and reviewed.
Once merged, the last steps of the process consist in pushing the git tag and publishing the crate.

## Steps

If this is the first time, make sure all the requirements from the section below are met.

This guide assumes that you have push access to the upstream repository of the project.
Since this is required to push the new tag we have decided to also place the PR branch on the upstream repository.

These steps show how to release version `x.y.z` on the `upstream` remote (this can be checked via `git remote -av`).

For each release to be published, proceed as follows:

#### 1. Make sure the project is clean and prepare the environment

* `cargo test`
* `cargo clean`
* `git clean -fd`
* `export RELEASE_VER=x.y.z`
* `export UPSTREAM_REMOTE=upstream`

#### 2. Create release commits on a dedicated branch and tag it

* `git checkout -b release-${RELEASE_VER}`
* This will create the tag after asking for version confirmation:

  `cargo release`

#### 3. Open a PR for this release

* `git push ${UPSTREAM_REMOTE} release-${RELEASE_VER}`
* Open a web browser and create a Pull Request for the branch above
* Make sure the resulting PR contains exactly two commits

#### 4. Get the PR reviewed, approved and merged

#### 5. Publish the artifacts (tag and crate)

* `git push ${UPSTREAM_REMOTE} ${RELEASE_VER}`
* Make sure the upstream tag matches the local tag:

    `git fetch --tags --verbose ${UPSTREAM_REMOTE} 2>&1 | grep ${RELEASE_VER}`
* `git checkout ${RELEASE_VER}`
* Make sure the tag is what you intend to release; if so this will show an empty output:

    `git diff release-${RELEASE_VER}~1 ${RELEASE_VER}`
* `cargo publish`

#### 6. Clean up the environment

* `unset RELEASE_VER`
* `unset UPSTREAM_REMOTE`
* `cargo clean`

## Requirements

This guide requires:

 * a web browser (and network connectivity)
 * `git`
 * GPG setup and personal key for signing
 * `cargo` (suggested: latest stable toolchain from [rustup][rustup])
 * `cargo-release` (suggested: `cargo install -f cargo-release`)

[cargo-release]: https://github.com/sunng87/cargo-release
[rustup]: https://rustup.rs/
[crates-io]: https://crates.io/
