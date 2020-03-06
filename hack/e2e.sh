#!/usr/bin/env bash
#
# This script assumes its running from a pod and does the following:
# * Processes the template to setup a local cincy instance
# * Prepares a graph using test data
# * Ensures generated graph is valid

# Prerequirements:
#   * pull secret (with registry.svc.ci.openshift.org) part in `/tmp/cluster/pull-secret`
#   * CINCINNATI_IMAGE (optional) - image with graph-builder and policy-engine
#   * env var IMAGE_FORMAT (e.g `registry.svc.ci.openshift.org/ci-op-ish8m5dt/stable:${component}`)

set -euo pipefail

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
PULL_SECRET=${PULL_SECRET:-/tmp/cluster/pull-secret}

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

# Apply oc template
oc new-app -f dist/openshift/cincinnati.yaml \
  -p IMAGE="${IMAGE}" \
  -p IMAGE_TAG="${IMAGE_TAG}" \
  -p GB_CPU_REQUEST=50m \
  -p PE_CPU_REQUEST=50m \
  -p RUST_BACKTRACE="1" \
  -p GB_PLUGIN_SETTINGS='
      [[plugin_settings]]
      name = "release-scrape-dockerv2"
      repository = "redhat/openshift-cincinnati-test-public-manual"
      fetch_concurrency = 128

      [[plugin_settings]]
      name = "quay-metadata"
      repository = "redhat/openshift-cincinnati-test-public-manual"

      [[plugin_settings]]
      name = "node-remove"

      [[plugin_settings]]
      name = "edge-add-remove"
  ' \
  ;

# Wait for dc to rollout
oc wait --for=condition=available --timeout=5m deploymentconfig/cincinnati || {
    status=$?
    set +e -x

    # Print various information about the deployment
    oc get events
    oc describe deploymentconfig/cincinnati
    oc get configmap/cincinnati-configs -o yaml
    oc logs --all-containers=true --timestamps=true --selector='app=cincinnati'

    exit $status
}

# Expose services
oc expose service cincinnati-policy-engine --port=policy-engine
PE_URL=$(oc get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
export GRAPH_URL="http://${PE_URL}/api/upgrades_info/v1/graph"

# Wait for route to become available
ATTEMPTS=10
DELAY=10

while [ $ATTEMPTS -ge 0 ]; do
  CODE=$(curl -s -o /dev/null -w "%{http_code}" --header 'Accept:application/json' "${GRAPH_URL}?channel=a")
  if [ "${CODE}" == "200" ]; then
    break
  else
    sleep ${DELAY}
    ATTEMPTS=$((ATTEMPTS-1))
  fi
done

# Run e2e tests
/usr/bin/cincinnati-e2e-test
