"""Database-backed development queue runner."""

from __future__ import annotations

import threading
import time
from datetime import datetime, timedelta, timezone
from uuid import uuid4

from sqlalchemy import select
from core.config import settings
from core.db import SessionLocal, engine
from core.logging import get_logger
from domain.models import DevTaskQueueRecord, JobRecord
from sqlalchemy.exc import SQLAlchemyError
from storage.repositories import add_job_event, claim_next_database_task, complete_database_task, create_database_task, set_job_status
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


def _normalize_utc(value: datetime | None) -> datetime | None:
    if value is None:
        return None
    if value.tzinfo is None:
        return value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def _mark_task_failed(task_id: str, message: str) -> None:
    try:
        with SessionLocal() as db:
            task_record = db.get(DevTaskQueueRecord, task_id)
            if task_record is not None:
                complete_database_task(db, task_record, "failed", message)
                _fail_linked_job(db, task_record, message)
            db.commit()
    except SQLAlchemyError as exc:
        logger.exception("Failed to persist failure state for database task %s: %s", task_id, exc)
        _dispose_engine()


def _fail_linked_job(db, task_record: DevTaskQueueRecord, message: str) -> None:
    if not task_record.job_id:
        return
    job = db.get(JobRecord, task_record.job_id)
    if job is None or job.status in {"succeeded", "failed"}:
        return
    set_job_status(
        db,
        job,
        "failed",
        error_code="QUEUE_TASK_FAILED",
        error_message=message,
    )
    add_job_event(db, job.id, f"Queue task failed before runtime completion: {message}", level="error")
    if job.operation_type == "ingest":
        job.version.status = "failed"


