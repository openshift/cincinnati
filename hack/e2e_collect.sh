#!/usr/bin/env bash

set -e

# Fetch graph data from Cincinnati
# Input params:
# * 1 - cincinnati URL
# * 2 - prefix

type -f curl > /dev/null || {
  echo Please install curl
  exit 1
}

test -z "${1}" && echo "Usage: ./e2e_collect.sh CINCINNATI_URL" && exit 1

TMPFILE="$(mktemp)"
curl --silent --header 'Accept:application/json' -o "${TMPFILE}" "${1}"
echo "${TMPFILE}"
