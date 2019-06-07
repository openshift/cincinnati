FROM centos:7

# base: EPEL repo for extra tools
RUN yum -y install epel-release

# build: system utilities and libraries
RUN yum -y groupinstall 'Development Tools'
RUN yum -y install openssl-devel

ENV HOME="/root"

# build: Rust stable toolchain
RUN \
    mkdir $HOME/rust && \
    curl https://static.rust-lang.org/dist/rust-1.34.2-x86_64-unknown-linux-gnu.tar.gz | \
    tar -xzvf - -C $HOME/rust --strip 1 && \
    $HOME/rust/install.sh; \
    rm -rf $HOME/rust

# test: linters
RUN yum -y install yamllint

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;
