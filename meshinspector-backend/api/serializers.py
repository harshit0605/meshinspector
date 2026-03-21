"""Serializer helpers for API responses."""

from __future__ import annotations

from domain.models import (
    AnalysisSnapshotRecord,
    JobEventRecord,
    JobRecord,
    ModelArtifactRecord,
    ModelRecord,
    ModelVersionRecord,
)
from domain.schemas import (
    ArtifactSummary,
    InspectionSnapshotResponse,
    InspectionSnapshotState,
    JobEventResponse,
    JobResponse,
    ManufacturabilitySnapshot,
    ModelSummary,
    ModelVersionSummary,
)
from storage.repositories import get_snapshot


def serialize_model(model: ModelRecord, latest_version_id: str | None = None) -> ModelSummary:
    return ModelSummary(
        id=model.id,
        source_filename=model.source_filename,
        source_type=model.source_type,
        created_at=model.created_at,
        latest_version_id=latest_version_id,
    )


def serialize_version(version: ModelVersionRecord) -> ModelVersionSummary:
    return ModelVersionSummary(
        id=version.id,
        model_id=version.model_id,
        parent_version_id=version.parent_version_id,
        operation_type=version.operation_type,
        operation_label=version.operation_label,
        status=version.status,
        created_at=version.created_at,
    )


def serialize_artifact(artifact: ModelArtifactRecord) -> ArtifactSummary:
    return ArtifactSummary(
        id=artifact.id,
        artifact_type=artifact.artifact_type,
        mime_type=artifact.mime_type,
        storage_key=artifact.storage_key,
        size_bytes=artifact.size_bytes,
        metadata_json=artifact.metadata_json,
    )


def serialize_job(job: JobRecord) -> JobResponse:
    return JobResponse(
        id=job.id,
        version_id=job.version_id,
        operation_type=job.operation_type,
        status=job.status,
        progress_pct=job.progress_pct,
        error_code=job.error_code,
        error_message=job.error_message,
        started_at=job.started_at,
        finished_at=job.finished_at,
        created_at=job.created_at,
    )


def serialize_job_event(event: JobEventRecord) -> JobEventResponse:
    return JobEventResponse(
        id=event.id,
        level=event.level,
        message=event.message,
        progress_pct=event.progress_pct,
        created_at=event.created_at,
    )


def serialize_snapshot(snapshot_record: AnalysisSnapshotRecord | None) -> ManufacturabilitySnapshot | None:
    if snapshot_record is None:
        return None
    return ManufacturabilitySnapshot.model_validate(snapshot_record.payload_json)


def serialize_inspection_snapshot(snapshot_record: AnalysisSnapshotRecord) -> InspectionSnapshotResponse:
    payload = InspectionSnapshotState.model_validate(snapshot_record.payload_json)
    return InspectionSnapshotResponse(
        id=snapshot_record.id,
        version_id=snapshot_record.version_id,
        created_at=snapshot_record.created_at,
        **payload.model_dump(),
    )
