FROM registry.ci.openshift.org/cincinnati-ci-public/rust:ubi9 as rust_builder
WORKDIR /opt/app-root/src/
COPY . .
USER 0
RUN hack/build_e2e.sh

FROM registry.ci.openshift.org/cincinnati-ci-public/rust:e2e-test-ubi9

ENV HOME="/root"
WORKDIR "${HOME}/cincinnati"

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY ./hack/e2e.sh hack/
COPY ./hack/vegeta.targets hack/
COPY ./dist/openshift/cincinnati-deployment.yaml dist/openshift/
COPY ./dist/openshift/cincinnati-e2e.yaml dist/openshift/
COPY ./dist/openshift/observability.yaml dist/openshift/
COPY ./dist/openshift/load-testing.yaml dist/openshift/
COPY ./e2e/tests/testdata e2e/tests/testdata
COPY ./dist/prepare_ci_credentials.sh dist/
COPY ./dist/cargo_test.sh dist/

ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
