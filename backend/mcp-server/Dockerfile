# Cream MCP Server — multi-stage Node.js build.
#
# Stage 1 (builder): installs ALL deps (including devDeps for tsc) and runs
# the TypeScript compile to emit dist/.
#
# Stage 2 (runtime): copies dist/ + production node_modules only. The final
# image is slimmer and ships no source files or test deps.
#
# This Dockerfile is standalone-ready. It is not wired into docker-compose.yml
# yet because the Rust API has no companion Dockerfile — that integration
# work is deferred to the deployment-infrastructure phase alongside fly.toml
# and the Rust backend Dockerfile.

FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY tsconfig.json ./
COPY src/ ./src/
RUN npm run build

FROM node:22-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev
COPY --from=builder /app/dist ./dist
ENV NODE_ENV=production
EXPOSE 3002
CMD ["node", "dist/index.js"]
