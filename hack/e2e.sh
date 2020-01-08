#!/usr/bin/env bash
#
# This script assumes its running from a pod and does the following:
# * Processes the template to setup a local cincy instance
# * Prepares a graph using test data
# * Ensures generated graph is valid

# Debug information
oc whoami
oc project

# Apply oc template
oc new-app -f ../dist/openshift/cincinnati.yaml \
  -p IMAGE=${CINCINNATI_E2E_IMAGE:-pipeline} \
  -p IMAGE_TAG=${CINCINNATI_E2E_IMAGE_TAG:-deploy} \
  -p PE_PORT=${CINCINNATI_PE_PORT:-8081} \
  -p GB_CINCINNATI_REPO="redhat/openshift-cincinnati-test-public-manual"

# Wait for dc to rollout
oc wait --for=condition=available deploymentconfig/cincinnati

# Check that policy engine returns channel data respond
GRAPH_URL="http://${CINCINNATI_PE_BASE_URL:-cincinnati-policy-engine}:${CINCINNATI_PE_PORT:-8081}/v1/graph"
curl --verbose --header 'Accept:application/json' "${GRAPH_URL}?channel=a"
