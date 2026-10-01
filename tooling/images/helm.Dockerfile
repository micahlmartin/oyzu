# Explicit CI provisioning. The build CLI never downloads this tool.
FROM python:3.12-slim-bookworm
ADD https://get.helm.sh/helm-v3.22.0-linux-amd64.tar.gz /tmp/helm.tar.gz
RUN echo '1e4ab49e429626cf6c6958d914248b78c9730803c2751b87627e171dc800e7bb  /tmp/helm.tar.gz' | sha256sum --check \
    && tar -xzf /tmp/helm.tar.gz -C /tmp linux-amd64/helm \
    && mv /tmp/linux-amd64/helm /usr/local/bin/helm \
    && rm /tmp/helm.tar.gz
