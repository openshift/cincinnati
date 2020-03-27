FROM quay.io/app-sre/cincinnati:builder AS rust_builder
# build e2e tests
COPY . .

# Build and copy test binary
RUN cd e2e && \
    cargo build --tests --features test-e2e-prom-query && \
    mkdir -p /opt/cincinnati/bin && \
    cp -rvf ../target/debug/e2e-* /opt/cincinnati/bin && \
    rm -rf /opt/cincinnati/bin/e2e-*.d && \
    mv /opt/cincinnati/bin/e2e-* /opt/cincinnati/bin/cincinnati-e2e-test && \
    cp -rvf ../target/debug/prometheus_query-* /opt/cincinnati/bin && \
    rm -rf /opt/cincinnati/bin/prometheus_query-*.d && \
    mv /opt/cincinnati/bin/prometheus_query-* /opt/cincinnati/bin/cincinnati-prometheus_query-test && \
    cp -rvf ../target/debug/slo-* /opt/cincinnati/bin && \
    rm -rf /opt/cincinnati/bin/slo-*.d && \
    mv /opt/cincinnati/bin/slo-* /opt/cincinnati/bin/cincinnati-e2e-slo

FROM registry.svc.ci.openshift.org/openshift/release:golang-1.13 AS golang_builder
RUN env GOBIN=/usr/local/bin go get -u github.com/tsenart/vegeta

FROM centos:7

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl https://mirror.openshift.com/pub/openshift-v4/clients/oc/latest/linux/oc.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

COPY --from=rust_builder /opt/cincinnati/bin/cincinnati-e2e-test /usr/bin
COPY --from=rust_builder /opt/cincinnati/bin/cincinnati-prometheus_query-test /usr/bin
COPY --from=rust_builder /opt/cincinnati/bin/cincinnati-e2e-slo /usr/bin
COPY --from=rust_builder hack/e2e.sh hack/
COPY --from=rust_builder dist/openshift/cincinnati.yaml dist/openshift/
COPY --from=rust_builder dist/openshift/observability.yaml dist/openshift/
COPY --from=golang_builder /usr/local/bin/vegeta /usr/bin
COPY --from=rust_builder hack/load-testing.sh /usr/local/bin/load-testing.sh
COPY --from=rust_builder hack/vegeta.targets vegeta.targets

ENTRYPOINT ["hack/e2e.sh"]
