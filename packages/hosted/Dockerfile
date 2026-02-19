FROM node:20-slim AS builder

WORKDIR /app

COPY package*.json ./
COPY packages/core/ packages/core/
COPY packages/app/ packages/app/
COPY packages/hosted/ packages/hosted/

RUN npm install --workspace=packages/app --workspace=packages/hosted
RUN npm run build --workspace=packages/app

# ─── Production image ────────────────────────────────────────────────────────

FROM node:20-slim

RUN apt-get update && apt-get install -y --no-install-recommends tini && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN groupadd --gid 1001 vault && useradd --uid 1001 --gid vault --create-home vault

COPY package*.json ./
COPY packages/core/ packages/core/
COPY packages/hosted/ packages/hosted/
RUN npm install --workspace=packages/hosted --omit=dev

# Copy built frontend from builder stage
COPY --from=builder /app/packages/app/dist/ packages/app/dist/

RUN mkdir -p /data && chown vault:vault /data

USER vault

EXPOSE 3000

ENV NODE_ENV=production

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD node -e "fetch('http://localhost:3000/health').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

ENTRYPOINT ["tini", "--"]
CMD ["node", "packages/hosted/src/index.js"]
