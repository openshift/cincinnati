FROM registry.access.redhat.com/ubi8/ubi:8.1
RUN dnf install git -y
WORKDIR /home
RUN git clone https://github.com/openshift/cincinnati-graph-data.git graph-data && cd graph-data && git checkout 421646a6e9592b179c26b3c77d406820c479a19c
RUN mkdir -p /var/lib/cincinnati/graph-data
CMD cp -rp graph-data/* /var/lib/cincinnati/graph-data
