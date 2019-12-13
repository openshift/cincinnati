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

To run unit tests

```shell
just test
```

To test with net-private

```shell
just test-net-private
```