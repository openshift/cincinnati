FROM clux/muslrust:1.31.0-stable

RUN \
  mkdir -p /root/.cargo/git/ && \
  find /root/ -type d -exec chmod 777 {} \; && \
  find /root/ -type f -exec chmod ugo+rw {} \;
