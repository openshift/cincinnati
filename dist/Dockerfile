FROM centos:7

ENV RUST_LOG=actix_web=error,dkregistry=error

COPY graph-builder policy-engine /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
