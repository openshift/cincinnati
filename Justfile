path_prefix := "api/upgrades_info/"
scrape_reference :='revision = "6420f7fbf3724e1e5e329ae8d1e2985973f60c14"'

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

_test component features='test-net,test-net-private' rustcargs='--ignored':
	#!/usr/bin/env bash
	set -xe
	(pushd {{component}} && cargo test --features {{features}} -- {{rustcargs}})


test-net-private:
	#!/usr/bin/env bash
	set -e
	just _test quay "test-net,test-net-private" ""
	just _test cincinnati "test-net,test-net-private" ""
	just _test graph-builder "test-net,test-net-private" ""

run-ci-tests:
	#!/usr/bin/env bash
	set -e
	hack/run-all-tests.sh

path_prefix := "api/upgrades_info/"

# Reads a graph on stdin, creates an SVG out of it and opens it with SVG-associated default viewer. Meant to be combined with one of the `get-graph-*` recipes.
display-graph:
	#!/usr/bin/env bash
	required_tools=("xdg-open" "dot" "jq" "adfasdf")
	for tool in "${required_tools[@]}"; do
		type ${tool} >/dev/null 2>&1 || {
			printf "ERROR: program '%s' not found, please install it.\n" "${tool}"
			exit 1
		}
	done

	jq -cM . | {{invocation_directory()}}/hack/graph.sh | dot -Tsvg > graph.svg; xdg-open graph.svg

run-graph-builder registry="https://quay.io" repository="openshift-release-dev/ocp-release" credentials_file="${HOME}/.docker/config.json":
	#!/usr/bin/env bash
	export RUST_BACKTRACE=1
	cargo run --package graph-builder -- -c <(cat <<'EOF'
		verbosity = "vvv"

		[service]
		pause_secs = 9999999
		address = "127.0.0.1"
		port = 8080
		path_prefix = "{{path_prefix}}"

		[status]
		address = "127.0.0.1"
		port = 9080

		[[plugin_settings]]
		name="release-scrape-dockerv2"
		registry = "{{registry}}"
		repository = "{{repository}}"
		fetch_concurrency=128
		credentials_file = "{{credentials_file}}"

		[[plugin_settings]]
		name="quay-metadata"
		repository="{{repository}}"

		[[plugin_settings]]
		name="node-remove"

		[[plugin_settings]]
		name="edge-add-remove"
	EOF
	)

run-graph-builder-satellite:
	just run-graph-builder 'sat-r220-02.lab.eng.rdu2.redhat.com' 'default_organization-custom-ocp'

run-policy-engine:
	#!/usr/bin/env bash
	export RUST_BACKTRACE=1 RUST_LOG="policy_engine=trace,cincinnati=trace,actix=trace,actix_web=trace"
	cargo run --package policy-engine -- -vvvv --service.address 0.0.0.0 --service.path_prefix {{path_prefix}} --upstream.cincinnati.url 'http://localhost:8080/{{path_prefix}}v1/graph' --service.mandatory_client_parameters='channel'

kill-daemons:
	pkill graph-builder
	pkill policy-engine

run-daemons:
	#!/usr/bin/env bash
	just run-graph-builder #"https://quay.io" "redhat/openshift-cincinnati-test-public-manual" ~/.docker/config.json 2>&1 &
	PG_PID=$!

	just run-policy-engine 2>&1 &
	PE_PID=$!

	trap "kill $PG_PID $PE_PID" EXIT
	while true; do sleep 30; done

get-graph port channel arch host="http://localhost":
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
