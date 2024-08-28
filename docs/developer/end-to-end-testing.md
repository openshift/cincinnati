e2e tests
====

Cincinnati CI is ensuring that pull-request has not broken Cincinnati setup, verifies the graph is valid and the performance didn't degrade. This is ensured by `e2e-aws` CI test on every pull-request, which is required to pass.

# e2e-aws test in CI

## Setup

In Cincinnati PRs Openshift CI (Prow) is [configured](https://github.com/openshift/release/blob/6c4e03a/ci-operator/config/openshift/cincinnati/openshift-cincinnati-master.yaml#L72-L77) to start a new openshift cluster on AWS. Once it's installed, CI is starting [hack/e2e.sh](../../hack/e2e.sh) script in the container.

This script installs Cincinnati on a cluster. In detail it does the following:
* create a new project
* use cluster's pull secret as a pull secret for Cincinnati
* apply Cincinnati deploy manifests (see [openshift template](../../dist/openshift/cincinnati-deployment.yaml))
* use prod-like configuration for graph-builder and policy-engine
* wait for Cincinnati to rollout, expecting liveness/readiness probes to pass

Additionally it reconfigures the cluster's Prometheus cluster to support [user workloads](https://docs.openshift.com/container-platform/4.4/monitoring/monitoring-your-own-services.html). This spawns a new 
Prometheus instance, which is used to scrape Cincinnati's metrics (using [observability](../../dist/openshift/observability.yaml) template).

## end to end tests

### Graph-builder and policy-engine functionality

Once the script is done it starts the end-to-end tests from [e2e package](../../e2e/tests). 

First test (`e2e_channel_success`) checking the graph in the deployed Cincinnati and ensure that [expected releases](../../e2e/tests/testdata) are present in the graph and have valid metadata. The test verifies that graph-builder is able to scrape production images from Quay correctly and policy-engine properly filters the graph by channel and architecture.

### Load testing

Second test is ensuring that Cincinnati instance can handle the load in order to adhere to SLO requirements from app-sre team. Load-testing is started by [load-testing.sh](../../hack/load-testing.sh) script.
This script starts multiple connections to the deployed Cincinnati instance using [vegeta](https://github.com/tsenart/vegeta) tool. The tool checks that repeated requests to [targets](../../hack/vegeta.targets) return a HTTP 2xx code. Vegeta varies the amount of parallel connections (`workers` in `load-testing.sh`) and rate of requests (`rate` in `load-testing.sh`). The tool also ensures that the response is received within the specified `duration` (30s).

### SLO verification

Once the load has finished the third test - `check_slo` from [slo.rs](../../e2e/tests/slo.rs) - ensures that metric values didn't break the established SLO. The test queries user workload Prometheus, runs queries specified as test case parameters and ensures expected result is received. SLO test ensures that neither service stopped reporting, container didn't restart, images have been scraped without errors, and any request has been processed in less that 0.5 second.

If any of these tests fail, the `e2e-aws` task fails and doesn't proceed further

# Test overrides

Use `/override e2e-aws` to reset the status of the test if the PR needs to merge urgently and test results is invalid. Thi command is restricted to [Cincinnati owners](../../OWNERS_ALIASES).

# Stage deployment

When cincinnati is deployed on stage environment the same end-to-end suite is performed, using app-sre's Prometheus as a metric source. This allows Cincinnati team to ensure that exposed metrics are expected, check that SLO requirements are met and the team is notified via alertmanager otherwise.
