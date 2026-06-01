"""Queue dispatch abstraction for Celery and the database-backed dev queue."""

from __future__ import annotations

from collections.abc import Callable

from sqlalchemy import event
from sqlalchemy.orm import Session

from core.config import settings
from storage.repositories import add_job_event, create_database_task
from workers.tasks import ingest_model_task, run_operation_task

_AFTER_COMMIT_CALLBACKS = "meshinspector_after_commit_callbacks"


def _enqueue_after_commit(db: Session, callback: Callable[[], None]) -> None:
    callbacks = db.info.setdefault(_AFTER_COMMIT_CALLBACKS, [])
    callbacks.append(callback)


@event.listens_for(Session, "after_commit")
def _run_after_commit_callbacks(session: Session) -> None:
    callbacks = session.info.pop(_AFTER_COMMIT_CALLBACKS, [])
    for callback in callbacks:
        callback()


@event.listens_for(Session, "after_rollback")
def _clear_after_commit_callbacks(session: Session) -> None:
    session.info.pop(_AFTER_COMMIT_CALLBACKS, None)


@event.listens_for(Session, "after_soft_rollback")
def _clear_after_soft_rollback_callbacks(session: Session, _previous_transaction) -> None:
    session.info.pop(_AFTER_COMMIT_CALLBACKS, None)


def dispatch_ingest_task(
    db: Session,
    model_id: str,
    version_id: str,
    job_id: str,
    source_storage_key: str | None = None,
    source_path: str | None = None,
) -> None:
    if not source_storage_key and not source_path:
        raise RuntimeError("dispatch_ingest_task requires source_storage_key or source_path")
    if settings.queue_uses_database:
        create_database_task(
            db,
            "ingest_model",
            {
                "model_id": model_id,
                "version_id": version_id,
                "job_id": job_id,
                "source_storage_key": source_storage_key,
                "source_path": source_path,
            },
            job_id=job_id,
        )
        add_job_event(db, job_id, "Queued on database worker", 0)
        return

    _enqueue_after_commit(
        db,
        lambda: ingest_model_task.delay(model_id, version_id, job_id, source_storage_key, source_path),
    )


def dispatch_operation_task(
    db: Session,
    operation_type: str,
    source_version_id: str,
    job_id: str,
    payload: dict,
) -> None:
    if settings.queue_uses_database:
        create_database_task(
            db,
            "run_operation",
            {
                "operation_type": operation_type,
                "source_version_id": source_version_id,
                "job_id": job_id,
                "payload": payload,
            },
            job_id=job_id,
        )
        add_job_event(db, job_id, "Queued on database worker", 0)
        return

    _enqueue_after_commit(
        db,
        lambda: run_operation_task.delay(operation_type, source_version_id, job_id, payload),
    )
