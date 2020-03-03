#!/usr/bin/env bash
#
# This script is used to load test Cincinnati instance.
# It uses vegeta - `go get -u github.com/tsenart/vegeta`

PE_URL=$(oc -n cincinnati-e2e get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
export GRAPH_URL="http://${PE_URL}/api/upgrades_info/v1/graph"

mkdir results

# duration has to be larger than Prometheus collection time to ensure metrics are collected
duration=30s

for workers in $(seq 5 5 50); do
  for rate in $(seq 50 50 500); do
    echo "Testing workers ${workers}, rate ${rate}"
    sed "s,GRAPH_URL,${GRAPH_URL},g" vegeta.targets | \
      vegeta attack -format http -workers=${workers} -rate=${rate} -duration ${duration} > results/ rate-${rate}-workers-${workers}.bin
  done
done

vegeta report -type='hist[0,50ms,100ms,500ms,1s,5s,10s]' results/*.bin
