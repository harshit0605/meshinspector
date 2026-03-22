"""Shared task runtime used by Celery and the database-backed dev queue."""

from __future__ import annotations

from pathlib import Path

from sqlalchemy.orm import Session

from domain.models import JobRecord, ModelRecord, ModelVersionRecord
from domain.schemas import (
    CompareRequest,
    HollowRequest,
    MakeManufacturableRequest,
    ResizeRequest,
    ScoopRequest,
    SmoothRequest,
    ThickenRequest,
)
from services.ingest import run_ingest_pipeline
from services.operations import (
    run_compare_operation,
    run_hollow_operation,
    run_make_manufacturable_operation,
    run_repair_operation,
    run_resize_operation,
    run_scoop_operation,
    run_smooth_operation,
    run_thicken_operation,
)
from storage.repositories import add_job_event, set_job_status


def execute_ingest_task(db: Session, model_id: str, version_id: str, job_id: str, source_path: str) -> None:
    model = db.get(ModelRecord, model_id)
    version = db.get(ModelVersionRecord, version_id)
    job = db.get(JobRecord, job_id)
    if not all([model, version, job]):
        raise RuntimeError("Ingest task context not found")
    try:
        run_ingest_pipeline(db, model, version, job, Path(source_path))
    except Exception as exc:
        db.rollback()
        set_job_status(db, job, "failed", error_code="INGEST_FAILED", error_message=str(exc))
        add_job_event(db, job.id, f"Ingest failed: {exc}", level="error")
        version.status = "failed"
        db.commit()
        raise


def execute_operation_task(db: Session, operation_type: str, source_version_id: str, job_id: str, payload: dict) -> dict | None:
    source_version = db.get(ModelVersionRecord, source_version_id)
    job = db.get(JobRecord, job_id)
    if not all([source_version, job]):
        raise RuntimeError("Operation task context not found")

    workdir = Path("temp") / job_id
    workdir.mkdir(parents=True, exist_ok=True)
    try:
        if operation_type == "repair":
            version = run_repair_operation(db, source_version, job, workdir)
            return {"version_id": version.id}
        if operation_type == "resize":
            version = run_resize_operation(db, source_version, job, workdir, ResizeRequest.model_validate(payload))
            return {"version_id": version.id}
        if operation_type == "hollow":
            version = run_hollow_operation(db, source_version, job, workdir, HollowRequest.model_validate(payload))
            return {"version_id": version.id}
        if operation_type == "thicken":
            version = run_thicken_operation(db, source_version, job, workdir, ThickenRequest.model_validate(payload))
            return {"version_id": version.id}
        if operation_type == "scoop":
            version = run_scoop_operation(db, source_version, job, workdir, ScoopRequest.model_validate(payload))
            return {"version_id": version.id}
        if operation_type == "smooth":
            version = run_smooth_operation(db, source_version, job, workdir, SmoothRequest.model_validate(payload))
            return {"version_id": version.id}
        if operation_type == "compare":
            request = CompareRequest.model_validate(payload)
            other_version = db.get(ModelVersionRecord, request.other_version_id)
            if other_version is None:
                raise RuntimeError("Comparison target not found")
            result = run_compare_operation(db, source_version, other_version, job, workdir)
            return result.model_dump(mode="json")
        if operation_type == "make_manufacturable":
            version = run_make_manufacturable_operation(
                db,
                source_version,
                job,
                workdir,
                MakeManufacturableRequest.model_validate(payload),
            )
            return {"version_id": version.id}

        raise RuntimeError(f"Unsupported operation type: {operation_type}")
    except Exception as exc:
        db.rollback()
        set_job_status(db, job, "failed", error_code="OPERATION_FAILED", error_message=str(exc))
        add_job_event(db, job.id, f"{operation_type} failed: {exc}", level="error")
        db.commit()
        raise
