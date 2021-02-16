FROM quay.io/app-sre/cincinnati:builder AS rust_builder
RUN hack/build_e2e.sh

FROM registry.ci.openshift.org/openshift/release:golang-1.13 AS golang_builder
RUN env GOBIN=/usr/local/bin go get -u github.com/tsenart/vegeta

FROM registry.centos.org/centos/centos:7

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl https://mirror.openshift.com/pub/openshift-v4/clients/oc/latest/linux/oc.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

# Install container tools
RUN yum update -y && \
    yum install -y skopeo buildah && \
    yum clean all

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/hack/e2e.sh hack/
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/dist/openshift/cincinnati.yaml dist/openshift/
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/dist/openshift/observability.yaml dist/openshift/
COPY --from=golang_builder /usr/local/bin/vegeta /usr/bin
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/hack/load-testing.sh /usr/local/bin/load-testing.sh
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/hack/vegeta.targets vegeta.targets
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/e2e/tests/testdata e2e/tests/testdata
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/dist/prepare_ci_credentials.sh dist/

ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
