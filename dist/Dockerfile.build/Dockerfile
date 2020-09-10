FROM centos:7

# base: EPEL repo for extra tools
RUN yum -y install epel-release

# build: system utilities and libraries
RUN yum -y groupinstall 'Development Tools'
RUN yum -y install openssl-devel protobuf-compiler jq

ENV HOME="/root"
ENV PATH="${HOME}/.cargo/bin:${PATH}"

# build: Rust stable toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.46.0 -y

# test: linters
RUN yum -y install yamllint

# test: code coverage
RUN yum -y install cmake elfutils-libelf-devel libcurl-devel binutils-devel elfutils-devel
RUN \
  mkdir $HOME/kcov && \
  pushd $HOME/kcov && \
  curl https://codeload.github.com/SimonKagstrom/kcov/tar.gz/v36 | tar -xz --strip-components=1 && \
  cmake . && make && make install

RUN \
  mkdir -p $HOME/.cargo/git/ && \
  find $HOME/. -type d -exec chmod 777 {} \; && \
  find $HOME/. -type f -exec chmod ugo+rw {} \;
