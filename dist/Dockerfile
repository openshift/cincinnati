FROM centos

RUN yum -y groupinstall 'Development Tools'
RUN yum -y install openssl-devel

ADD https://static.rust-lang.org/dist/rust-1.29.0-x86_64-unknown-linux-gnu.tar.gz rust.tar.gz
RUN tar -xf rust.tar.gz --strip 1
RUN ./install.sh

RUN mkdir app
WORKDIR app
COPY . .
RUN cargo build --release

FROM centos
ENV RUST_LOG=actix_web=error
ENTRYPOINT ["/usr/bin/graph-builder"]
COPY --from=0 /app/target/release/graph-builder /app/target/release/policy-engine /usr/bin/
