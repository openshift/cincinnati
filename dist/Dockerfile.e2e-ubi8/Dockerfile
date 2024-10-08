FROM registry.access.redhat.com/ubi8/ubi:latest as rust_builder
WORKDIR /opt/app-root/src/
COPY . .
# copy git information for built crate
COPY .git/ ./.git/
USER 0
RUN dnf update -y \
    && dnf install -y jq rust cargo \
    && dnf install -y openssl-devel \
    && dnf clean all

RUN hack/build_e2e.sh

FROM registry.access.redhat.com/ubi8/ubi:latest AS vegeta_fetcher
RUN curl -L https://github.com/tsenart/vegeta/releases/download/v12.8.4/vegeta_12.8.4_linux_amd64.tar.gz| tar xvzf - -C /usr/local/bin/ vegeta

FROM registry.access.redhat.com/ubi8/ubi:latest

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl -L https://mirror.openshift.com/pub/openshift-v4/clients/ocp/stable/openshift-client-linux-amd64-rhel8.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY --from=rust_builder /opt/app-root/src/hack/e2e.sh hack/
COPY --from=rust_builder /opt/app-root/src/hack/vegeta.targets hack/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/cincinnati-deployment.yaml dist/openshift/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/cincinnati-e2e.yaml dist/openshift/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/observability.yaml dist/openshift/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/load-testing.yaml dist/openshift/
COPY --from=vegeta_fetcher /usr/local/bin/vegeta /usr/bin
COPY --from=rust_builder /opt/app-root/src/e2e/tests/testdata e2e/tests/testdata
COPY --from=rust_builder /opt/app-root/src/dist/prepare_ci_credentials.sh dist/
COPY --from=rust_builder /opt/app-root/src/dist/cargo_test.sh dist/

ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
