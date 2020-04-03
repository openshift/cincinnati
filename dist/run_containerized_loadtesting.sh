#!/usr/bin/env bash

# Pull vegeta container, mount load testing script and run it against GRAPH_URL
# This is a wrapper around hack/load-testing.sh which we run after stage deployment is complete

set -ex

VEGETA_IMAGE="docker.io/peterevans/vegeta:6.8"

docker run --rm \
  --volume hack/load-testing.sh:/usr/local/bin/load-testing.sh \
  --volume hack/vegeta.targets:/tmp/vegeta.targets \
  --env GRAPH_URL=${GRAPH_URL} \
  --workdir /tmp \
  --entrypoint=/usr/local/bin/load-testing.sh \
  -ti "${VEGETA_IMAGE}"
