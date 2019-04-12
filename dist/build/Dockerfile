FROM clux/muslrust:1.34.0-stable

RUN \
  mkdir -p /root/.cargo/git/ && \
  mkdir -p /root/.cargo/registry/ && \
  find /root/. -type d -exec chmod 777 {} \; && \
  find /root/. -type f -exec chmod ugo+rw {} \;
