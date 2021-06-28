path_prefix := "api/upgrades_info/"

testdata_dir := "e2e/tests/testdata/"
metadata_revision_file := "metadata_revision"
metadata_reference :='reference_branch = "master"'

pause_secs := "9999999"
registry := "https://quay.io"
repository := "openshift-release-dev/ocp-release"
credentials_file := "${HOME}/.docker/config.json"
default_tracing_endpoint := "localhost:6831"

metadata_reference_e2e:
	printf 'reference_revision = "%s"' "$(cat {{testdata_dir}}/{{metadata_revision_file}})"

metadata_reference_revision:
	#!/usr/bin/env bash
	read -r var <"{{testdata_dir}}"/metadata_revision; printf $var

format:
	cargo fmt --all

commit +args="": format
	git commit {{args}}

build:
	cargo build

_coverage:
	cargo kcov --verbose --all --no-clean-rebuild --open

coverage: test _coverage

test-pwd +args="":
	#!/usr/bin/env bash
	set -e
	export RUST_BACKTRACE=1 RUST_LOG="graph-builder=trace,cincinnati=trace,dkregistry=trace"
	pushd {{invocation_directory()}}
	cargo test {{args}}

test: format
	#!/usr/bin/env bash
	set -e
	export RUST_BACKTRACE=1 RUST_LOG="graph-builder=trace,cincinnati=trace,dkregistry=trace"
	cargo test --all

_test component cargoargs='--features test-net,test-net-private' rustcargs='--ignored':
	#!/usr/bin/env bash
	set -xe
	export RUST_BACKTRACE=1 RUST_LOG="graph-builder=trace,cincinnati=trace,dkregistry=trace"
	(pushd {{component}} && cargo test -- --nocapture {{cargoargs}} -- {{rustcargs}})


test-net-private:
	#!/usr/bin/env bash
	set -e
	just _test quay "--features test-net,test-net-private" ""
	just _test cincinnati "--features test-net,test-net-private" ""
	just _test graph-builder "--features test-net,test-net-private" ""

run-ci-tests:
	#!/usr/bin/env bash
	set -e
	hack/run-all-tests.sh

# Runs the client part of the e2e test suite.
run-e2e-test-only filter="e2e":
	#!/usr/bin/env bash
	set -e
	export GRAPH_URL='http://127.0.0.1:8081/{{path_prefix}}v1/graph'
	export E2E_METADATA_REVISION="$(just metadata_reference_revision)"

	# we need to use the parent-directory here because the test runs in the graph-builder directory
	export E2E_TESTDATA_DIR="../{{ testdata_dir }}"

	just _test e2e "" "{{ filter }}"

# Spawns a Cincinnati stack on localhost and runs the e2e test suite.
run-e2e:
	#!/usr/bin/env bash
	set -e

	just \
		registry="{{registry}}" repository="{{repository}}" \
		run-daemons-e2e > /dev/null 2>&1 &

	trap "just kill-daemons" EXIT
	# give the graph-builder time to scrape
	sleep 180

	for i in `seq 1 100`; do
		just run-e2e-test-only && {
			echo Test successful.
			exit 0
		} || {
			echo Attempt failed. Trying again in 10 seconds.
			sleep 10
		}
	done

	echo Test failed.
	exit 1

# Capture new e2e fixtures and refresh the metadata revision file.
e2e-fixtures-capture-only:
	#!/usr/bin/env bash
	set -e

	for base in "stable"; do
		for version in "4.2" "4.3"; do
			for arch in "amd64" "s390x"; do
				just get-graph-pe "${base}-${version}" "${arch}" | hack/graph-normalize.sh > {{testdata_dir}}/"$(just metadata_reference_revision)_${base}-${version}_${arch}".json
			done
		done
	done

# Reads a graph on stdin, creates an SVG out of it and opens it with SVG-associated default viewer. Meant to be combined with one of the `get-graph-*` recipes.
display-graph:
	#!/usr/bin/env bash
	required_tools=("xdg-open" "dot" "jq")
	for tool in "${required_tools[@]}"; do
		type ${tool} >/dev/null 2>&1 || {
			printf "ERROR: program '%s' not found, please install it.\n" "${tool}"
			exit 1
		}
	done

	jq -cM . | {{invocation_directory()}}/hack/graph.sh | dot -Tsvg > graph.svg; xdg-open graph.svg

