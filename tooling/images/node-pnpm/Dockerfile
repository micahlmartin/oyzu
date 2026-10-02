# Explicit provisioning; Oyzu never installs a manager during build execution.
FROM oyzu-toolchain/node:quality
COPY package.json package-lock.json /opt/oyzu-pnpm/
RUN npm ci --prefix /opt/oyzu-pnpm --ignore-scripts --no-audit --no-fund
ENV PATH="/opt/oyzu-pnpm/node_modules/.bin:${PATH}"
