FROM cincinnati:build as builder
# build e2e tests
COPY . .

# Build and copy test binary
RUN cd graph-builder && \
    cargo build --tests --features test-e2e --no-default-features && \
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
COPY --from=builder dist/openshift/cincinnati.yaml template/

ENTRYPOINT ["hack/e2e.sh"]
