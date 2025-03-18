FROM registry.access.redhat.com/ubi9:latest as builder


# build: system utilities and libraries
RUN yum update -y && \
    yum -y install gcc openssl-devel && \
    yum -y install cmake libcurl-devel && \
    yum clean all

ENV HOME="/root"
ENV PATH="${HOME}/.cargo/bin:${PATH}"

WORKDIR ${HOME}

# build: Rust stable toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.76.0 -y && \
  rustup install 1.76.0

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;

COPY . .

RUN cargo build --release && \
    mkdir -p /opt/cincinnati/bin && \
    cp -rvf $HOME/target/release/graph-builder /opt/cincinnati/bin && \
    cp -rvf $HOME/target/release/policy-engine /opt/cincinnati/bin && \
    cp -rvf $HOME/target/release/metadata-helper /opt/cincinnati/bin 

FROM registry.access.redhat.com/ubi9:latest

ENV RUST_LOG=actix_web=error,dkregistry=error

RUN yum update -y && \
    yum install -y openssl && \
    yum clean all

COPY --from=builder /opt/cincinnati/bin/* /usr/bin/

ENTRYPOINT ["/usr/bin/graph-builder"]
