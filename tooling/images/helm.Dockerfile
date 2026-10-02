# Explicit CI provisioning. The build CLI never downloads this tool.
FROM python:3.12-slim-bookworm
COPY tooling/provision-yamlfmt.py /tmp/provision-yamlfmt.py
RUN python /tmp/provision-yamlfmt.py --destination /opt/oyzu-yamlfmt \
    && ln -s /opt/oyzu-yamlfmt/yamlfmt /usr/local/bin/yamlfmt
ADD https://get.helm.sh/helm-v3.22.0-linux-amd64.tar.gz /tmp/helm.tar.gz
RUN echo '1e4ab49e429626cf6c6958d914248b78c9730803c2751b87627e171dc800e7bb  /tmp/helm.tar.gz' | sha256sum --check \
    && tar -xzf /tmp/helm.tar.gz -C /tmp linux-amd64/helm \
    && mv /tmp/linux-amd64/helm /usr/local/bin/helm \
    && rm /tmp/helm.tar.gz
ADD https://github.com/helm-unittest/helm-unittest/releases/download/v1.2.0/helm-unittest-linux-amd64-1.2.0.tgz /tmp/unittest.tgz
RUN echo '115c690234847d316f0a814beb9cceaf9c21bb407173179c0e82076bdce1efc0  /tmp/unittest.tgz' | sha256sum --check \
    && mkdir -p /opt/oyzu-helm-plugins/unittest \
    && tar -xzf /tmp/unittest.tgz -C /opt/oyzu-helm-plugins/unittest \
    && chmod +x /opt/oyzu-helm-plugins/unittest/untt-linux-amd64 \
    && rm /tmp/unittest.tgz
ENV HELM_PLUGINS="/opt/oyzu-helm-plugins"
