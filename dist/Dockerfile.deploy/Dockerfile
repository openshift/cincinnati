FROM registry.access.redhat.com/ubi9/ubi-minimal:latest as builder
WORKDIR /opt/app-root/src/
COPY . .

RUN microdnf update -y \
    && microdnf install -y jq rust cargo \
    && microdnf install -y openssl-devel \
    && microdnf clean all \
    && cargo build --release \
    && mkdir -p /opt/cincinnati/bin \
    && cp -rvf target/release/graph-builder /opt/cincinnati/bin \
    && cp -rvf target/release/policy-engine /opt/cincinnati/bin \
    && cp -rvf target/release/metadata-helper /opt/cincinnati/bin

FROM registry.access.redhat.com/ubi9/ubi-minimal:latest
ENV RUST_LOG=actix_web=error,dkregistry=error
COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
