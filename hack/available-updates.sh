#!/bin/sh

UPSTREAM="${UPSTREAM:-https://api.openshift.com/api/upgrades_info/v1/graph}"
CHANNEL="${CHANNEL:-stable-4.3}"
ARCH="${ARCH:-amd64}"
VERSION="$1"

if test -z "${VERSION}" -o "$#" -ne 1
then
	printf 'usage: %s VERSION\n\nOptional environment variables:\n\nUPSTREAM: Cincinnati upstream (default %s)\nCHANNEL: Graph channel (default %s)\nARCH: Cluster architecture (default %s)\n' "$0" "${UPSTREAM}" "${CHANNEL}" "${ARCH}" >&2
	exit 1
fi

URI="${UPSTREAM}?channel=${CHANNEL}&arch=${ARCH}"
DATA="$(curl -sHL Accept:application/json "${URI}")"
if test -z "${DATA}"
then
	 echo "Failed to fetch data from ${URI}"
fi

echo "${DATA}" | jq -r "
  (.nodes | with_entries(.key |= tostring)) as \$nodes_by_index |
  [
    .edges[] |
    select(\$nodes_by_index[(.[0] | tostring)].version == \"${VERSION}\")[1] |
    tostring |
    \$nodes_by_index[.]
  ] | sort_by(.version)[] |
  .version + \"\\t\" + .payload + \"\\t\" + (.metadata.url // \"-\")
"
