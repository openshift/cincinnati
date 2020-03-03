#!/usr/bin/env bash
#
# This script is used to load test Cincinnati instance.
# It uses vegeta - `go get -u github.com/tsenart/vegeta`

# PE_URL=$(oc -n cincinnati-e2e get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
# export GRAPH_URL="http://${PE_URL}/api/upgrades_info/v1/graph"


mkdir /tmp/results

# duration has to be larger than Prometheus collection time to ensure metrics are collected
duration=30s

for workers in 10 20 30 40 50; do
  for rate in 20 40 60 80 100; do
    file="/tmp/results/rate-${rate}-workers-${workers}.bin"
    echo "Testing workers ${workers}, rate ${rate} -> ${file}"
    sed "s,GRAPH_URL,${GRAPH_URL},g" vegeta.targets | \
      vegeta attack -format http -workers=${workers} -rate=${rate} -duration ${duration} > ${file}
    vegeta report -type=text ${file}
  done
done

vegeta report -type='hist[0,50ms,100ms,500ms,1s,5s,10s]' /tmp/results/*.bin

sleep infinity
