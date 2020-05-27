# The WORKDIR in rust-toolset is /opt/app-root/src/
FROM rhel8/rust-toolset:1.41.1 as builder
COPY . .
COPY .git/ ./.git/
COPY .cargo/ .cargo/
RUN cargo build --offline --release && \
    mkdir artifacts && \
    cp -L --reflink=never target/release/{graph-builder,policy-engine} artifacts

FROM ubi8/ubi-minimal:8.2
COPY --from=builder /opt/app-root/src/artifacts/graph-builder /usr/local/bin/
COPY --from=builder /opt/app-root/src/artifacts/policy-engine /usr/local/bin/
CMD ["/usr/local/bin/graph-builder"]

LABEL com.redhat.component="cincinnati-container" \
    name="cincinnati/cincinnati" \
    version="v4.5.0" \
    summary="OpenShift Cincinnati" \
    io.openshift.expose-services="" \
    io.openshift.tags="cincinnati,upgrade,update" \
    io.k8s.display-name="cincinnati" \
    maintainer="Lalatendu Mohanty <lmohanty@redhat.com>, Stefan Junker <sjunker@redhat.com>" \
    description="OpenShift Cincinnati Operator"
