# Explicit provisioning; Oyzu never installs a manager during build execution.
ARG QUALITY_IMAGE=oyzu-toolchain/node:quality
FROM ${QUALITY_IMAGE}
COPY package.json package-lock.json /opt/oyzu-pnpm/
RUN npm ci --prefix /opt/oyzu-pnpm --ignore-scripts --no-audit --no-fund
ENV PATH="/opt/oyzu-pnpm/node_modules/.bin:${PATH}"
