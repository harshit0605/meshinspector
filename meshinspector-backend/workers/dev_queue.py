"""Database-backed development queue runner."""

from __future__ import annotations

import threading
import time
from uuid import uuid4

from core.config import settings
from core.db import SessionLocal
from core.logging import get_logger
from domain.models import DevTaskQueueRecord
from storage.repositories import claim_next_database_task, complete_database_task
from workers.runtime import execute_ingest_task, execute_operation_task

logger = get_logger(__name__)


def run_database_queue_once(runner_id: str | None = None) -> bool:
    runner = runner_id or f"runner_{uuid4().hex[:8]}"
    with SessionLocal() as db:
        task = claim_next_database_task(db, runner)
        if task is None:
            db.commit()
            return False
        db.commit()

    try:
        if task.task_name == "ingest_model":
            with SessionLocal() as db:
                execute_ingest_task(
                    db,
                    task.payload_json["model_id"],
                    task.payload_json["version_id"],
                    task.payload_json["job_id"],
                    task.payload_json["source_path"],
                )
                task_record = db.get(DevTaskQueueRecord, task.id)
                if task_record is not None:
                    complete_database_task(db, task_record, "succeeded")
                db.commit()
            return True

        if task.task_name == "run_operation":
            with SessionLocal() as db:
                execute_operation_task(
                    db,
                    task.payload_json["operation_type"],
                    task.payload_json["source_version_id"],
                    task.payload_json["job_id"],
                    task.payload_json["payload"],
                )
                task_record = db.get(DevTaskQueueRecord, task.id)
                if task_record is not None:
                    complete_database_task(db, task_record, "succeeded")
                db.commit()
            return True

        with SessionLocal() as db:
            task_record = db.get(DevTaskQueueRecord, task.id)
            if task_record is not None:
                complete_database_task(db, task_record, "failed", f"Unsupported task: {task.task_name}")
            db.commit()
        return True
    except Exception as exc:
        logger.exception("Database queue task failed: %s", exc)
        with SessionLocal() as db:
            task_record = db.get(DevTaskQueueRecord, task.id)
            if task_record is not None:
                complete_database_task(db, task_record, "failed", str(exc))
            db.commit()
        return True


class DatabaseQueueRunner:
    def __init__(self) -> None:
        self._runner_id = f"runner_{uuid4().hex[:8]}"
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        if self._thread and self._thread.is_alive():
            return
        self._thread = threading.Thread(target=self._loop, name="meshinspector-db-queue", daemon=True)
        self._thread.start()
        logger.info("Started database queue runner %s", self._runner_id)

    def stop(self) -> None:
        self._stop.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=5)

    def _loop(self) -> None:
        poll_interval = max(settings.DEV_DB_QUEUE_POLL_INTERVAL_MS, 100) / 1000.0
        while not self._stop.is_set():
            processed = False
            for _ in range(max(settings.DEV_DB_QUEUE_BATCH_SIZE, 1)):
                if not run_database_queue_once(self._runner_id):
                    break
                processed = True
            if not processed:
                time.sleep(poll_interval)
