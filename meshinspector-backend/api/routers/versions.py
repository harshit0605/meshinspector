"""Version and artifact routes."""

from __future__ import annotations

import json
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import FileResponse
import meshlib.mrmeshpy as mm
import numpy as np
from sqlalchemy.orm import Session

from api.serializers import serialize_artifact, serialize_inspection_snapshot, serialize_snapshot, serialize_version
from core.db import get_db
from domain.models import ModelArtifactRecord, ModelVersionRecord
from domain.schemas import BranchVersionRequest, CompareCacheEntry, InspectionSnapshotResponse, InspectionSnapshotState, ModelVersionSummary, VersionDetailResponse, ViewerManifest
from services.versioning import duplicate_version, register_file_artifact
from storage.object_store import object_store
from storage.repositories import create_snapshot_record, get_artifact_by_type, get_snapshot, get_version_artifacts, list_snapshots_by_prefix

router = APIRouter()


def _load_json_artifact(artifact: ModelArtifactRecord | None) -> dict | None:
    if artifact is None:
        return None
    if object_store.driver == "local":
        path = object_store.get_local_path(artifact.storage_key)
    else:
        path = object_store.download_to_path(artifact.storage_key, Path("temp") / "downloads" / Path(artifact.storage_key).name)
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _materialize_artifact_to_path(artifact: ModelArtifactRecord) -> Path:
    if object_store.driver == "local":
        return object_store.get_local_path(artifact.storage_key)
    return object_store.download_to_path(artifact.storage_key, Path("temp") / "downloads" / Path(artifact.storage_key).name)


def _load_npz_artifact(artifact: ModelArtifactRecord | None) -> dict[str, np.ndarray] | None:
    if artifact is None:
        return None
    path = _materialize_artifact_to_path(artifact)
    payload = np.load(path)
    return {key: payload[key] for key in payload.files}


@router.get("/versions/{version_id}", response_model=VersionDetailResponse)
async def get_version(version_id: str, db: Session = Depends(get_db)) -> VersionDetailResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = get_snapshot(db, version_id)
    return VersionDetailResponse(
        version=serialize_version(version),
        artifacts=[serialize_artifact(artifact) for artifact in get_version_artifacts(db, version_id)],
        latest_snapshot=serialize_snapshot(snapshot),
    )


@router.get("/versions/{version_id}/manuf")
async def get_manufacturability_snapshot(version_id: str, db: Session = Depends(get_db)):
    snapshot = get_snapshot(db, version_id)
    if snapshot is None:
        raise HTTPException(status_code=404, detail="Manufacturability snapshot not found")
    return snapshot.payload_json


@router.get("/versions/{version_id}/viewer", response_model=ViewerManifest)
async def get_viewer_manifest(version_id: str, db: Session = Depends(get_db)) -> ViewerManifest:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = serialize_snapshot(get_snapshot(db, version_id))
    high = get_artifact_by_type(db, version_id, "preview_glb_high")
    low = get_artifact_by_type(db, version_id, "preview_glb_low")
    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    thickness = get_artifact_by_type(db, version_id, "analysis_thickness_npz")
    regions = get_artifact_by_type(db, version_id, "analysis_regions_json")
    if snapshot is None:
        raise HTTPException(status_code=404, detail="Snapshot not available")
    region_payload = _load_json_artifact(regions) or {}
    return ViewerManifest(
        version_id=version_id,
        preview_low_url=f"/api/artifacts/{low.id}" if low else None,
        preview_high_url=f"/api/artifacts/{high.id}" if high else None,
        normalized_mesh_url=f"/api/artifacts/{normalized.id}" if normalized else None,
        thickness_artifact_url=f"/api/artifacts/{thickness.id}" if thickness else None,
        region_artifact_url=f"/api/artifacts/{regions.id}" if regions else None,
        bounding_box=snapshot.dimensions.bbox_mm,
        available_overlays=[item for item in ["thickness" if thickness else None, "regions" if regions else None] if item],
        region_manifest=snapshot.regions if snapshot.regions else region_payload.get("regions", []),
        measurements_summary=snapshot.dimensions.model_dump(mode="json"),
        needs_axis_confirmation=snapshot.dimensions.needs_axis_confirmation,
    )


@router.get("/versions/{version_id}/compare-cache", response_model=list[CompareCacheEntry])
async def get_compare_cache(version_id: str, db: Session = Depends(get_db)) -> list[CompareCacheEntry]:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    entries: list[CompareCacheEntry] = []
    for artifact in get_version_artifacts(db, version_id):
        if not artifact.artifact_type.startswith("analysis_compare_npz_"):
            continue
        other_version_id = str(artifact.metadata_json.get("other_version_id") or artifact.artifact_type.removeprefix("analysis_compare_npz_"))
        entries.append(
            CompareCacheEntry(
                other_version_id=other_version_id,
                artifact_id=artifact.id,
                created_at=artifact.created_at,
                generated_by=artifact.metadata_json.get("generated_by"),
            )
        )
    return sorted(entries, key=lambda entry: entry.created_at, reverse=True)


@router.post("/versions/{version_id}/branch", response_model=ModelVersionSummary)
async def branch_version(
    version_id: str,
    request: BranchVersionRequest,
    db: Session = Depends(get_db),
) -> ModelVersionSummary:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    cloned = duplicate_version(
        db,
        version,
        operation_type="branch",
        operation_label=request.operation_label.strip(),
    )
    db.commit()
    db.refresh(cloned)
    return serialize_version(cloned)


