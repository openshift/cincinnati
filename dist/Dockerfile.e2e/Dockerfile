FROM registry.centos.org/centos/centos:7 as rust_builder

# base: EPEL repo for extra tools
RUN yum -y install epel-release

# build: system utilities and libraries
RUN yum update -y && \
    yum -y groupinstall 'Development Tools' && \
    yum -y install gcc openssl-devel protobuf-compiler jq skopeo buildah libgit2 && \
    yum -y install yamllint && \
    yum -y install cmake elfutils-libelf-devel libcurl-devel binutils-devel elfutils-devel && \
    yum clean all

ENV HOME="/root"
ENV PATH="${HOME}/.cargo/bin:${PATH}"

# build: Rust stable toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.53.0 -y && \
  rustup install 1.49.0

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;


RUN hack/build_e2e.sh

FROM registry.ci.openshift.org/openshift/release:golang-1.13 AS golang_builder
RUN curl -L https://github.com/tsenart/vegeta/releases/download/v12.8.4/vegeta_12.8.4_linux_amd64.tar.gz| tar xvzf - -C /usr/local/bin/ vegeta

FROM registry.centos.org/centos/centos:7

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl https://mirror.openshift.com/pub/openshift-v4/clients/oc/latest/linux/oc.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

# Install container tools
RUN yum update -y && \
    yum install -y skopeo buildah && \
    yum clean all

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/hack/e2e.sh hack/
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/dist/openshift/cincinnati.yaml dist/openshift/
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/dist/openshift/observability.yaml dist/openshift/
COPY --from=golang_builder /usr/local/bin/vegeta /usr/bin
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/hack/load-testing.sh /usr/local/bin/load-testing.sh
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/hack/vegeta.targets vegeta.targets
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/e2e/tests/testdata e2e/tests/testdata
COPY --from=rust_builder /go/src/github.com/openshift/cincinnati/dist/prepare_ci_credentials.sh dist/

ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
