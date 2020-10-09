#!/usr/bin/env bash

set -euxo pipefail

YAML_LINTER='yamllint'
YAML_RULES='{extends: default, rules: {line-length: {max: 120}}}'
YAML_LINT_CMD=("${YAML_LINTER}" '-s' '-d' "${YAML_RULES}")

if ! type -f "${YAML_LINTER}"; then
  echo "error: could not find ${YAML_LINTER} in PATH"
  exit 1
fi

find . \
  -path "./cincinnati/src/plugins/internal/graph_builder/openshift_secondary_metadata_parser/test_fixtures" -prune -o \
  -path "./vendor" -prune -o \
  -type f \( -name '*.yaml' -o -name '*.yml' \) -print0 | xargs -L 1 -0 "${YAML_LINT_CMD[@]}"
