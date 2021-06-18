FROM registry.centos.org/centos/centos:7 as builder

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

WORKDIR ${HOME}

# build: Rust stable toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.53.0 -y && \
  rustup install 1.49.0

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;

COPY . .
# copy git information for built crate
COPY .git/ ./.git/

RUN cargo build --release && \
    mkdir -p /opt/cincinnati/bin && \
    cp -rvf $HOME/target/release/graph-builder /opt/cincinnati/bin && \
    cp -rvf $HOME/target/release/policy-engine /opt/cincinnati/bin

FROM registry.centos.org/centos/centos:7

ENV RUST_LOG=actix_web=error,dkregistry=error

RUN yum update -y && \
    yum install -y openssl libgit2 && \
    yum clean all

COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
