FROM registry.ci.openshift.org/cincinnati-ci-public/rust:boot-root-1.79.0 as builder

COPY . .

RUN cargo build --release

FROM registry.access.redhat.com/ubi9:latest

ENV RUST_LOG=actix_web=error,dkregistry=error

COPY --from=builder /go/target/release/graph-builder \
                    /go/target/release/policy-engine \
                    /go/target/release/metadata-helper \
                    /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
