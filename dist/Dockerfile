FROM centos:7

RUN yum -y groupinstall 'Development Tools'
RUN yum -y install openssl-devel

ENV RUST_LOG=actix_web=error,dkregistry=error

COPY graph-builder policy-engine /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
