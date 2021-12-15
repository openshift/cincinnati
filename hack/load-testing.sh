#!/usr/bin/env sh
#
# This script is used to load test Cincinnati instance.
# It uses vegeta - `go get -u github.com/tsenart/vegeta`

# PE_URL=$(oc -n cincinnati-e2e get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
# export GRAPH_URL="http://${PE_URL}/api/upgrades_info/graph"


TMP_DIR=$(mktemp -d)

# duration has to be larger than Prometheus collection time to ensure metrics are collected
duration=30s

for workers in 50 100 500 1000; do
  for rate in 1000 2500 5000 7500 10000 15000; do
    file="${TMP_DIR}/rate-${rate}-workers-${workers}.bin"
    echo "Testing workers ${workers}, rate ${rate} -> ${file}"
    sed "s,GRAPH_URL,${GRAPH_URL},g" vegeta.targets | \
      vegeta attack -insecure -format http -workers=${workers} -max-workers=${workers} -rate=${rate} -duration ${duration} > ${file}
    vegeta report -type=text ${file}
    # Sleep here to clear up connections cache in cincinnati
    sleep 30
  done
done

vegeta report -type='hist[0,50ms,100ms,500ms,1s,5s,10s]' ${TMP_DIR}/*.bin
