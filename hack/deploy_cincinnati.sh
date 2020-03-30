#!/usr/bin/env bash

set -o nounset
set -o errexit
set -o pipefail

echo -e "\nThis Cincinnati deployment script is just an example; it should not be used in production.\n" >&2

export IMAGE_TAG=90efacd

# Create a new namespace/project for Cincinnati
oc new-project cincinnati

# Create a dummy secret as a workaround to not having real secrets
oc create secret generic cincinnati-credentials --from-literal=""


# Apply oc template
oc new-app -f dist/openshift/cincinnati.yaml \
  -p IMAGE_TAG=${IMAGE_TAG}\
  -p GB_PAUSE_SECS=300 \
  -p GB_PLUGIN_SETTINGS="$(cat <<-EOF
      [[plugin_settings]]
      name = "release-scrape-dockerv2"
      repository = "openshift-release-dev/ocp-release"
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
  -p ENVIRONMENT_SECRETS="{}" \
  ;
