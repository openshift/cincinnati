FROM centos:7

RUN yum -y groupinstall 'Development Tools'
RUN yum -y install openssl-devel

ENV RUST_LOG=actix_web=error,dkregistry=error

COPY target/x86_64-unknown-linux-musl/release/graph-builder target/x86_64-unknown-linux-musl/release/policy-engine /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
