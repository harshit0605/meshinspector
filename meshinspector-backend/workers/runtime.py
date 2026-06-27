"""Shared task runtime used by Celery and the database-backed dev queue."""

from __future__ import annotations

from pathlib import Path

from sqlalchemy.orm import Session

from core.config import settings
from domain.models import JobRecord, ModelRecord, ModelVersionRecord
from domain.schemas import (
    BrushReplayRequest,
    CompareRequest,
    DecimateRequest,
    HollowRequest,
    InteractiveCommitRequest,
    MakeDeloneRequest,
    MakeManufacturableRequest,
    RepairRequest,
    ResizeRequest,
    ScoopRequest,
    SmoothRequest,
    SubdivideRequest,
    ThickenRequest,
)
from services.ingest import run_ingest_pipeline
from services.operations import (
    run_compare_operation,
    run_decimate_operation,
    run_hollow_operation,
    run_interactive_brush_replay_operation,
    run_interactive_commit_operation,
    run_make_delone_operation,
    run_make_manufacturable_operation,
    run_repair_operation,
    run_resize_operation,
    run_scoop_operation,
    run_smooth_operation,
    run_subdivide_operation,
    run_thicken_operation,
)
from storage.object_store import object_store
from storage.repositories import add_job_event, set_job_status


def _materialize_input(storage_key: str | None, local_path: str | None, target_dir: Path) -> Path:
    if storage_key:
        if object_store.driver == "local":
            return object_store.get_local_path(storage_key)
        return object_store.download_to_path(storage_key, target_dir / Path(storage_key).name)
    if local_path:
        return Path(local_path)
    raise RuntimeError("Task input location not found")


def execute_ingest_task(
    db: Session,
    model_id: str,
    version_id: str,
    job_id: str,
    source_storage_key: str | None = None,
    source_path: str | None = None,
) -> None:
    model = db.get(ModelRecord, model_id)
    version = db.get(ModelVersionRecord, version_id)
    job = db.get(JobRecord, job_id)
    if not all([model, version, job]):
        raise RuntimeError("Ingest task context not found")
    try:
        materialized_source = _materialize_input(
            source_storage_key,
            source_path,
            settings.TEMP_DIR / "ingest_inputs" / job_id,
        )
        run_ingest_pipeline(db, model, version, job, materialized_source)
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
    if operation_type == "make_manufacturable" and job.operation_request is not None:
        payload = dict(job.operation_request.payload_json or payload)

    workdir = settings.TEMP_DIR / job_id
    workdir.mkdir(parents=True, exist_ok=True)
    try:
        result: dict | None = None
        if operation_type == "repair":
            version = run_repair_operation(db, source_version, job, workdir, RepairRequest.model_validate(payload))
            result = {"version_id": version.id}
        elif operation_type == "resize":
            version = run_resize_operation(db, source_version, job, workdir, ResizeRequest.model_validate(payload))
            result = {"version_id": version.id}
        elif operation_type == "hollow":
            version = run_hollow_operation(db, source_version, job, workdir, HollowRequest.model_validate(payload))
            result = {"version_id": version.id}
        elif operation_type == "thicken":
            version = run_thicken_operation(db, source_version, job, workdir, ThickenRequest.model_validate(payload))
            result = {"version_id": version.id}
        elif operation_type == "scoop":
            version = run_scoop_operation(db, source_version, job, workdir, ScoopRequest.model_validate(payload))
            result = {"version_id": version.id}
        elif operation_type == "smooth":
            version = run_smooth_operation(db, source_version, job, workdir, SmoothRequest.model_validate(payload))
            result = {"version_id": version.id}
        elif operation_type == "decimate":
            version = run_decimate_operation(
                db,
                source_version,
                job,
                workdir,
                DecimateRequest.model_validate(payload),
            )
            result = {"version_id": version.id}
        elif operation_type == "subdivide":
            version = run_subdivide_operation(
                db,
                source_version,
                job,
                workdir,
                SubdivideRequest.model_validate(payload),
            )
            result = {"version_id": version.id}
        elif operation_type == "make_delone":
            version = run_make_delone_operation(
                db,
                source_version,
                job,
                workdir,
                MakeDeloneRequest.model_validate(payload),
            )
            result = {"version_id": version.id}
        elif operation_type == "interactive_brush_replay":
            version = run_interactive_brush_replay_operation(
                db,
                source_version,
                job,
                workdir,
                BrushReplayRequest.model_validate(payload),
            )
            result = {"version_id": version.id}
        elif operation_type == "compare":
            request = CompareRequest.model_validate(payload)
            other_version = db.get(ModelVersionRecord, request.other_version_id)
            if other_version is None:
                raise RuntimeError("Comparison target not found")
            compare_result = run_compare_operation(db, source_version, other_version, job, workdir)
            result = compare_result.model_dump(mode="json")
        elif operation_type == "make_manufacturable":
            version = run_make_manufacturable_operation(
                db,
                source_version,
                job,
                workdir,
                MakeManufacturableRequest.model_validate(payload),
            )
            result = {"version_id": version.id}
        elif operation_type == "interactive_commit":
            upload_path = payload.get("upload_path")
            upload_storage_key = payload.get("upload_storage_key")
            if not upload_path and not upload_storage_key:
                raise RuntimeError("interactive_commit requires upload_storage_key or upload_path")
            materialized_upload = _materialize_input(
                upload_storage_key,
                upload_path,
                workdir / "interactive_input",
            )
            version = run_interactive_commit_operation(
                db,
                source_version,
                job,
                workdir,
                InteractiveCommitRequest.model_validate(payload),
                materialized_upload,
            )
            result = {"version_id": version.id}
        else:
            raise RuntimeError(f"Unsupported operation type: {operation_type}")

        if result is not None:
            db.refresh(job)
            if job.operation_request is not None:
                next_payload = dict(job.operation_request.payload_json or {})
                next_payload["_result"] = result
                job.operation_request.payload_json = next_payload
                db.commit()
        return result
    except Exception as exc:
        db.rollback()
        set_job_status(db, job, "failed", error_code="OPERATION_FAILED", error_message=str(exc))
        add_job_event(db, job.id, f"{operation_type} failed: {exc}", level="error")
        db.commit()
        raise
