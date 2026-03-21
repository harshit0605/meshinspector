"""Queue dispatch abstraction for Celery and the database-backed dev queue."""

from __future__ import annotations

from sqlalchemy.orm import Session

from core.config import settings
from storage.repositories import add_job_event, create_database_task
from workers.tasks import ingest_model_task, run_operation_task


def dispatch_ingest_task(db: Session, model_id: str, version_id: str, job_id: str, source_path: str) -> None:
    if settings.queue_uses_database:
        create_database_task(
            db,
            "ingest_model",
            {
                "model_id": model_id,
                "version_id": version_id,
                "job_id": job_id,
                "source_path": source_path,
            },
            job_id=job_id,
        )
        add_job_event(db, job_id, "Queued on database worker", 0)
        return

    ingest_model_task.delay(model_id, version_id, job_id, source_path)


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

    run_operation_task.delay(operation_type, source_version_id, job_id, payload)
