FROM quay.io/app-sre/busybox as downloader
ADD https://github.com/openshift/cincinnati-graph-data/archive/e8692fe50ccbfa525cce340f781d56d5a1d4364d.tar.gz /graph-data.tar.gz
RUN mkdir -p /graph-data
RUN tar xav -C /graph-data -f /graph-data.tar.gz --no-same-owner

FROM scratch
COPY --from=downloader /graph-data/* /
