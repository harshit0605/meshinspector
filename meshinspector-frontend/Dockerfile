# syntax=docker/dockerfile:1.7
###############################################################################
# MeshInspector frontend — Next.js 16 (standalone output).
#
# NOTE: NEXT_PUBLIC_API_URL is BUILD-TIME (baked into the client bundle). It must
# be passed as a --build-arg pointing at the public backend API URL. Changing it
# later requires a rebuild.
#
# The 37 MB public/meshlib-workbench/ tree (incl. the .wasm viewer) is served as
# same-origin static files and is copied explicitly into the runner (Next's
# standalone output does NOT include public/).
###############################################################################

# ---------------------------------------------------------------------------
# Stage 1: install dependencies (cached on package*.json)
# ---------------------------------------------------------------------------
FROM node:20-slim AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci

# ---------------------------------------------------------------------------
# Stage 2: build (next build downloads Google fonts -> needs network)
# ---------------------------------------------------------------------------
FROM node:20-slim AS builder
WORKDIR /app
ARG NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL} \
    NEXT_TELEMETRY_DISABLED=1 \
    NODE_ENV=production
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN test -n "$NEXT_PUBLIC_API_URL" \
      || echo "WARNING: NEXT_PUBLIC_API_URL not set; client will fall back to http://localhost:8000" >&2
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 3: minimal runtime (Next standalone server)
# ---------------------------------------------------------------------------
FROM node:20-slim AS runner
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=8080 \
    HOSTNAME=0.0.0.0

# public/ (incl. meshlib-workbench) is NOT bundled by standalone — copy it in.
COPY --from=builder /app/public ./public
# Standalone server + its trimmed node_modules, then the static chunks.
COPY --from=builder --chown=node:node /app/.next/standalone ./
COPY --from=builder --chown=node:node /app/.next/static ./.next/static

USER node
EXPOSE 8080
CMD ["node", "server.js"]
