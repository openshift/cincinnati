FROM quay.io/app-sre/cincinnati:builder AS rust_builder
# build e2e tests
COPY . .

RUN hack/build_e2e.sh

FROM registry.svc.ci.openshift.org/openshift/release:golang-1.13 AS golang_builder
RUN env GOBIN=/usr/local/bin go get -u github.com/tsenart/vegeta

FROM quay.io/app-sre/centos:7

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl https://mirror.openshift.com/pub/openshift-v4/clients/oc/latest/linux/oc.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

# Install container tools
RUN yum -y install skopeo buildah yamllint

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY --from=rust_builder hack/e2e.sh hack/
COPY --from=rust_builder dist/openshift/cincinnati.yaml dist/openshift/
COPY --from=rust_builder dist/openshift/observability.yaml dist/openshift/
COPY --from=golang_builder /usr/local/bin/vegeta /usr/bin
COPY --from=rust_builder hack/load-testing.sh /usr/local/bin/load-testing.sh
COPY --from=rust_builder hack/vegeta.targets vegeta.targets
COPY --from=rust_builder e2e/tests/testdata e2e/tests/testdata

COPY --from=rust_builder dist/prow_yaml_lint.sh dist/
COPY --from=rust_builder dist/prow_rustfmt.sh dist/
COPY --from=rust_builder dist/prepare_ci_credentials.sh dist/

ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
