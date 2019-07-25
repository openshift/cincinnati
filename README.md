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
cargo run --package graph-builder -- --service.address 0.0.0.0 --upstream.registry.url "${CINCINNATI_REGISTRY}" --upstream.registry.repository "${CINCINNATI_REPO}" &
cargo run --package policy-engine -- --service.address 0.0.0.0 &
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

### Run CI testsuite locally
The script `hack/run-all-tests.sh` can be used to run the CI tests locally before submitting a pull-request.

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
* Uses *dist/Dockerfile.build/Dockerfile* as the build container image
* Run the following scripts on PR
    * `dist/cargo_test.sh`
    * `dist/prow_yaml_lint.sh`

For details please see [github.com/openshift/release/(...)/openshift-cincinnati-master.yaml][1].

#### App-SRE
* Uses *dist/Dockerfile.build/Dockerfile* as the build container image
* Runs `dist/build_deploy.sh` for successful merges to the *master* branch and pushes the result to the staging environment *(URL is not yet publicly available)*


## Development

### Updating the Plugin-Interface Scheme
The interface for external plugins is defined as a Protobuf v3 scheme in the file [cincinnati/src/plugins/interface.proto][./cincinnati/src/plugins/interface.proto].
In order to regenerate the files the *cincinnati* crate must be built with the `codegen-protoc` feature:

```console
cd cincinnati
cargo build --features=codegen-protoc
```

The CI/CD system doesn't do this and it relies on the generated code being committed to the repository; please do so after generating new code!

[1]: https://github.com/openshift/release/blob/master/ci-operator/config/openshift/cincinnati/openshift-cincinnati-master.yaml
