FROM registry.access.redhat.com/ubi8/ubi:latest as builder
WORKDIR /opt/app-root/src/
COPY . .
# copy git information for built crate
COPY .git/ ./.git/
USER 0
RUN dnf update -y \
    && dnf install -y jq rust cargo \
    && dnf install -y openssl-devel \
    && dnf clean all \
    && cargo build --release \
    && mkdir -p /opt/cincinnati/bin \
    && cp -rvf target/release/graph-builder /opt/cincinnati/bin \
    && cp -rvf target/release/policy-engine /opt/cincinnati/bin \
    && cp -rvf target/release/metadata-helper /opt/cincinnati/bin

FROM registry.access.redhat.com/ubi8/ubi:latest
ENV RUST_LOG=actix_web=error,dkregistry=error
COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
