# Explicit provisioning; Oyzu never installs a manager during build execution.
FROM node:22-bookworm-slim
COPY package.json package-lock.json /opt/oyzu-yarn/
RUN npm ci --prefix /opt/oyzu-yarn --ignore-scripts --no-audit --no-fund
ENV PATH="/opt/oyzu-yarn/node_modules/.bin:${PATH}"
