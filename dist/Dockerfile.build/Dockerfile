FROM registry.centos.org/centos/centos:7

# base: EPEL repo for extra tools
RUN yum -y install epel-release

# build: system utilities and libraries
RUN yum update -y && \
    yum -y install epel-release && \
    yum -y groupinstall 'Development Tools' && \
    yum -y install openssl-devel protobuf-compiler jq skopeo buildah && \
    yum -y install yamllint && \
    yum -y install cmake elfutils-libelf-devel libcurl-devel binutils-devel elfutils-devel && \
    yum clean all

ENV HOME="/root"
ENV PATH="${HOME}/.cargo/bin:${PATH}"

# build: Rust stable toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.50.0 -y && \
  rustup install 1.46.0

# test: code coverage
RUN \
  mkdir $HOME/kcov && \
  pushd $HOME/kcov && \
  curl https://codeload.github.com/SimonKagstrom/kcov/tar.gz/v36 | tar -xz --strip-components=1 && \
  cmake . && make && make install

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;
