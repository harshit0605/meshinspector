#!/usr/bin/env bash
# Entrypoint for the MeshInspector backend image.
# Selects the process role and applies Cloud-Run-safe defaults (writable /tmp dirs,
# bind 0.0.0.0:$PORT). Override any of these via environment variables.
set -euo pipefail

export HOST="${HOST:-0.0.0.0}"
export PORT="${PORT:-8080}"

# Cloud Run's filesystem is read-only except /tmp. config.py creates these dirs at
# import, so repoint them to /tmp unless the deployment overrides them (e.g. a volume).
export DATA_DIR="${DATA_DIR:-/tmp/data}"
export TEMP_DIR="${TEMP_DIR:-/tmp/temp}"
export STORAGE_DIR="${STORAGE_DIR:-/tmp/storage}"
export MODELS_DIR="${MODELS_DIR:-/tmp/models}"
export UPLOAD_DIR="${UPLOAD_DIR:-/tmp/uploads}"

ROLE="${ROLE:-web}"

case "$ROLE" in
  web)
    # API server. Does NOT claim queue jobs (the worker service does).
    export DEV_DB_QUEUE_RUNNER_ENABLED="${DEV_DB_QUEUE_RUNNER_ENABLED:-false}"
    exec uvicorn main:app --host "$HOST" --port "$PORT" --workers "${WEB_CONCURRENCY:-1}"
    ;;
  worker)
    # Background job processor: serves /health on $PORT (so it is a valid Cloud Run
    # service) and runs the in-process DatabaseQueueRunner thread.
    export QUEUE_BACKEND="${QUEUE_BACKEND:-database}"
    export DEV_DB_QUEUE_RUNNER_ENABLED="${DEV_DB_QUEUE_RUNNER_ENABLED:-true}"
    exec uvicorn main:app --host "$HOST" --port "$PORT" --workers 1
    ;;
  migrate)
    # One-shot schema migration (Cloud Run Job).
    exec alembic upgrade head
    ;;
  *)
    # Escape hatch: run an arbitrary command (e.g. a shell for debugging).
    exec "$@"
    ;;
esac
