# syntax=docker/dockerfile:1.7
###############################################################################
# MeshInspector backend — FastAPI + Rust/PyO3 geometry kernel.
#
# The Rust extension (geometry_sdk/_zennah_geometry_rs.abi3.so) is MANDATORY and
# is compiled inside this image for linux/amd64 (the committed .so is gitignored
# and macOS/arm64). meshlib ships glibc manylinux wheels only, so the base is
# Debian (NOT Alpine).
#
# One image, three roles (selected by the ROLE env via docker-entrypoint.sh):
#   ROLE=web      -> uvicorn API server (default)
#   ROLE=worker   -> uvicorn + in-process DB queue runner (background jobs)
#   ROLE=migrate  -> alembic upgrade head (one-shot)
###############################################################################

# ---------------------------------------------------------------------------
# Stage 1: builder — Python deps (uv) + compile the Rust geometry extension
# ---------------------------------------------------------------------------
FROM python:3.12-slim-bookworm AS builder

ENV DEBIAN_FRONTEND=noninteractive \
    PIP_DISABLE_PIP_VERSION_CHECK=1 \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    VIRTUAL_ENV=/app/.venv \
    UV_LINK_MODE=copy \
    PATH=/usr/local/cargo/bin:/app/.venv/bin:/root/.local/bin:$PATH

# C/C++ toolchain (blosc-src compiles C), cmake, git, curl for rustup
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential cmake pkg-config git curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain — use latest stable. The workspace declares MSRV 1.82, but some
# transitive deps require Cargo's edition2024 feature (stabilized in Rust 1.85), so
# an older pin fails `cargo metadata`.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal

# uv (fast, lockfile-driven Python installer)
COPY --from=ghcr.io/astral-sh/uv:0.5 /uv /usr/local/bin/uv

WORKDIR /app

# 1) Resolve + install Python deps first (cached unless pyproject/lock change)
COPY pyproject.toml uv.lock .python-version ./
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-dev --no-install-project
# maturin is the PyO3 build backend; install it into the project venv
RUN --mount=type=cache,target=/root/.cache/uv \
    uv pip install "maturin>=1.8,<2"

# 2) Copy the source and compile the Rust kernel into geometry_sdk/
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/geometry-rs/target \
    cd geometry-rs && maturin develop --release \
    && test -f /app/geometry_sdk/_zennah_geometry_rs.abi3.so

# ---------------------------------------------------------------------------
# Stage 2: runtime — slim image with the venv + source + built .so
# ---------------------------------------------------------------------------
FROM python:3.12-slim-bookworm AS runtime

ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    VIRTUAL_ENV=/app/.venv \
    PATH=/app/.venv/bin:$PATH \
    HOST=0.0.0.0 \
    PORT=8080

# Shared libs meshlib's native wheel needs at runtime (OpenMP + headless GL/font stack)
RUN apt-get update && apt-get install -y --no-install-recommends \
        libgomp1 libgl1 libglu1-mesa libxrender1 libxext6 libsm6 fontconfig \
        ca-certificates curl tini \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN useradd --create-home --uid 10001 appuser

# venv (deps), app source, and the compiled extension — all built in stage 1.
# --chown on COPY avoids a second full-size image layer that a post-copy
# `chown -R /app` would create (it rewrites every file).
COPY --chown=appuser:appuser --from=builder /app /app

RUN chmod +x /app/docker-entrypoint.sh
USER appuser

EXPOSE 8080
# tini reaps zombies / forwards signals for clean Cloud Run shutdowns
ENTRYPOINT ["tini", "--", "/app/docker-entrypoint.sh"]
