FROM quay.io/app-sre/cincinnati:builder AS builder
# build e2e tests
COPY . .

# Build and copy test binary
RUN cd e2e && \
    cargo build --tests && \
    mkdir -p /opt/cincinnati/bin && \
    cp -rvf ../target/debug/mod-* /opt/cincinnati/bin && \
    rm -rf /opt/cincinnati/bin/mod-*.d && \
    mv /opt/cincinnati/bin/mod-* /opt/cincinnati/bin/cincinnati-e2e-test

FROM centos:7

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl https://mirror.openshift.com/pub/openshift-v4/clients/oc/latest/linux/oc.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

COPY --from=builder /opt/cincinnati/bin/cincinnati-e2e-test /usr/bin
COPY --from=builder hack/e2e.sh hack/
COPY --from=builder dist/openshift/cincinnati.yaml dist/openshift/
COPY --from=builder dist/openshift/observability.yaml dist/openshift/

ENTRYPOINT ["hack/e2e.sh"]