@router.get("/versions/{version_id}/inspection-snapshots", response_model=list[InspectionSnapshotResponse])
async def get_inspection_snapshots(version_id: str, db: Session = Depends(get_db)) -> list[InspectionSnapshotResponse]:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    return [serialize_inspection_snapshot(snapshot) for snapshot in list_snapshots_by_prefix(db, version_id, "inspection:")]


@router.post("/versions/{version_id}/inspection-snapshots", response_model=InspectionSnapshotResponse)
async def create_inspection_snapshot(
    version_id: str,
    request: InspectionSnapshotState,
    db: Session = Depends(get_db),
) -> InspectionSnapshotResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = create_snapshot_record(
        db,
        version_id,
        f"inspection:{request.name.strip()}:{version_id}",
        request.model_dump(mode="json"),
    )
    db.commit()
    db.refresh(snapshot)
    return serialize_inspection_snapshot(snapshot)


@router.get("/versions/{version_id}/overlays/thickness")
async def get_thickness_overlay(version_id: str, db: Session = Depends(get_db)):
    artifact = get_artifact_by_type(db, version_id, "analysis_thickness_npz")
    payload = _load_npz_artifact(artifact)
    if payload is None:
        raise HTTPException(status_code=404, detail="Thickness overlay not found")
    values = payload["thickness"].astype(np.float32)
    finite = values[np.isfinite(values)]
    threshold = float(payload.get("threshold_mm", np.array([0.0], dtype=np.float32)).reshape(-1)[0])
    return {
        "overlay_type": "thickness",
        "values": np.nan_to_num(values, nan=0.0).round(5).tolist(),
        "min_value": round(float(np.min(finite)), 5) if finite.size else 0.0,
        "max_value": round(float(np.max(finite)), 5) if finite.size else 0.0,
        "center_value": threshold,
        "threshold_mm": threshold,
    }


@router.get("/versions/{version_id}/overlays/compare/{other_version_id}")
async def get_compare_overlay(version_id: str, other_version_id: str, db: Session = Depends(get_db)):
    version = db.get(ModelVersionRecord, version_id)
    other_version = db.get(ModelVersionRecord, other_version_id)
    if version is None or other_version is None:
        raise HTTPException(status_code=404, detail="Version not found")

    artifact_a = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    artifact_b = get_artifact_by_type(db, other_version_id, "normalized_mesh_ply")
    if artifact_a is None or artifact_b is None:
        raise HTTPException(status_code=404, detail="Comparison mesh artifact not found")

    cached = get_artifact_by_type(db, version_id, f"analysis_compare_npz_{other_version_id}")
    cached_payload = _load_npz_artifact(cached)
    if cached_payload is not None:
        values = cached_payload["values"].astype(np.float32)
        finite = values[np.isfinite(values)]
        abs_max = float(np.max(np.abs(finite))) if finite.size else 0.0
        return {
            "overlay_type": "compare",
            "values": np.nan_to_num(values, nan=0.0).round(5).tolist(),
            "min_value": round(float(np.min(finite)), 5) if finite.size else 0.0,
            "max_value": round(float(np.max(finite)), 5) if finite.size else 0.0,
            "center_value": 0.0,
            "threshold_mm": None,
            "summary": {
                "other_version_id": other_version_id,
                "max_abs_distance_mm": round(abs_max, 5),
                "mean_distance_mm": round(float(np.mean(finite)), 5) if finite.size else 0.0,
                "cached": True,
            },
        }

    path_a = _materialize_artifact_to_path(artifact_a)
    path_b = _materialize_artifact_to_path(artifact_b)
    mesh_a = mm.loadMesh(str(path_a))
    mesh_b = mm.loadMesh(str(path_b))
    scalars = mm.findSignedDistances(mesh_a, mesh_b)
    values = np.array([float(scalars.vec[i]) for i in range(scalars.size())], dtype=np.float32)
    finite = values[np.isfinite(values)]
    abs_max = float(np.max(np.abs(finite))) if finite.size else 0.0
    compare_npz = Path("temp") / "downloads" / f"{version_id}_compare_{other_version_id}.npz"
    compare_npz.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(compare_npz, values=np.nan_to_num(values, nan=0.0), other_version_id=np.array([other_version_id]))
    register_file_artifact(
        db,
        version_id,
        compare_npz,
        f"analysis_compare_npz_{other_version_id}",
        "application/octet-stream",
        metadata_json={"other_version_id": other_version_id, "generated_by": "overlay_endpoint"},
    )
    db.commit()
    return {
        "overlay_type": "compare",
        "values": np.nan_to_num(values, nan=0.0).round(5).tolist(),
        "min_value": round(float(np.min(finite)), 5) if finite.size else 0.0,
        "max_value": round(float(np.max(finite)), 5) if finite.size else 0.0,
        "center_value": 0.0,
        "threshold_mm": None,
        "summary": {
            "other_version_id": other_version_id,
            "max_abs_distance_mm": round(abs_max, 5),
            "mean_distance_mm": round(float(np.mean(finite)), 5) if finite.size else 0.0,
            "cached": False,
        },
    }


@router.get("/artifacts/{artifact_id}")
async def download_artifact(artifact_id: str, db: Session = Depends(get_db)):
    artifact = db.get(ModelArtifactRecord, artifact_id)
    if artifact is None:
        raise HTTPException(status_code=404, detail="Artifact not found")

    if object_store.driver == "local":
        path = object_store.get_local_path(artifact.storage_key)
        return FileResponse(path, media_type=artifact.mime_type, filename=Path(path).name)

    temp_path = Path("temp") / "downloads" / Path(artifact.storage_key).name
    object_store.download_to_path(artifact.storage_key, temp_path)
    return FileResponse(temp_path, media_type=artifact.mime_type, filename=temp_path.name)
