FROM registry.access.redhat.com/ubi9/ubi-minimal:latest as builder
WORKDIR /opt/app-root/src/
COPY . .

RUN microdnf update -y \
    && microdnf install -y jq rust cargo gcc glibc-devel \
    && microdnf install -y openssl-devel \
    && microdnf clean all

RUN cargo build --release --verbose --offline --locked --frozen \
    && mkdir -p /opt/cincinnati/bin \
    && cp -rvf target/release/graph-builder /opt/cincinnati/bin \
    && cp -rvf target/release/policy-engine /opt/cincinnati/bin

FROM registry.access.redhat.com/ubi9/ubi:latest
ENV RUST_LOG=actix_web=error,dkregistry=error
COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]

LABEL description="OpenShift Update Service Operand"
LABEL distribution-scope="public"
LABEL release="5.0.4"
LABEL url="https://github.com/openshift/cincinnati"
LABEL vendor="Red Hat, Inc."
LABEL io.k8s.description="OpenShift Update Service Operand for managing updates"