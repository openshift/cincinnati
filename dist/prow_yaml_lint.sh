#!/usr/bin/env bash

set -euxo pipefail

cfg="$(mktemp)"
trap 'rm -vf -- "$cfg"' EXIT

cat >"$cfg" <<EOF
extends: default

ignore:
- cincinnati/src/plugins/internal/graph_builder/openshift_secondary_metadata_parser/test_fixtures/
- vendor/
- dist/grafana/dashboards/

rules:
  line-length:
    max: 120
EOF

yamllint -v

yamllint -s -c "$cfg" .
