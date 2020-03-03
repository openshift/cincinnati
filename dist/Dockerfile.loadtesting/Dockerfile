FROM registry.svc.ci.openshift.org/openshift/release:golang-1.13 AS builder
RUN env GOBIN=/usr/local/bin go get -u github.com/tsenart/vegeta

FROM centos:7

# Copy vegeta binary
COPY --from=builder /usr/local/bin/vegeta /usr/bin

# Copy load testing files
COPY hack/load-testing.sh /usr/local/bin/load-testing.sh
COPY hack/vegeta.targets /usr/local/bin/vegeta.targets
WORKDIR /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/load-testing.sh"]
