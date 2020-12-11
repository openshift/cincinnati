FROM quay.io/app-sre/cincinnati:builder AS rust_builder
# build e2e tests
COPY . .
# Use root user
USER 0

# Install jq
RUN mkdir -p ${HOME}/bin && \
    curl -L https://github.com/stedolan/jq/releases/download/jq-1.6/jq-linux64 2>/dev/null -o ${HOME}/bin/jq && chmod a+x ${HOME}/bin/jq
ENV PATH="${PATH}:${HOME}/bin"

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
RUN yum -y install yamllint

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY --from=golang_builder /usr/local/bin/vegeta /usr/bin

COPY hack/e2e.sh hack/
COPY dist/openshift/cincinnati.yaml dist/openshift/
COPY dist/openshift/observability.yaml dist/openshift/
COPY hack/load-testing.sh /usr/local/bin/load-testing.sh
COPY hack/vegeta.targets vegeta.targets
COPY e2e/tests/testdata e2e/tests/testdata
COPY dist/prow_yaml_lint.sh dist/
COPY dist/prow_rustfmt.sh dist/
COPY dist/prepare_ci_credentials.sh dist/
ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