run-graph-builder:
	#!/usr/bin/env bash
	export RUST_BACKTRACE=1

	trap 'rm -rf "$TMPDIR"' EXIT
	export TMPDIR=$(mktemp -d)

	cargo run --package graph-builder -- -c <(cat <<-EOF
		verbosity = "vvv"

		[service]
		scrape_timeout_secs = 300
		pause_secs = {{pause_secs}}
		address = "127.0.0.1"
		port = 8080
		path_prefix = "{{path_prefix}}"
		tracing_endpoint = "{{default_tracing_endpoint}}"

		[status]
		address = "127.0.0.1"
		port = 9080

		[[plugin_settings]]
		name="release-scrape-dockerv2"
		registry = "{{registry}}"
		repository = "{{repository}}"
		fetch_concurrency=128
		credentials_path = "{{credentials_file}}"

		[[plugin_settings]]
		name = "github-secondary-metadata-scrape"
		github_org = "openshift"
		github_repo = "cincinnati-graph-data"
		branch = "master"
		output_directory = "${TMPDIR}"
		{{metadata_reference}}

		[[plugin_settings]]
		name = "openshift-secondary-metadata-parse"

		[[plugin_settings]]
		name = "edge-add-remove"
	EOF
	)

run-graph-builder-satellite:
	just registry='sat-r220-02.lab.eng.rdu2.redhat.com' repository='default_organization-custom-ocp' run-graph-builder

run-graph-builder-e2e:
	just \
		registry="{{registry}}" repository="{{repository}}" \
		metadata_reference="$(just metadata_reference_e2e)" \
		run-graph-builder

run-policy-engine:
	#!/usr/bin/env bash
	export RUST_BACKTRACE=1 RUST_LOG="policy_engine=trace,cincinnati=trace,actix=trace,actix_web=trace"
	cargo run --package policy-engine -- -vvvv --service.address 0.0.0.0 --service.path_prefix {{path_prefix}} --upstream.cincinnati.url 'http://127.0.0.1:8080/{{path_prefix}}v1/graph' --service.mandatory_client_parameters='channel' --service.tracing_endpoint "{{default_tracing_endpoint}}"

kill-daemons:
	pkill graph-builder
	pkill policy-engine

run-daemons:
	#!/usr/bin/env bash
	just run-graph-builder 2>&1 &
	PG_PID=$!

	just run-policy-engine 2>&1 &
	PE_PID=$!

	trap "kill $PG_PID $PE_PID" EXIT
	sleep infinity

run-daemons-e2e:
	#!/usr/bin/env bash
	just \
		registry="{{registry}}" repository="{{repository}}" \
		run-graph-builder-e2e 2>&1 &

	just run-policy-engine 2>&1 &

	trap "just kill-daemons" EXIT
	sleep infinity

get-graph port channel arch host="http://127.0.0.1":
	curl --header 'Accept:application/json' {{host}}:{{port}}/{{path_prefix}}v1/graph?channel='{{channel}}'\&arch='{{arch}}' | jq .

get-graph-gb:
	just get-graph 8080 "" ""

get-graph-pe channel='' arch='':
	just get-graph 8081 "{{channel}}" "{{arch}}"

get-graph-pe-staging channel='stable-4.1' arch='amd64':
	just get-graph 443 "{{channel}}" "{{arch}}" https://api.stage.openshift.com

get-graph-pe-production channel='stable-4.0' arch='amd64':
	just get-graph 443 "{{channel}}" "{{arch}}" https://api.openshift.com

get-openapi host port:
	curl --header 'Accept:application/json' {{host}}:{{port}}/{{path_prefix}}v1/openapi | jq .

get-openapi-staging:
	just get-openapi https://api.stage.openshift.com 443

get-openapi-production:
	just get-openapi https://api.openshift.com 443
