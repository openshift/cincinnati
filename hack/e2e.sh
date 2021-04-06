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

# Use defined PULL_SECRET or fall back to CI location
PULL_SECRET=${PULL_SECRET:-/var/run/secrets/ci.openshift.io/cluster-profile/pull-secret}

set -euo pipefail
# Copy KUBECONFIG so that it can be mutated
cp -Lrvf $KUBECONFIG /tmp/kubeconfig
export KUBECONFIG=/tmp/kubeconfig

# Create a new project
oc new-project cincinnati-e2e
oc project cincinnati-e2e

# Create a dummy secret as a workaround to not having real secrets in e2e
oc create secret generic cincinnati-credentials --from-literal=""

# Use this pull secret to fetch images from CI
oc create secret generic ci-pull-secret --from-file=.dockercfg=${PULL_SECRET} --type=kubernetes.io/dockercfg

# Wait for default service account to appear
for ATTEMPT in $(seq 0 5); do
  oc get serviceaccount default && break
  sleep 5
done
# Allow default serviceaccount to use CI pull secret
oc secrets link default ci-pull-secret --for=pull

# Reconfigure monitoring operator to support user workloads

# https://docs.openshift.com/container-platform/4.5/monitoring/monitoring-your-own-services.html
oc -n openshift-monitoring create configmap cluster-monitoring-config --from-literal='config.yaml={"techPreviewUserWorkload": {"enabled": true}}' -o yaml --dry-run=client > /tmp/cluster-monitoring-config.yaml
oc apply -f /tmp/cluster-monitoring-config.yaml

# https://docs.openshift.com/container-platform/4.7/monitoring/configuring-the-monitoring-stack.html#creating-user-defined-workload-monitoring-configmap_configuring-the-monitoring-stack
oc -n openshift-user-workload-monitoring create configmap user-workload-monitoring-config --from-literal='config.yaml=' -o yaml --dry-run=client > /tmp/cluster-user-workload-monitoring-config.yaml
oc apply -f /tmp/cluster-user-workload-monitoring-config.yaml

# Import observability template
# ServiceMonitors are imported before app deployment to give Prometheus time to catch up with
# metrics
# `oc new-app` would stumble on unknown monitoring.coreos.com/v1 objects, so process and create instead
oc process -f dist/openshift/observability.yaml -p NAMESPACE="cincinnati-e2e" | oc apply -f -

# Export the e2e test environment variables
E2E_TESTDATA_DIR="${E2E_TESTDATA_DIR:-e2e/tests/testdata}"
export E2E_TESTDATA_DIR
read -r E2E_METADATA_REVISION <"${E2E_TESTDATA_DIR}"/metadata_revision
export E2E_METADATA_REVISION

# Apply oc template
oc new-app -f dist/openshift/cincinnati.yaml \
  -p IMAGE="${IMAGE}" \
  -p IMAGE_TAG="${IMAGE_TAG}" \
  -p GB_CPU_REQUEST=50m \
  -p PE_CPU_REQUEST=50m \
  -p RUST_BACKTRACE="1" \
  -p GB_PLUGIN_SETTINGS="$(cat <<-EOF
      [[plugin_settings]]
      name = "release-scrape-dockerv2"
      registry = "${E2E_SCRAPE_REGISTRY:-quay.io}"
      repository = "${E2E_SCRAPE_REPOSITORY:-openshift-release-dev/ocp-release}"
      fetch_concurrency = 128

      [[plugin_settings]]
      name = "github-secondary-metadata-scrape"
      github_org = "openshift"
      github_repo = "cincinnati-graph-data"
      reference_revision = "${E2E_METADATA_REVISION}"
      output_directory = "/tmp/cincinnati-graph-data"

      [[plugin_settings]]
      name = "openshift-secondary-metadata-parse"

      [[plugin_settings]]
      name = "edge-add-remove"
EOF
)" \
  -p ENVIRONMENT_SECRETS="{}" \
  -p REPLICAS="2" \
  ;

# Wait for dc to rollout
oc wait --for=condition=available --timeout=10m deploymentconfig/cincinnati || {
    status=$?
    set +e -x

    # Print various information about the deployment
    oc get events
    oc describe deploymentconfig/cincinnati
    oc get configmap/cincinnati-configs -o yaml
    oc describe pods --selector='app=cincinnati'
    oc logs --all-containers=true --timestamps=true --selector='app=cincinnati'

    exit $status
}

# Expose services
oc expose service cincinnati-policy-engine --port=policy-engine
PE_URL=$(oc get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
export GRAPH_URL="http://${PE_URL}/api/upgrades_info/v1/graph"

# Wait for route to become available
DELAY=10
for i in $(seq 1 10); do
  CODE=$(curl -s -o /dev/null -w "%{http_code}" --header 'Accept:application/json' "${GRAPH_URL}?channel=a")
  if [ "${CODE}" == "200" ]; then
    break
  fi
  sleep ${DELAY}
done

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
/usr/bin/cincinnati-e2e-test

# Ensure prometheus_query tests are executed
/usr/bin/cincinnati-prometheus_query-test

# Run load-testing script
/usr/local/bin/load-testing.sh

# sleep for 30 secs to allow Prometheus scrape latest data
sleep 30

# Verify SLO metrics
/usr/bin/cincinnati-e2e-slo
