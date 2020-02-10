FROM cincinnati:build as builder
# build e2e tests
COPY . .

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

# cargo would create test file with a randomized name
COPY --from=builder /opt/cincinnati/bin/cincinnati-e2e-test /usr/bin

ENTRYPOINT ["hack/e2e.sh"]
