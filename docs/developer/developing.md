# Developing Cincinnati

## Overview

The following sections describe how to build and test the project for local development. 


## Prerequisites

* Git
* Rust
* [just](https://github.com/casey/just)

*Note:* We recommend using [Rustup](https://github.com/rust-lang/rustup/blob/master/README.md) for Rust toolchain management. 

## Running it locally

Run below commands in parallel (for e.g. in different terminals at the same time).

```shell
just run-graph-builder
```

```shell
just run-policy-engine
```

Here is the command to get the graph for stable-4.2 amd64 architecture

```shell
just get-graph-pe "stable-4.2" "amd64"
```

## Running tests locally

### Unit tests

```shell
just test
```

### CI tests


```shell
just run-ci-tests
```

### To run test with net-private

These tests need secrets which can not be shared publicly at this point.
We will provide a solution fix this in the future.
Until then, please rely on the CI to run these tests for each pull request.

If you have access to the secrets, the following environment variables need to be set before running the tests.

* CINCINNATI_TEST_CREDENTIALS_PATH
* CINCINNATI_TEST_QUAY_API_TOKEN_PATH
* CINCINNATI_TEST_QUAY_API_TOKEN

#### Example:

```shell
export CINCINNATI_TEST_CREDENTIALS_PATH="$PWD/.ci_credentials/docker.config.json"
export CINCINNATI_TEST_QUAY_API_TOKEN_PATH="$PWD/.ci_credentials/api_access_token"
export CINCINNATI_TEST_QUAY_API_TOKEN="$(cat $PWD/.ci_credentials/api_access_token)"
just test-net-private
```
