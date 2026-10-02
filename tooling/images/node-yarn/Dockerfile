# Explicit provisioning; Oyzu never installs a manager during build execution.
ARG QUALITY_IMAGE=oyzu-toolchain/node:quality
FROM ${QUALITY_IMAGE}
COPY package.json package-lock.json /opt/oyzu-yarn/
RUN npm ci --prefix /opt/oyzu-yarn --ignore-scripts --no-audit --no-fund
ENV PATH="/opt/oyzu-yarn/node_modules/.bin:${PATH}"
