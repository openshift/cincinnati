FROM centos:7 as builder

RUN yum -y groupinstall 'Development Tools'
RUN yum -y install openssl-devel

ADD https://static.rust-lang.org/dist/rust-1.31.0-x86_64-unknown-linux-gnu.tar.gz rust.tar.gz
RUN tar -xf rust.tar.gz --strip 1
RUN ./install.sh

ENV HOME="/root"

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;
