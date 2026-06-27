"""Compatibility mesh analysis endpoint."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException, Query, Response
from sqlalchemy.orm import Session

from core.config import settings
from core.db import get_db
from geometry_sdk import default_sdk
from models.schemas import AnalysisResult, MaterialType
from storage.object_store import object_store
from storage.repositories import get_artifact_by_type, get_latest_ready_version, get_snapshot

router = APIRouter()

_DEPRECATION_WARNING = '299 - "Compatibility API is deprecated; migrate to /api/models and /api/versions routes."'


def _set_compat_headers(response: Response) -> None:
    response.headers["Deprecation"] = "true"
    response.headers["Sunset"] = "Wed, 31 Dec 2026 23:59:59 GMT"
    response.headers["Warning"] = _DEPRECATION_WARNING


@router.get("/analyze/{model_id}", response_model=AnalysisResult)
async def analyze_model(
    response: Response,
    model_id: str,
    material: MaterialType = Query(default=MaterialType.GOLD_18K),
    db: Session = Depends(get_db),
):
    _set_compat_headers(response)
    latest = get_latest_ready_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Ready version not found")
    response.headers["X-Compat-Version-Id"] = latest.id

    snapshot_record = get_snapshot(db, latest.id)
    if snapshot_record is None:
        raise HTTPException(status_code=404, detail="Manufacturability snapshot not ready")

    snapshot = snapshot_record.payload_json
    weight_entry = snapshot["material_weight"][material.value]
    dimensions = snapshot["dimensions"]
    mesh_health = snapshot["mesh_health"]
    normalized = get_artifact_by_type(db, latest.id, "normalized_mesh_ply")
    vertex_count = 0
    face_count = 0
    if normalized is not None:
        if object_store.driver == "local":
            mesh_path = object_store.get_local_path(normalized.storage_key)
        else:
            temp_path = settings.TEMP_DIR / "compat_artifacts" / latest.id / Path(normalized.storage_key).name
            mesh_path = object_store.download_to_path(normalized.storage_key, temp_path)
        mesh = default_sdk.load_mesh(mesh_path)
        vertex_count = mesh.vertex_count
        face_count = mesh.face_count
    return AnalysisResult(
        volume_mm3=weight_entry["volume_mm3"],
        weight_g=weight_entry["weight_g"],
        bbox_mm=tuple(dimensions["bbox_mm"]),
        is_watertight=mesh_health["is_closed"],
        vertex_count=vertex_count,
        face_count=face_count,
    )
