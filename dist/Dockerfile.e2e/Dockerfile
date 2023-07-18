FROM quay.io/centos/centos:stream9 as rust_builder

# base: EPEL repo for extra tools
RUN dnf -y install epel-release

# build: system utilities and libraries
RUN dnf update -y && \
    dnf -y groupinstall 'Development Tools' && \
    dnf -y install gcc openssl-devel jq skopeo buildah libgit2 && \
    dnf -y install yamllint && \
    dnf -y install cmake elfutils-libelf-devel libcurl-devel binutils-devel elfutils-devel && \
    dnf clean all

ENV HOME="/root"
ENV PATH="${HOME}/.cargo/bin:${PATH}"

# build: Rust stable toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.70.0 -y && \
  rustup install 1.70.0

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;

WORKDIR /opt/app-root/src/
COPY . .
COPY .git .git
RUN hack/build_e2e.sh

FROM quay.io/centos/centos:stream9 AS vegeta_fetcher
RUN curl -L https://github.com/tsenart/vegeta/releases/download/v12.8.4/vegeta_12.8.4_linux_amd64.tar.gz| tar xvzf - -C /usr/local/bin/ vegeta

FROM quay.io/centos/centos:stream9

ENV HOME="/root"

RUN mkdir -p "${HOME}/cincinnati"
WORKDIR "${HOME}/cincinnati"

# Get oc CLI
RUN mkdir -p ${HOME}/bin && \
    curl -L https://mirror.openshift.com/pub/openshift-v4/clients/ocp/stable/openshift-client-linux.tar.gz 2>/dev/null | tar xzf - -C "${HOME}/bin/" oc
ENV PATH="${PATH}:${HOME}/bin"

# Install container tools
RUN yum update -y && \
    yum install -y skopeo buildah && \
    yum clean all

COPY --from=rust_builder /opt/cincinnati/bin/e2e /usr/bin/cincinnati-e2e-test
COPY --from=rust_builder /opt/cincinnati/bin/prometheus_query /usr/bin/cincinnati-prometheus_query-test
COPY --from=rust_builder /opt/cincinnati/bin/slo /usr/bin/cincinnati-e2e-slo
COPY --from=rust_builder /opt/app-root/src/hack/e2e.sh hack/
COPY --from=rust_builder /opt/app-root/src/hack/vegeta.targets hack/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/cincinnati.yaml dist/openshift/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/cincinnati-e2e.yaml dist/openshift/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/observability.yaml dist/openshift/
COPY --from=rust_builder /opt/app-root/src/dist/openshift/load-testing.yaml dist/openshift/
COPY --from=vegeta_fetcher /usr/local/bin/vegeta /usr/bin
COPY --from=rust_builder /opt/app-root/src/e2e/tests/testdata e2e/tests/testdata
COPY --from=rust_builder /opt/app-root/src/dist/prepare_ci_credentials.sh dist/

ENV E2E_TESTDATA_DIR "e2e/tests/testdata"

ENTRYPOINT ["hack/e2e.sh"]
