"""Database repositories."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import Any

from sqlalchemy import Select, func, select
from sqlalchemy.orm import Session

from domain.models import (
    AnalysisSnapshotRecord,
    DevTaskQueueRecord,
    JobEventRecord,
    JobRecord,
    ModelArtifactRecord,
    ModelRecord,
    ModelVersionRecord,
    OperationRequestRecord,
)


def generate_id(prefix: str) -> str:
    from uuid import uuid4

    return f"{prefix}_{uuid4().hex[:12]}"


def create_model(db: Session, source_filename: str, source_type: str) -> ModelRecord:
    model = ModelRecord(
        id=generate_id("mdl"),
        source_filename=source_filename,
        source_type=source_type,
    )
    db.add(model)
    db.flush()
    return model


def create_version(
    db: Session,
    model_id: str,
    operation_type: str,
    operation_label: str,
    parent_version_id: str | None = None,
    status: str = "ready",
) -> ModelVersionRecord:
    version = ModelVersionRecord(
        id=generate_id("ver"),
        model_id=model_id,
        parent_version_id=parent_version_id,
        operation_type=operation_type,
        operation_label=operation_label,
        status=status,
    )
    db.add(version)
    db.flush()
    return version


def create_artifact(
    db: Session,
    version_id: str,
    artifact_type: str,
    mime_type: str,
    storage_key: str,
    size_bytes: int,
    metadata_json: dict[str, Any] | None = None,
) -> ModelArtifactRecord:
    artifact = ModelArtifactRecord(
        id=generate_id("art"),
        version_id=version_id,
        artifact_type=artifact_type,
        mime_type=mime_type,
        storage_key=storage_key,
        size_bytes=size_bytes,
        metadata_json=metadata_json or {},
    )
    db.add(artifact)
    db.flush()
    return artifact


def upsert_snapshot(
    db: Session,
    version_id: str,
    snapshot_type: str,
    payload_json: dict[str, Any],
) -> AnalysisSnapshotRecord:
    existing = db.scalar(
        select(AnalysisSnapshotRecord).where(
            AnalysisSnapshotRecord.version_id == version_id,
            AnalysisSnapshotRecord.snapshot_type == snapshot_type,
        )
    )
    if existing:
        existing.payload_json = payload_json
        db.flush()
        return existing

    snapshot = AnalysisSnapshotRecord(
        id=generate_id("snp"),
        version_id=version_id,
        snapshot_type=snapshot_type,
        payload_json=payload_json,
    )
    db.add(snapshot)
    db.flush()
    return snapshot


def create_snapshot_record(
    db: Session,
    version_id: str,
    snapshot_type: str,
    payload_json: dict[str, Any],
) -> AnalysisSnapshotRecord:
    snapshot = AnalysisSnapshotRecord(
        id=generate_id("snp"),
        version_id=version_id,
        snapshot_type=snapshot_type,
        payload_json=payload_json,
    )
    db.add(snapshot)
    db.flush()
    return snapshot


def create_job(
    db: Session,
    version_id: str,
    operation_type: str,
    payload_json: dict[str, Any],
) -> JobRecord:
    job = JobRecord(
        id=generate_id("job"),
        version_id=version_id,
        operation_type=operation_type,
        status="queued",
        progress_pct=0,
    )
    db.add(job)
    db.flush()

    request = OperationRequestRecord(id=generate_id("req"), job_id=job.id, payload_json=payload_json)
    db.add(request)
    db.flush()
    return job


def create_database_task(
    db: Session,
    task_name: str,
    payload_json: dict[str, Any],
    job_id: str | None = None,
) -> DevTaskQueueRecord:
    task = DevTaskQueueRecord(
        id=generate_id("tsk"),
        task_name=task_name,
        job_id=job_id,
        payload_json=payload_json,
        status="queued",
        attempts=0,
    )
    db.add(task)
    db.flush()
    return task


def add_job_event(db: Session, job_id: str, message: str, progress_pct: int | None = None, level: str = "info") -> JobEventRecord:
    event = JobEventRecord(
        id=generate_id("evt"),
        job_id=job_id,
        message=message,
        progress_pct=progress_pct,
        level=level,
    )
    db.add(event)
    db.flush()
    return event


def set_job_status(
    db: Session,
    job: JobRecord,
    status: str,
    progress_pct: int | None = None,
    error_code: str | None = None,
    error_message: str | None = None,
) -> JobRecord:
    if status == "running" and job.started_at is None:
        job.started_at = datetime.now(timezone.utc)
    if status in {"succeeded", "failed"}:
        job.finished_at = datetime.now(timezone.utc)
    job.status = status
    if progress_pct is not None:
        job.progress_pct = progress_pct
    job.error_code = error_code
    job.error_message = error_message
    db.flush()
    return job


def get_latest_version(db: Session, model_id: str) -> ModelVersionRecord | None:
    return db.scalar(
        select(ModelVersionRecord)
        .where(ModelVersionRecord.model_id == model_id)
        .order_by(ModelVersionRecord.created_at.desc())
    )


def list_model_versions(db: Session, model_id: str) -> list[ModelVersionRecord]:
    return list(
        db.scalars(
            select(ModelVersionRecord)
            .where(ModelVersionRecord.model_id == model_id)
            .order_by(ModelVersionRecord.created_at.desc())
        )
    )


def get_version_artifacts(db: Session, version_id: str) -> list[ModelArtifactRecord]:
    return list(
        db.scalars(
            select(ModelArtifactRecord)
            .where(ModelArtifactRecord.version_id == version_id)
            .order_by(ModelArtifactRecord.created_at.asc())
        )
    )


def get_artifact_by_type(db: Session, version_id: str, artifact_type: str) -> ModelArtifactRecord | None:
    return db.scalar(
        select(ModelArtifactRecord).where(
            ModelArtifactRecord.version_id == version_id,
            ModelArtifactRecord.artifact_type == artifact_type,
        )
    )


def get_snapshot(db: Session, version_id: str, snapshot_type: str = "manufacturability") -> AnalysisSnapshotRecord | None:
    return db.scalar(
        select(AnalysisSnapshotRecord).where(
            AnalysisSnapshotRecord.version_id == version_id,
            AnalysisSnapshotRecord.snapshot_type == snapshot_type,
        )
    )


def list_snapshots_by_prefix(
    db: Session,
    version_id: str,
    snapshot_type_prefix: str,
) -> list[AnalysisSnapshotRecord]:
    return list(
        db.scalars(
            select(AnalysisSnapshotRecord)
            .where(
                AnalysisSnapshotRecord.version_id == version_id,
                AnalysisSnapshotRecord.snapshot_type.like(f"{snapshot_type_prefix}%"),
            )
            .order_by(AnalysisSnapshotRecord.created_at.desc())
        )
    )


def get_job_events(
    db: Session,
    job_id: str,
    after_created_at: datetime | None = None,
    limit: int = 100,
) -> list[JobEventRecord]:
    query: Select[tuple[JobEventRecord]] = (
        select(JobEventRecord)
        .where(JobEventRecord.job_id == job_id)
        .order_by(JobEventRecord.created_at.asc())
        .limit(limit)
    )
    if after_created_at is not None:
        query = query.where(JobEventRecord.created_at > after_created_at)
    return list(db.scalars(query))


def claim_next_database_task(db: Session, runner_id: str) -> DevTaskQueueRecord | None:
    task = db.scalar(
        select(DevTaskQueueRecord)
        .where(
            DevTaskQueueRecord.status == "queued",
            DevTaskQueueRecord.available_at <= func.now(),
        )
        .order_by(DevTaskQueueRecord.created_at.asc())
        .with_for_update(skip_locked=True)
    )
    if task is None:
        return None

    task.status = "running"
    task.locked_at = datetime.now(timezone.utc)
    task.locked_by = runner_id
    task.attempts += 1
    db.flush()
    return task


def complete_database_task(
    db: Session,
    task: DevTaskQueueRecord,
    status: str,
    error_message: str | None = None,
) -> DevTaskQueueRecord:
    task.status = status
    task.error_message = error_message
    if status != "running":
        task.locked_at = None
        task.locked_by = None
    db.flush()
    return task
