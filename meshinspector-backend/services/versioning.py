"""Helpers for working with versions and artifacts."""

from __future__ import annotations

import tempfile
from pathlib import Path

from sqlalchemy.orm import Session

from domain.models import AnalysisSnapshotRecord, ModelArtifactRecord, ModelVersionRecord
from storage.object_store import object_store
from storage.repositories import create_artifact, create_version, generate_id


def artifact_storage_key(version_id: str, artifact_type: str, suffix: str) -> str:
    suffix = suffix.lstrip(".")
    return f"{version_id}/{artifact_type}.{suffix}"


def register_file_artifact(
    db: Session,
    version_id: str,
    file_path: Path,
    artifact_type: str,
    mime_type: str | None = None,
    metadata_json: dict | None = None,
) -> ModelArtifactRecord:
    key = artifact_storage_key(version_id, artifact_type, file_path.suffix)
    size = object_store.put_file(file_path, key, content_type=mime_type)
    return create_artifact(
        db=db,
        version_id=version_id,
        artifact_type=artifact_type,
        mime_type=mime_type or object_store.guess_content_type(file_path),
        storage_key=key,
        size_bytes=size,
        metadata_json=metadata_json or {},
    )


def materialize_artifact(artifact: ModelArtifactRecord, workdir: Path) -> Path:
    workdir.mkdir(parents=True, exist_ok=True)
    target = workdir / Path(artifact.storage_key).name
    return object_store.download_to_path(artifact.storage_key, target)


def duplicate_version(
    db: Session,
    source_version: ModelVersionRecord,
    *,
    operation_type: str = "branch",
    operation_label: str = "Restore Branch",
) -> ModelVersionRecord:
    """Clone a ready version into a new immutable working branch."""
    cloned = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type=operation_type,
        operation_label=operation_label,
        status="ready",
    )

    scratch = Path(tempfile.gettempdir()) / f"meshinspector_clone_{cloned.id}"
    scratch.mkdir(parents=True, exist_ok=True)

    for artifact in source_version.artifacts:
        source_path = materialize_artifact(artifact, scratch / "artifacts")
        register_file_artifact(
            db,
            cloned.id,
            source_path,
            artifact.artifact_type,
            artifact.mime_type,
            metadata_json={**artifact.metadata_json, "cloned_from_artifact_id": artifact.id},
        )

    for snapshot in source_version.snapshots:
        cloned_snapshot = AnalysisSnapshotRecord(
            id=generate_id("snp"),
            version_id=cloned.id,
            snapshot_type=snapshot.snapshot_type,
            payload_json=snapshot.payload_json,
        )
        db.add(cloned_snapshot)

    db.flush()
    return cloned
