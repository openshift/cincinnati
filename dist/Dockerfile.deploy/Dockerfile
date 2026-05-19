FROM quay.io/openshift-ota/cincinnati-ci-public:rust_boot-root-1.84.1 as builder
COPY . .

RUN bin_folder="/go/bin" just build --release

FROM registry.access.redhat.com/ubi9/ubi-minimal:latest
ENV RUST_LOG=actix_web=error,dkregistry=error
COPY --from=builder /go/bin/graph-builder /go/bin/policy-engine /go/bin/metadata-helper /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
