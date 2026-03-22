"""Database-backed development queue runner."""

from __future__ import annotations

import threading
import time
from datetime import datetime, timedelta, timezone
from uuid import uuid4

from core.config import settings
from core.db import SessionLocal, engine
from core.logging import get_logger
from domain.models import DevTaskQueueRecord
from sqlalchemy.exc import SQLAlchemyError
from storage.repositories import claim_next_database_task, complete_database_task
from workers.runtime import execute_ingest_task, execute_operation_task

logger = get_logger(__name__)


def _dispose_engine() -> None:
    try:
        engine.dispose()
    except Exception:
        logger.exception("Failed to dispose SQLAlchemy engine during queue recovery")


def _stale_task_cutoff() -> datetime:
    stale_ms = max(settings.DEV_DB_QUEUE_STALE_LOCK_MS, settings.DEV_DB_QUEUE_POLL_INTERVAL_MS)
    return datetime.now(timezone.utc) - timedelta(milliseconds=stale_ms)


def _mark_task_failed(task_id: str, message: str) -> None:
    try:
        with SessionLocal() as db:
            task_record = db.get(DevTaskQueueRecord, task_id)
            if task_record is not None:
                complete_database_task(db, task_record, "failed", message)
            db.commit()
    except SQLAlchemyError as exc:
        logger.exception("Failed to persist failure state for database task %s: %s", task_id, exc)
        _dispose_engine()


def run_database_queue_once(runner_id: str | None = None) -> bool:
    runner = runner_id or f"runner_{uuid4().hex[:8]}"
    try:
        with SessionLocal() as db:
            task = claim_next_database_task(db, runner, stale_before=_stale_task_cutoff())
            db.commit()
    except SQLAlchemyError as exc:
        logger.warning("Database queue claim failed for %s: %s", runner, exc)
        _dispose_engine()
        return False

    if task is None:
        return False

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
    except SQLAlchemyError as exc:
        logger.exception("Database queue task %s hit a database error: %s", task.id, exc)
        if getattr(exc, "connection_invalidated", False):
            _dispose_engine()
            return False
        _mark_task_failed(task.id, str(exc))
        return True
    except Exception as exc:
        logger.exception("Database queue task failed: %s", exc)
        _mark_task_failed(task.id, str(exc))
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
            try:
                for _ in range(max(settings.DEV_DB_QUEUE_BATCH_SIZE, 1)):
                    if not run_database_queue_once(self._runner_id):
                        break
                    processed = True
            except Exception as exc:
                logger.exception("Database queue runner loop crashed: %s", exc)
                _dispose_engine()
            if not processed:
                time.sleep(poll_interval)
