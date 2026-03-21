"""Versioned ingest pipeline."""

from __future__ import annotations

from pathlib import Path

from sqlalchemy.orm import Session

from core.logging import get_logger
from domain.models import JobRecord, ModelRecord, ModelVersionRecord
from services.convert import to_glb, to_ply, to_stl
from services.manufacturability import compute_manufacturability_snapshot
from services.versioning import register_file_artifact
from storage.repositories import add_job_event, set_job_status, upsert_snapshot

logger = get_logger(__name__)


def run_ingest_pipeline(
    db: Session,
    model: ModelRecord,
    version: ModelVersionRecord,
    job: JobRecord,
    source_path: Path,
) -> None:
    """Materialize upload artifacts and compute the baseline manufacturability snapshot."""
    add_job_event(db, job.id, "Ingest started", 5)
    set_job_status(db, job, "running", progress_pct=5)

    register_file_artifact(
        db,
        version.id,
        source_path,
        artifact_type="original_upload",
        metadata_json={"source_filename": model.source_filename},
    )
    add_job_event(db, job.id, "Stored original upload", 15)

    workdir = source_path.parent
    normalized_ply = workdir / f"{version.id}.ply"
    preview_glb_high = workdir / f"{version.id}_high.glb"
    preview_glb_low = workdir / f"{version.id}_low.glb"
    manufacturing_stl = workdir / f"{version.id}.stl"

    to_ply(source_path, normalized_ply)
    to_glb(normalized_ply, preview_glb_high)
    to_glb(normalized_ply, preview_glb_low)
    to_stl(normalized_ply, manufacturing_stl)
    add_job_event(db, job.id, "Generated normalized and preview artifacts", 45)

    register_file_artifact(db, version.id, normalized_ply, "normalized_mesh_ply", "model/ply")
    register_file_artifact(db, version.id, preview_glb_high, "preview_glb_high", "model/gltf-binary")
    register_file_artifact(db, version.id, preview_glb_low, "preview_glb_low", "model/gltf-binary")
    register_file_artifact(db, version.id, manufacturing_stl, "manufacturing_stl", "application/sla")
    add_job_event(db, job.id, "Registered mesh artifacts", 60)

    snapshot, snapshot_artifacts = compute_manufacturability_snapshot(normalized_ply, workdir)
    thickness_artifact = register_file_artifact(
        db,
        version.id,
        snapshot_artifacts.thickness_scalar_path,
        "analysis_thickness_npz",
        "application/octet-stream",
    )
    register_file_artifact(
        db,
        version.id,
        snapshot_artifacts.region_json_path,
        "analysis_regions_json",
        "application/json",
    )
    snapshot.version_id = version.id
    snapshot.thickness.scalar_field_artifact_id = thickness_artifact.id
    upsert_snapshot(db, version.id, "manufacturability", snapshot.model_dump(mode="json"))
    add_job_event(db, job.id, "Manufacturability snapshot computed", 90)

    version.status = "ready"
    set_job_status(db, job, "succeeded", progress_pct=100)
    add_job_event(db, job.id, "Ingest completed", 100)
    db.commit()
    logger.info(f"Ingest pipeline completed for version {version.id}")
