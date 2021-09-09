#!/usr/bin/env bash
set -e

CINCINNATI_REPO="${CINCINNATI_REPO:?Please set CINCINNATI_REPO}"
CINCINNATI_REGISTRY="${CINCINNATI_REGISTRY:?Please set CINCINNATI_REGISTRY}"

CINCINNATI_IMG="${CINCINNATI_IMG:-cincinnati-img}"
CINCINNATI_IMG_TAG="${CINCINNATI_IMG_TAG:-quickstart}"
CINCINNATI_IMG_TAGGED="${CINCINNATI_IMG}:${CINCINNATI_IMG_TAG}"
CINCINNATI_GB="cincinnati-graph-builder"
CINCINNATI_GB_PORT="${CINCINNATI_GB_PORT:-8080}"
CINCINNATI_GB_CREDENTIALS="${CINCINNATI_GB_CREDENTIALS:-}"
CINCINNATI_PE="cincinnati-pe"
CINCINNATI_PE_PORT="${CINCINNATI_PE_PORT:-8081}"
CINCINNATI_LABEL="cincinnati-$RANDOM"

CURL_OUT="$(mktemp)"

# Build a container image with both components
if ! 2>&1 >/dev/null docker image inspect "${CINCINNATI_IMG_TAGGED}" || \
    test "${FORCE_REBUILD}"; then
    IMAGE="${CINCINNATI_IMG}" IMAGE_TAG="${CINCINNATI_IMG_TAG}" dist/build_deploy.sh
fi

function cleanup() {
    set +e
    containers=$(docker ps --quiet --filter "label=${CINCINNATI_LABEL}")
    if test "${containers}"; then
        docker stop ${containers} 2>&1 >/dev/null
    fi
    rm -f "${CURL_OUT}"
    pkill "docker logs -f ${CINCINNATI_GB}"
}
trap cleanup EXIT

# Run the graph builder
echo Spawning graph-builder container...
docker run -d --rm \
  --label="${CINCINNATI_LABEL}" \
  --name "${CINCINNATI_GB}" \
  --env "RUST_LOG=${RUST_LOG:-actix_web=error,dkregistry=error}" \
  -p 127.0.0.1:"${CINCINNATI_GB_PORT}":"${CINCINNATI_GB_PORT}" \
  ${CINCINNATI_GB_CREDENTIALS:+-v "${CINCINNATI_GB_CREDENTIALS}":/etc/docker/config:ro} \
  "${CINCINNATI_IMG_TAGGED}" \
  \
  -vvv \
  --address 0.0.0.0 --port "${CINCINNATI_GB_PORT}" --registry "${CINCINNATI_REGISTRY}" --repository "${CINCINNATI_REPO}" \
  ${CINCINNATI_GB_CREDENTIALS:+"--credentials-file=/etc/docker/config"} \
  2>&1 >/dev/null
docker logs -f ${CINCINNATI_GB} &

# Run the policy engine
echo Spawning policy-engine container...
docker run -d --rm \
  --label="${CINCINNATI_LABEL}" \
  --name "${CINCINNATI_PE}" \
  --env "RUST_LOG=${RUST_LOG:-actix_web=error,dkregistry=error}" \
  -p 127.0.0.1:"${CINCINNATI_PE_PORT}":"${CINCINNATI_PE_PORT}" \
  --entrypoint "/usr/bin/policy-engine" \
  "${CINCINNATI_IMG_TAGGED}" \
  \
  -vvv \
  --address 0.0.0.0 --port "${CINCINNATI_PE_PORT}" \
  --upstream "http://$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${CINCINNATI_GB}):"${CINCINNATI_GB_PORT}"/graph" \
  2>&1 >/dev/null

# Test the policy engine endpoint, which depends on the graph builder being available
TIMEOUT="${TIMEOUT:-30}"
echo Trying to reach the policy-engine with a timeout of ${TIMEOUT} seconds...
jq=$(which jq || echo cat)
for attempt in $(seq 0 ${TIMEOUT}); do
    statuscode=$(curl --silent --output "${CURL_OUT}" --write-out "%{http_code}" --header 'Accept:application/json' "http://localhost:"${CINCINNATI_PE_PORT}"/graph")
    if test "$statuscode" -eq 200; then
        echo "Got graph:"
        $jq < "${CURL_OUT}"
        break
    fi

    printf "HTTP status: ${statuscode}. Trying $((${TIMEOUT} - ${attempt})) more times until giving up. CTRL+C to abort...\n"
    sleep 1
done