def _heartbeat_task_lock(task_id: str, runner_id: str, stop_event: threading.Event) -> None:
    heartbeat_interval = max(
        min(settings.DEV_DB_QUEUE_STALE_LOCK_MS // 4, 30_000),
        settings.DEV_DB_QUEUE_POLL_INTERVAL_MS,
        1_000,
    ) / 1000.0
    while not stop_event.wait(heartbeat_interval):
        try:
            with SessionLocal() as db:
                task_record = db.get(DevTaskQueueRecord, task_id)
                if task_record is None or task_record.status != "running" or task_record.locked_by != runner_id:
                    return
                task_record.locked_at = datetime.now(timezone.utc)
                db.commit()
        except SQLAlchemyError as exc:
            logger.warning("Database queue heartbeat failed for task %s: %s", task_id, exc)
            _dispose_engine()
            return


def _reset_orphaned_running_tasks() -> tuple[int, int]:
    reset_tasks = 0
    reset_jobs = 0
    stale_before = _stale_task_cutoff()
    with SessionLocal() as db:
        running_tasks = list(
            db.scalars(
                select(DevTaskQueueRecord).where(DevTaskQueueRecord.status == "running")
            )
        )
        for task in running_tasks:
            locked_at = _normalize_utc(task.locked_at)
            if locked_at is not None and locked_at > stale_before:
                continue

            task.locked_at = None
            task.locked_by = None
            task.error_message = None

            job = db.get(JobRecord, task.job_id) if task.job_id else None
            if job is not None and job.status in {"succeeded", "failed"}:
                task.status = "succeeded" if job.status == "succeeded" else "failed"
                if job.status == "failed":
                    task.error_message = job.error_message
                reset_tasks += 1
                continue

            task.status = "queued"
            task.available_at = datetime.now(timezone.utc)
            reset_tasks += 1

            if job is not None and job.status == "running":
                job.status = "queued"
                job.started_at = None
                job.finished_at = None
                job.error_code = None
                job.error_message = None
                reset_jobs += 1
                add_job_event(db, job.id, "Recovered stale queue task on startup after worker restart", level="warning")
        db.commit()
    return reset_tasks, reset_jobs


def _reconcile_missing_queue_tasks() -> tuple[int, int]:
    requeued = 0
    failed = 0
    with SessionLocal() as db:
        pending_jobs = list(
            db.scalars(
                select(JobRecord).where(JobRecord.status.in_(("queued", "running")))
            )
        )
        for job in pending_jobs:
            task = db.scalar(
                select(DevTaskQueueRecord).where(DevTaskQueueRecord.job_id == job.id)
            )
            if task is not None:
                if task.status == "failed" and job.status in {"queued", "running"}:
                    job.status = "failed"
                    job.finished_at = datetime.now(timezone.utc)
                    job.error_code = "QUEUE_TASK_FAILED"
                    job.error_message = task.error_message or "Queue task failed before job completion"
                    add_job_event(db, job.id, "Marked failed during startup reconciliation after queue task failure", level="error")
                    if job.operation_type == "ingest":
                        job.version.status = "failed"
                    failed += 1
                continue

            request_payload = dict(job.operation_request.payload_json or {}) if job.operation_request is not None else {}
            if job.operation_type == "ingest":
                source_storage_key = request_payload.get("source_storage_key")
                source_path = request_payload.get("source_path")
                if not source_storage_key and not source_path:
                    job.status = "failed"
                    job.finished_at = datetime.now(timezone.utc)
                    job.error_code = "QUEUE_TASK_MISSING"
                    job.error_message = "Ingest queue task missing and no recoverable source input remained"
                    job.version.status = "failed"
                    add_job_event(db, job.id, "Startup reconciliation could not recover ingest input; job marked failed", level="error")
                    failed += 1
                    continue
                payload = {
                    "model_id": job.version.model_id,
                    "version_id": job.version_id,
                    "job_id": job.id,
                    "source_storage_key": source_storage_key,
                    "source_path": source_path,
                }
                create_database_task(db, "ingest_model", payload, job_id=job.id)
                if job.status == "running":
                    job.status = "queued"
                    job.started_at = None
                add_job_event(db, job.id, "Requeued missing ingest task during startup reconciliation", level="warning")
                requeued += 1
                continue

            if not request_payload:
                job.status = "failed"
                job.finished_at = datetime.now(timezone.utc)
                job.error_code = "QUEUE_TASK_MISSING"
                job.error_message = "Operation queue task missing and request payload could not be reconstructed"
                add_job_event(db, job.id, "Startup reconciliation could not rebuild operation payload; job marked failed", level="error")
                failed += 1
                continue

            payload = {
                "operation_type": job.operation_type,
                "source_version_id": job.version_id,
                "job_id": job.id,
                "payload": request_payload,
            }
            create_database_task(db, "run_operation", payload, job_id=job.id)
            if job.status == "running":
                job.status = "queued"
                job.started_at = None
            add_job_event(db, job.id, "Requeued missing operation task during startup reconciliation", level="warning")
            requeued += 1
        db.commit()
    return requeued, failed


def reconcile_database_queue_state() -> dict[str, int]:
    reset_tasks, reset_jobs = _reset_orphaned_running_tasks()
    requeued, failed = _reconcile_missing_queue_tasks()
    if reset_tasks or reset_jobs or requeued or failed:
        logger.warning(
            "Database queue reconciliation: reset_tasks=%s reset_jobs=%s requeued=%s failed=%s",
            reset_tasks,
            reset_jobs,
            requeued,
            failed,
        )
    return {
        "reset_tasks": reset_tasks,
        "reset_jobs": reset_jobs,
        "requeued": requeued,
        "failed": failed,
    }


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

    heartbeat_stop = threading.Event()
    heartbeat_thread = threading.Thread(
        target=_heartbeat_task_lock,
        args=(task.id, runner, heartbeat_stop),
        name=f"meshinspector-db-queue-heartbeat-{task.id}",
        daemon=True,
    )
    heartbeat_thread.start()
    try:
        if task.task_name == "ingest_model":
            with SessionLocal() as db:
                execute_ingest_task(
                    db,
                    task.payload_json["model_id"],
                    task.payload_json["version_id"],
                    task.payload_json["job_id"],
                    source_storage_key=task.payload_json.get("source_storage_key"),
                    source_path=task.payload_json.get("source_path"),
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
                message = f"Unsupported task: {task.task_name}"
                complete_database_task(db, task_record, "failed", message)
                _fail_linked_job(db, task_record, message)
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
    finally:
        heartbeat_stop.set()
        heartbeat_thread.join(timeout=1)


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
