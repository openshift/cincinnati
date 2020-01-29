#!/usr/bin/env bash
#
# This script assumes its running from a pod and does the following:
# * Processes the template to setup a local cincy instance
# * Prepares a graph using test data
# * Ensures generated graph is valid

# Prerequirements:
#   * pull secret (with registry.svc.ci.openshift.org) part in `/tmp/cluster/pull-secret`
#   * env var IMAGE_FORMAT (e.g `registry.svc.ci.openshift.org/ci-op-ish8m5dt/stable:${component}`)

set -euo pipefail

# Create a new project
oc new-project cincinnati-e2e
oc project cincinnati-e2e

# Use pull secret in cincinnati
oc create secret generic cincinnati-registry-credentials --from-file=registry-credentials=/tmp/cluster/pull-secret

# Use this pull secret to fetch images from CI
oc create secret generic ci-pull-secret --from-file=.dockercfg=/tmp/cluster/pull-secret --type=kubernetes.io/dockercfg
oc secrets link default ci-pull-secret --for=pull

# Apply oc template
oc new-app -f dist/openshift/cincinnati.yaml \
  -p IMAGE="${IMAGE_FORMAT%/*}/stable" \
  -p IMAGE_TAG=deploy \
  -p GB_CINCINNATI_REPO="redhat/openshift-cincinnati-test-public-manual"

# Wait for dc to rollout
oc wait --for=condition=available --timeout=5m deploymentconfig/cincinnati

# Expose services
oc expose service cincinnati-policy-engine --port=policy-engine
PE_URL=$(oc get route cincinnati-policy-engine -o jsonpath='{.spec.host}')
GRAPH_URL="http://${PE_URL}/api/upgrades_info/v1/graph?channel=a"

# Wait for route to become available
ATTEMPTS=10
DELAY=10

while [ $ATTEMPTS -ge 0 ]; do
  CODE=$(curl -s -o /dev/null -w "%{http_code}" --header 'Accept:application/json' "${GRAPH_URL}")
  if [ "${CODE}" == "200" ]; then
    break
  else
    sleep ${DELAY}
    ATTEMPTS=$((ATTEMPTS-1))
  fi
done

# Check that policy engine returns channel data respond
curl --header 'Accept:application/json' "${GRAPH_URL}"
