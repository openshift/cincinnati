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
RUN mkdir /licenses
COPY --from=builder /opt/app-root/src/LICENSE /licenses/LICENSE
USER 65532

ENTRYPOINT ["/usr/bin/graph-builder"]

LABEL description="OpenShift Update Service Operand"
LABEL distribution-scope="public"
LABEL release="5.0.5"
LABEL url="https://github.com/openshift/cincinnati"
LABEL vendor="Red Hat, Inc."
LABEL io.k8s.description="OpenShift Update Service Operand for managing updates"
LABEL com.redhat.component="Red Hat Openshift Update Service"
LABEL io.k8s.display-name="Red Hat OpenShift Update Service Operand"
LABEL name="Red Hat OpenShift Update Service"
LABEL summary="Red Hat OpenShift Update Service Operand"
LABEL io.openshift.tags="openshift,update,service,operand" 