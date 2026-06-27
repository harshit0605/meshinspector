"""Standalone database job-queue worker.

Processes the database-backed job queue (decimate / hollow / subdivide / scoop /
make-manufacturable / ...) WITHOUT serving HTTP. Run ONE of these alongside N
stateless uvicorn web workers that set ``DEV_DB_QUEUE_RUNNER_ENABLED=false`` so a
single runner claims jobs — correct on SQLite (one writer) and Postgres alike
(Postgres also tolerates many runners via ``SELECT ... FOR UPDATE SKIP LOCKED``).

Run:  DEV_DB_QUEUE_RUNNER_ENABLED=true python -m workers.run_db_queue
"""

from __future__ import annotations

import signal
import threading

from core.config import settings
from core.db import Base, engine
from core.logging import get_logger, setup_logging
from storage.object_store import object_store
from workers.dev_queue import DatabaseQueueRunner, reconcile_database_queue_state


def main() -> None:
    setup_logging()
    log = get_logger("workers.run_db_queue")

    if not settings.queue_uses_database:
        log.warning("QUEUE_BACKEND=%s is not the database queue; nothing to run.", settings.QUEUE_BACKEND)
        return
    if not settings.DEV_DB_QUEUE_RUNNER_ENABLED:
        log.warning("DEV_DB_QUEUE_RUNNER_ENABLED is false; refusing to start the queue worker.")
        return

    # Mirror the API lifespan's one-time setup so this process can run alone.
    if settings.AUTO_CREATE_SCHEMA and settings.DATABASE_URL.startswith("sqlite"):
        Base.metadata.create_all(bind=engine)
    object_store.ensure_bucket()
    reconcile_database_queue_state()

    runner = DatabaseQueueRunner()
    runner.start()
    log.info("Standalone DB queue worker started; processing jobs. Ctrl+C to stop.")

    stop = threading.Event()
    signal.signal(signal.SIGINT, lambda *_: stop.set())
    signal.signal(signal.SIGTERM, lambda *_: stop.set())
    try:
        stop.wait()
    finally:
        runner.stop()
        log.info("Standalone DB queue worker stopped.")


if __name__ == "__main__":
    main()
