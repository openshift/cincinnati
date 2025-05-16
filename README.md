# Cincinnati

Cincinnati is an update protocol designed to facilitate automatic updates. It describes a particular method for representing transitions between releases of a project and allowing a client to perform automatic updates between these releases.

## Quick Start

Prepare custom environment variables

```console
# Please change these accordingly
export CINCINNATI_REGISTRY="https://quay.io"
export CINCINNATI_REPO="openshift-ota/openshift-cincinnati-test-public-manual"
```

See the details about [pushing images to the above repo](./docs/developer/push-to-openshift-cincinnati-test-public-manual.md).

### Executables on the build host

```console
cargo run --package graph-builder -- --service.address 0.0.0.0 --upstream.registry.url "${CINCINNATI_REGISTRY}" --upstream.registry.repository "${CINCINNATI_REPO}" &
cargo run --package policy-engine -- --service.address 0.0.0.0 &
 curl -s 'Accept:application/json' http://localhost:8081/graph\?channel\=candidate-4.18 | jq
{
  "version": 1,
  "nodes": [
    {
      "version": "4.18.3",
      "payload": "quay.io/openshift-ota/openshift-cincinnati-test-public-manual@sha256:fdcb3da3a1086d664df31a1fa2a629c77780f844d458af956928cca297da343c",
      "metadata": {
        "io.openshift.upgrades.graph.release.manifestref": "sha256:fdcb3da3a1086d664df31a1fa2a629c77780f844d458af956928cca297da343c",
        "io.openshift.upgrades.graph.release.channels": "candidate-4.18,eus-4.18,fast-4.18,stable-4.18,candidate-4.19",
        "io.openshift.upgrades.graph.previous.remove_regex": ".*|4[.]17[.].*|4[.](17[.](1[01]|0-.*|[0-9])|18.0-(ec[.].*|rc[.][0-3]))",
        "url": "https://access.redhat.com/errata/RHBA-2025:2229"
      }
    }
  ],
  "edges": [],
  "conditionalEdges": []
}
```

***Note:*** the default configuration of the policy-engine requires the `channel` parameter to be present in each request.

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
* Uses *dist/Dockerfile.build/Dockerfile* as the build container image
* Run the following scripts on PR
    * `dist/prow_yaml_lint.sh`
    * `dist/prow_rustfmt.sh`
    * `dist/cargo_test.sh`

For details please see [github.com/openshift/release/(...)/openshift-cincinnati-master.yaml][1].

#### App-SRE
* Uses *dist/Dockerfile.build/Dockerfile* as the build container image
* Runs `dist/build_deploy.sh` for successful merges to the *master* branch and pushes the result to the staging environment *(URL is not yet publicly available)*


## Development

For developing Cincinnati refer to [the developer documentation](./docs/developer/developing.md) document.

### Updating the Plugin-Interface Scheme
The interface for external plugins is defined as a Protobuf v3 scheme in the file [cincinnati/src/plugins/interface.proto][./cincinnati/src/plugins/interface.proto].
In order to regenerate the files the *cincinnati* crate must be built with the `codegen-protoc` feature:

```console
cd cincinnati
cargo build --features=codegen-protoc
```

The CI/CD system doesn't do this and it relies on the generated code being committed to the repository; please do so after generating new code!

[1]: https://github.com/openshift/release/blob/master/ci-operator/config/openshift/cincinnati/openshift-cincinnati-master.yaml
