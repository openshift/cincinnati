#!/bin/sh
#
# Usage:
#
#   cat cincinnati.json | graph.sh >graph.dot
#
# For example:
#
#   curl -sH 'Accept:application/json' 'https://api.openshift.com/api/upgrades_info/v1/graph?channel=prerelease-4.1' | graph.sh | dot -Tsvg >graph.svg
set -e

JQ_SCRIPT='"digraph Upgrades {\n  labelloc=t;\n  rankdir=BT;" as $header |
  (
    [
      .nodes |
      to_entries[] |
      "  " + (.key | tostring) +
             " [ label=\"" + .value.version + "\"" + (
               if .value.metadata.url then " href=\"" + .value.metadata.url + "\"" else "" end
             ) +
             " ];"
    ] | join("\n")
  ) as $nodes |
  (
    [
      .edges[] |
      "  " + (.[0] | tostring) + "->" + (.[1] | tostring) + ";"
    ] | join("\n")
  ) as $edges |
  [$header, $nodes, $edges, "}"] | join("\n")
'

exec jq -r "${JQ_SCRIPT}"
