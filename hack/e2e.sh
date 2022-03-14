#!/usr/bin/env bash
#
# This script assumes its running from a pod and does the following:
# * Processes the template to setup a local cincy instance
# * Prepares a graph using test data
# * Ensures generated graph is valid

# Prerequirements:
#   * pull secret (with registry.ci.openshift.org) part in `/tmp/cluster/pull-secret`
#   * CINCINNATI_IMAGE (optional) - image with graph-builder and policy-engine
#   * env var IMAGE_FORMAT (e.g `registry.ci.openshift.org/ci-op-ish8m5dt/stable:${component}`)

# ARGUMENTS
#    -ri --run-ignored    Run the ignored tests only for cincinnati e2e

# parse the input arguments
run_ignored=""
while [[ $# -gt 0 ]]
do
  key="$1"
  case $key in
      -ri|--run-ignored)
      run_ignored="--ignored"
      shift
      ;;
  esac
done

# Use CI image format by default unless CINCINNATI_IMAGE is set
if [[ ! -z "${CINCINNATI_IMAGE}" ]]; then
  IMAGE=$(echo "${CINCINNATI_IMAGE}" | cut -d ':' -f1)
  IMAGE_TAG=$(echo "${CINCINNATI_IMAGE}" | cut -d ':' -f2)
else
  IMAGE="${IMAGE_FORMAT%/*}/stable"
  IMAGE_TAG="deploy"
fi

echo "IMAGE=${IMAGE}"
echo "IMAGE_TAG=${IMAGE_TAG}"

function backoff() {
    local max_attempts=60
    local attempt=0
    local failed=0
    while true; do
        "$@" && failed=0 || failed=1
        if [[ $failed -eq 0 ]]; then
            break
        fi
        attempt=$(( attempt + 1 ))
        if [[ $attempt -gt $max_attempts ]]; then
            break
        fi
        echo "command failed, retrying in 10 seconds"
        sleep 10
    done
    return $failed
}

# Use defined PULL_SECRET or fall back to in-cluster pull secret
if test -z "${PULL_SECRET}"
then
  PULL_SECRET=/tmp/pull_secret
  oc -n openshift-config get --template='{{index .data ".dockerconfigjson" | base64decode}}' secret pull-secret > "${PULL_SECRET}"
  echo "using in-cluster pull-secret"
fi

set -euo pipefail
set -x
# Copy KUBECONFIG so that it can be mutated
cp -Lrvf $KUBECONFIG /tmp/kubeconfig
export KUBECONFIG=/tmp/kubeconfig

# Create a new project
backoff oc create namespace openshift-update-service
backoff oc project openshift-update-service

# Create a dummy secret as a workaround to not having real secrets in e2e
backoff oc create secret generic cincinnati-credentials --from-literal="foo=bar"

# Use this pull secret to fetch images from CI
backoff oc create secret generic ci-pull-secret --from-file=.dockercfg=${PULL_SECRET} --type=kubernetes.io/dockercfg

# Wait for default service account to appear
backoff oc get serviceaccount default
# Allow default serviceaccount to use CI pull secret
backoff oc secrets link default ci-pull-secret --for=pull

# Import observability template
# ServiceMonitors are imported before app deployment to give Prometheus time to catch up with
# metrics
# `oc new-app` would stumble on unknown monitoring.coreos.com/v1 objects, so process and create instead
backoff oc process -f dist/openshift/observability.yaml -p NAMESPACE="openshift-update-service" | oc apply -f -

# Export the e2e test environment variables
E2E_TESTDATA_DIR="${E2E_TESTDATA_DIR:-e2e/tests/testdata}"
export E2E_TESTDATA_DIR
read -r E2E_METADATA_REVISION <"${E2E_TESTDATA_DIR}"/metadata_revision
export E2E_METADATA_REVISION

export GRAPH_SOURCE="${GRAPHDATA_IMAGE:-quay.io/openshift-ota/cincinnati-graph-data:${E2E_METADATA_REVISION}}"

echo "GRAPHDATA_IMAGE=${GRAPH_SOURCE}"

# Render the template and apply subscription/operand
oc process -f dist/openshift/cincinnati-e2e.yaml \
  -p IMAGE="${IMAGE}" \
  -p IMAGE_TAG="${IMAGE_TAG}" \
  -p GRAPHDATA_IMAGE="${GRAPH_SOURCE}" \
  -p REPLICAS=2 \
  > /tmp/manifests.yaml
backoff oc apply -f /tmp/manifests.yaml

backoff oc -n openshift-update-service wait --for=condition=Ready pod -l app=e2e || {
    status=$?
    set +e -x

    # Print various information about the deployment
    oc describe operators/cincinnati-operator.openshift-update-service -n openshift-update-service
    oc get events
    oc describe deployment/e2e
    oc describe pods --selector='app=e2e'
    oc logs --all-containers=true --timestamps=true --selector='app=e2e'

    exit $status
}

# Expose services
PE_URL=$(oc get route e2e-policy-engine-route -o jsonpath='{.spec.host}')
export GRAPH_URL="https://${PE_URL}/api/upgrades_info/graph"

# Wait for route to become available
backoff test "$(curl -ks -o /dev/null -w "%{http_code}" --header 'Accept:application/json' "${GRAPH_URL}?channel=a")" = "200"

# Wait for cincinnati metrics to be recorded
# Find out the token Prometheus uses from its serviceaccount secrets
# and use it to query for GB build info
PROM_ROUTE=$(oc -n openshift-monitoring get route thanos-querier -o jsonpath="{.spec.host}")
export PROM_ENDPOINT="https://${PROM_ROUTE}"
echo "Using Prometheus endpoint ${PROM_ENDPOINT}"

export PROM_TOKEN=$(oc -n openshift-monitoring get secret \
  $(oc -n openshift-monitoring get serviceaccount prometheus-k8s \
    -o jsonpath='{range .secrets[*]}{.name}{"\n"}{end}' | grep prometheus-k8s-token) \
  -o go-template='{{.data.token | base64decode}}')

DELAY=30
for i in $(seq 1 10); do
  PROM_OUTPUT=$(curl -kLs -H "Authorization: Bearer ${PROM_TOKEN}" "${PROM_ENDPOINT}/api/v1/query?query=cincinnati_gb_build_info") || continue
  grep "metric" <<< "${PROM_OUTPUT}" || continue && break

  sleep ${DELAY}
done

# Show test failure details
export RUST_BACKTRACE="1"

# Run e2e tests
/usr/bin/cincinnati-e2e-test $run_ignored

# Ensure prometheus_query tests are executed
/usr/bin/cincinnati-prometheus_query-test

# Run load-testing script
export GRAPH_INTERNAL_URL="http://e2e-policy-engine.openshift-update-service.svc.cluster.local/api/upgrades_info/graph"
cat hack/vegeta.targets| sed "s;GRAPH_URL;${GRAPH_INTERNAL_URL};g" > /tmp/vegeta.targets
oc create configmap vegeta-config --from-file /tmp/vegeta.targets

# Create a in-cluster Job
oc apply -f dist/openshift/load-testing.yaml

# Wait for it to complete
backoff oc -n openshift-update-service wait --for=condition=Complete job load-testing --timeout=900s

# sleep for 30 secs to allow Prometheus scrape latest data
sleep 30

# Verify SLO metrics
# Disabled due to a known bug in performance w/ over 40rps
# /usr/bin/cincinnati-e2e-slo
