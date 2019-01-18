# Cincinnati

Cincinnati is an update protocol designed to facilitate automatic updates. It describes a particular method for representing transitions between releases of a project and allowing a client to perform automatic updates between these releases.

## Quick Start

Prepare custom environment variables

```console
# Please change these accordingly
export CINCINNATI_REGISTRY="https://quay.io"
export CINCINNATI_REPO="redhat/openshift-cincinnati-test-public-manual"
```

### Executables on the build host

```console
cargo run --package graph-builder -- --address 0.0.0.0 --registry "${CINCINNATI_REGISTRY}" --repository "${CINCINNATI_REPO}" &
cargo run --package policy-engine -- --address 0.0.0.0 &
curl --verbose --header 'Accept:application/json' http://localhost:8081/v1/graph
```

## Tests
There are several ways of testing various parts of the Cincinnati stack.

### Offline

#### Language-Level
The language-level tests can be run using `cargo --test` in the repository's root directory:

```console
cargo test
```

### Online
The online tests for the graph-builder depend on a curated set of repositories to be available on *quay.io* in the *redhat* organization.
The build instructions for (re-)populating the repositories are available at *graph-builder/tests/images/build-n-push.sh*.
The script must run be run from its directory to function:

```console
cd graph-builder/tests/images
./build-n-push.sh test-*
```

#### Language-Level
The graph-builder package currently has network dependent tests which gated behind the feature `test-net` and `test-net-private`.
The latter requires setting the environment variable `CINCINNATI_TEST_CREDENTIALS_PATH` which is equivalent to *graph-builder's* `--credentials-path`.

Assuming you have access to images under the *quay.io/redhat* organization, and have an appropriate *$HOME/.docker/config.json* in place, this might work on your machine:

```console
cd graph-builder
export CINCINNATI_TEST_CREDENTIALS_PATH="$HOME/.docker/config.json"
cargo test --features test-net,test-net-private
```

### CI/CD
The *dist/* directory contains various CI/CD related files.

#### Openshift Dev
* Uses *dist/openshift-release/Dockerfile.builder* as the build container
* Runs `dist/pr_check.sh` for PRs
* Runs `dist/pr_check.sh` for successful merges to the *master* branch

For details please see [github.com/openshift/release/(...)/openshift-cincinnati-master.yaml][1].

#### App-SRE
* Uses *dist/build/Dockerfile* as a build container
* Runs `dist/pr_check.sh` for PRs
* Runs `dist/build_deploy.sh` for successful merges to the *master* branch and pushes the result to the staging environment *(URL is not yet publicly available)*

[1]: https://github.com/openshift/release/blob/master/ci-operator/config/openshift/cincinnati/openshift-cincinnati-master.yaml

