# Explicit CI provisioning, outside Oyzu build execution and tool installation.
ARG QUALITY_IMAGE=oyzu-toolchain/node:quality
FROM ${QUALITY_IMAGE}
COPY package.json package-lock.json /opt/oyzu-npm/
RUN npm ci --prefix /opt/oyzu-npm --ignore-scripts --no-audit --no-fund \
    && /opt/oyzu-npm/node_modules/.bin/npm --version
ENV PATH="/opt/oyzu-npm/node_modules/.bin:${PATH}"
