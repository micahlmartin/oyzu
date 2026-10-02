# Explicit provisioned defaults, never installed by an Oyzu build action.
FROM node:22-bookworm-slim
COPY package.json package-lock.json /opt/oyzu-node-quality/
RUN npm ci --prefix /opt/oyzu-node-quality --ignore-scripts --no-audit --no-fund
ENV OYZU_NODE_QUALITY_HOME="/opt/oyzu-node-quality"
