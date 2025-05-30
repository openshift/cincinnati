#!/usr/bin/env bash

set -o nounset
set -o errexit
set -o pipefail

echo -e "\nThis Cincinnati deployment script is just an example; it should not be used in production.\n" >&2

export IMAGE_TAG=6284444

# Create a new namespace/project for Cincinnati if it does not exist
oc create namespace cincinnati --dry-run=client -o yaml | oc apply -f -
oc project cincinnati

# Create a dummy secret as a workaround to not having real secrets
oc create secret generic cincinnati-credentials --from-literal="foo=bar" --dry-run=client -o yaml | oc apply -f -

# Install keda CRD required by cincinnati-deployment.yaml
# --server-side is for https://github.com/kedacore/keda/issues/4740
oc get crd scaledjobs.keda.sh || oc apply --server-side -f https://github.com/kedacore/keda/releases/download/v2.17.1/keda-2.17.1-crds.yaml

# Apply oc template
oc process -f dist/openshift/cincinnati-deployment.yaml \
  -p IMAGE_TAG=${IMAGE_TAG}\
  -p GB_PAUSE_SECS=300 \
  -p GB_PLUGIN_SETTINGS="$(cat <<-EOF
      [[plugin_settings]]
      name = "release-scrape-dockerv2"
      repository = "openshift-ota/openshift-cincinnati-test-public-manual"
      fetch_concurrency = 16

      [[plugin_settings]]
      name = "github-secondary-metadata-scrape"
      github_org = "openshift"
      github_repo = "cincinnati-graph-data"
      reference_branch = "master"
      output_directory = "/tmp/cincinnati-graph-data"

      [[plugin_settings]]
      name = "openshift-secondary-metadata-parse"

      [[plugin_settings]]
      name = "edge-add-remove"
EOF
)" \
  -p ENVIRONMENT_SECRETS="{}" | oc apply -f - \
  ;
