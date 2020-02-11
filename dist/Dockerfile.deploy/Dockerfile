FROM cincinnati:build as builder
# build
COPY . .
RUN cargo build --release && \
    mkdir -p /opt/cincinnati/bin && \
    cp -rvf target/release/graph-builder /opt/cincinnati/bin && \
    cp -rvf target/release/policy-engine /opt/cincinnati/bin

FROM centos:7

ENV RUST_LOG=actix_web=error,dkregistry=error

RUN yum update -y && \
    yum install -y openssl && \
    yum clean all

COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
