#!/bin/sh

UPSTREAM="${UPSTREAM:-https://api.openshift.com/api/upgrades_info/v1/graph}"
CHANNEL="${CHANNEL:-stable-4.7}"
ARCH="${ARCH:-amd64}"
VERSION="$1"
OUTPUT=$(mktemp)

# Register function to be called on EXIT to remove tmp file
function cleanup {
  rm -f "${OUTPUT}"
}
trap cleanup EXIT

if test -z "${VERSION}" -o "$#" -ne 1
then
	printf 'usage: %s VERSION\n\nOptional environment variables:\n\nUPSTREAM: Cincinnati upstream (default %s)\nCHANNEL: Graph channel (default %s)\nARCH: Cluster architecture (default %s)\n' "$0" "${UPSTREAM}" "${CHANNEL}" "${ARCH}" >&2
	exit 1
fi

if ! curl --silent --location --fail --header 'Accept:application/json' "${UPSTREAM}?channel=${CHANNEL}&arch=${ARCH}" -o "${OUTPUT}"; then
	 echo "Failed to fetch data from ${URI}"
   exit 1
fi

cat "${OUTPUT}" | jq -r "
  (.nodes | with_entries(.key |= tostring)) as \$nodes_by_index |
  [
    .edges[] |
    select(\$nodes_by_index[(.[0] | tostring)].version == \"${VERSION}\")[1] |
    tostring |
    \$nodes_by_index[.]
  ] | sort_by(.version)[] |
  .version + \"\\t\" + .payload + \"\\t\" + (.metadata.url // \"-\")
"
