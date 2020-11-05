#!/bin/sh
#
# Usage:
#
#   graph-normalize.sh <cincinnati.json
#
# For example:
#
#   curl -sH 'Accept:application/json' 'https://api.openshift.com/api/upgrades_info/v1/graph?channel=prerelease-4.1' | graph-normalize.sh
set -e

JQ_SCRIPT='
  (.nodes | with_entries(.key |= tostring)) as $nodes_by_index |
  (
    [
      .edges[] |
      {
        from: $nodes_by_index[(.[0] | tostring)].version,
        to: $nodes_by_index[(.[1] | tostring)].version,
      }
    ]
  ) as $edges |
  (
    [
      .nodes[] |
      {
        version,
        payload,
        metadata: .metadata | to_entries | sort_by(.key) | from_entries,
      }
    ] | sort_by(.version | [split(".")[] | split("-")[] | tonumber? // .])
  ) as $reordered_nodes |
  ($reordered_nodes | [to_entries[] | {key: .value.version, value: .key}] | from_entries) as $reordered_nodes_by_version |
  {
    nodes: $reordered_nodes,
    edges: (
      [
        $edges[] |
        [$reordered_nodes_by_version[.from],
          $reordered_nodes_by_version[.to]]
      ] | sort
    ),
  }
'

exec jq -r "${JQ_SCRIPT}"
