# Cincinnati

Cincinnati is an update protocol designed to facility automatic updates. It describes a particular method for representing transitions between releases of a project and allowing a client to perform automatic updates between these releases.

## Quick Start

```console
cargo run --package graph-builder -- --address 0.0.0.0 --registry https://quay.io --repository <namespace/reponame> &
cargo run --package policy-engine -- --address 0.0.0.0 &
curl --verbose --header 'Accept:application/json' http://localhost:8081/v1/graph
```
