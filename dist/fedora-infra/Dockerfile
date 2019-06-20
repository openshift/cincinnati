FROM fedora:30

# build: project sources
ADD . /src
WORKDIR /src

# build: system utilities, libraries, system Rust toolchain
RUN dnf -y install g++ openssl-devel rust cargo

# build: graph-builder binary
RUN cargo build --release --bin graph-builder && mv /src/target/release/graph-builder /usr/local/bin/

# build: policy-engine binary
RUN cargo build --release --bin policy-engine && mv /src/target/release/policy-engine /usr/local/bin/

# build: cleanup
RUN rm -rf /src $HOME/.cargo

# run: default config
WORKDIR /
CMD [ "/usr/local/bin/policy-engine" ]
