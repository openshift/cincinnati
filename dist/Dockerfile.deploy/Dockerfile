FROM centos:7

ENV RUST_LOG=actix_web=error,dkregistry=error

RUN yum update -y && \
    yum install -y openssl && \
    yum clean all

COPY graph-builder policy-engine /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
