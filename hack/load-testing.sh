#!/usr/bin/env bash
#
# This script is used to load test Cincinnati instance.
# It uses vegeta - `go get -u github.com/tsenart/vegeta`

PE_URL=$(oc -n cincinnati-e2e get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
export GRAPH_URL="http://${PE_URL}/api/upgrades_info/v1/graph"

for workers in $(seq 1 10); do
  for rate in $(seq 10 1500 10); do
    echo "Testing workers ${workers}, rate ${rate}"
    sed "s,GRAPH_URL,${GRAPH_URL},g" vegeta.targets | vegeta attack -format http -max-workers=${workers} -rate=${rate} > rate-${rate}-workers-${workers}.bin
  done
done
