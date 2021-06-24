FROM registry.redhat.io/rhel8/rust-toolset:1.47.0 as builder
WORKDIR /opt/app-root/src/
COPY . .
# copy git information for built crate
COPY .git/ ./.git/

USER 0
RUN bash -c "source /opt/app-root/etc/scl_enable && cargo build --release" \
    && mkdir -p /opt/cincinnati/bin \
    && cp -rvf target/release/graph-builder /opt/cincinnati/bin \
    && cp -rvf target/release/policy-engine /opt/cincinnati/bin

FROM registry.access.redhat.com/ubi8/ubi:latest
ENV RUST_LOG=actix_web=error,dkregistry=error
COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
