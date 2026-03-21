"""Celery task wrappers for mesh processing."""

from __future__ import annotations

from core.db import SessionLocal
from workers.celery_app import celery_app
from workers.runtime import execute_ingest_task, execute_operation_task


@celery_app.task(name="meshinspector.ingest_model")
def ingest_model_task(model_id: str, version_id: str, job_id: str, source_path: str) -> None:
    with SessionLocal() as db:
        execute_ingest_task(db, model_id, version_id, job_id, source_path)


@celery_app.task(name="meshinspector.run_operation")
def run_operation_task(operation_type: str, source_version_id: str, job_id: str, payload: dict) -> dict | None:
    with SessionLocal() as db:
        return execute_operation_task(db, operation_type, source_version_id, job_id, payload)
