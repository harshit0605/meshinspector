#!/usr/bin/env bash
# Run the MeshInspector backend with MULTIPLE uvicorn web workers + one dedicated
# job-queue worker.
#
#   - N stateless web workers (uvicorn --workers N) serve HTTP. Heavy synchronous
#     voxel/offset/boolean ops run in each worker's thread pool (run_in_threadpool),
#     so a heavy op never blocks the event loop, and one busy worker no longer
#     freezes the whole backend.
#   - ONE queue worker processes the database job queue (decimate / hollow /
#     subdivide / scoop / make-manufacturable / ...). The web workers run with the
#     per-process queue runner DISABLED so jobs are claimed by a single runner —
#     correct on SQLite (one writer) and Postgres (also fine with many via
#     SELECT ... FOR UPDATE SKIP LOCKED).
#
# Env (all optional):
#   WORKERS  number of web workers   (default: CPU-based, min 2)
#   HOST     bind host               (default: 127.0.0.1)
#   PORT     bind port               (default: 48100)
#   DATABASE_URL / DIRECT_URL  passed through (Postgres in prod; sqlite for dev)
#
# Example:  WORKERS=4 PORT=8000 DATABASE_URL=postgresql://... ./run-backend.sh
set -euo pipefail
cd "$(dirname "$0")"

PY="${PY:-.venv/bin/python}"
UVICORN="${UVICORN:-.venv/bin/uvicorn}"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-48100}"
if [ -z "${WORKERS:-}" ]; then
  CORES="$("$PY" -c 'import os;print(os.cpu_count() or 2)')"
  WORKERS="$(( CORES > 2 ? CORES - 1 : 2 ))"
fi

echo "Starting backend: ${WORKERS} web workers on ${HOST}:${PORT} + 1 queue worker"

# 1) one dedicated job-queue worker (heavy job ops run here, off the web tier)
DEV_DB_QUEUE_RUNNER_ENABLED=true "$PY" -m workers.run_db_queue &
QUEUE_PID=$!
cleanup() { kill "$QUEUE_PID" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

# 2) N web workers, per-worker queue runner disabled (single claimer = the worker above)
DEV_DB_QUEUE_RUNNER_ENABLED=false "$UVICORN" main:app \
  --host "$HOST" --port "$PORT" --workers "$WORKERS"
