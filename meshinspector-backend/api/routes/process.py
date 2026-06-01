"""Compatibility processing and file delivery endpoints."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException, Response
from fastapi.responses import FileResponse
from sqlalchemy.orm import Session

from core.db import get_db
from core.config import settings
from domain.models import ModelArtifactRecord
from domain.schemas import HollowRequest, MaterialType as NewMaterialType, ResizeRequest
from models.schemas import ProcessRequest, ProcessResponse
from storage.object_store import object_store
from storage.repositories import create_job, get_artifact_by_type, get_latest_ready_version, get_snapshot
from workers.dispatch import dispatch_operation_task

router = APIRouter()

_DEPRECATION_WARNING = '299 - "Compatibility API is deprecated; migrate to /api/models and /api/versions routes."'


def _compat_headers() -> dict[str, str]:
    return {
        "Deprecation": "true",
        "Sunset": "Wed, 31 Dec 2026 23:59:59 GMT",
        "Warning": _DEPRECATION_WARNING,
    }


def _set_compat_headers(response: Response) -> None:
    for key, value in _compat_headers().items():
        response.headers[key] = value


def _download_artifact_file(artifact: ModelArtifactRecord) -> Path:
    if object_store.driver == "local":
        return object_store.get_local_path(artifact.storage_key)
    temp_path = settings.TEMP_DIR / "compat_downloads" / artifact.version_id / artifact.id / Path(artifact.storage_key).name
    return object_store.download_to_path(artifact.storage_key, temp_path)


@router.post("/process", response_model=ProcessResponse)
async def process_model(request: ProcessRequest, response: Response, db: Session = Depends(get_db)):
    _set_compat_headers(response)
    if settings.queue_uses_database or not settings.CELERY_TASK_ALWAYS_EAGER:
        raise HTTPException(
            status_code=409,
            detail="Compatibility process endpoint is not supported in async queue mode; use /api/versions operations instead",
            headers=_compat_headers(),
        )
    source_version = get_latest_ready_version(db, request.model_id)
    if source_version is None:
        raise HTTPException(status_code=404, detail="Ready version not found")
    response.headers["X-Compat-Version-Id"] = source_version.id

    original_snapshot_record = get_snapshot(db, source_version.id)
    if original_snapshot_record is None:
        raise HTTPException(status_code=409, detail="Manufacturability snapshot not ready")
    original_snapshot = original_snapshot_record.payload_json
    original_weight = original_snapshot["material_weight"][request.material.value]["weight_g"]

    current_version_id = source_version.id

    if request.ring_size is not None:
        resize_job = create_job(db, current_version_id, "resize", ResizeRequest(target_ring_size_us=request.ring_size).model_dump(mode="json"))
        dispatch_operation_task(
            db,
            "resize",
            current_version_id,
            resize_job.id,
            ResizeRequest(target_ring_size_us=request.ring_size).model_dump(mode="json"),
        )
        db.commit()
        latest = get_latest_ready_version(db, request.model_id)
        current_version_id = latest.id if latest else current_version_id

    if request.wall_thickness_mm is not None or request.target_weight_g is not None:
        hollow_payload = HollowRequest(
            mode="target_weight" if request.target_weight_g is not None else "fixed_thickness",
            material=NewMaterialType(request.material.value),
            wall_thickness_mm=request.wall_thickness_mm,
            target_weight_g=request.target_weight_g,
        ).model_dump(mode="json")
        hollow_job = create_job(db, current_version_id, "hollow", hollow_payload)
        dispatch_operation_task(db, "hollow", current_version_id, hollow_job.id, hollow_payload)
        db.commit()
        latest = get_latest_ready_version(db, request.model_id)
        current_version_id = latest.id if latest else current_version_id

    final_version = get_latest_ready_version(db, request.model_id)
    if final_version is None:
        raise HTTPException(status_code=500, detail="Processed version not found")
    response.headers["X-Compat-Version-Id"] = final_version.id
    final_snapshot_record = get_snapshot(db, final_version.id)
    if final_snapshot_record is None:
        raise HTTPException(status_code=500, detail="Processed snapshot not available")
    final_snapshot = final_snapshot_record.payload_json
    final_weight = final_snapshot["material_weight"][request.material.value]["weight_g"]

    return ProcessResponse(
        model_id=request.model_id,
        original_weight_g=round(original_weight, 3),
        final_weight_g=round(final_weight, 3),
        wall_thickness_mm=request.wall_thickness_mm,
        ring_size=request.ring_size,
        preview_url=f"/api/preview/{request.model_id}",
        download_url_glb=f"/api/download/{request.model_id}/glb",
        download_url_stl=f"/api/download/{request.model_id}/stl",
        achieved_weight_g=request.target_weight_g,
        iterations=None,
        warning=None,
    )


@router.get("/download/{model_id}/{format}")
async def download_model(model_id: str, format: str, response: Response, db: Session = Depends(get_db)):
    _set_compat_headers(response)
    latest = get_latest_ready_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Ready version not found")
    response.headers["X-Compat-Version-Id"] = latest.id

    normalized_format = format.lower()
    if normalized_format not in {"glb", "stl"}:
        raise HTTPException(status_code=400, detail="Unsupported download format")

    artifact_type = "preview_glb_high" if normalized_format == "glb" else "manufacturing_stl"
    media_type = "model/gltf-binary" if normalized_format == "glb" else "application/sla"
    artifact = get_artifact_by_type(db, latest.id, artifact_type)
    if artifact is None:
        raise HTTPException(status_code=404, detail="Artifact not found")
    file_path = _download_artifact_file(artifact)
    return FileResponse(path=file_path, media_type=media_type, filename=file_path.name, headers=_compat_headers())


@router.get("/preview/{model_id}")
async def preview_model(model_id: str, response: Response, db: Session = Depends(get_db)):
    _set_compat_headers(response)
    latest = get_latest_ready_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Ready version not found")
    response.headers["X-Compat-Version-Id"] = latest.id
    artifact = get_artifact_by_type(db, latest.id, "preview_glb_high") or get_artifact_by_type(db, latest.id, "preview_glb_low")
    if artifact is None:
        raise HTTPException(status_code=404, detail="Preview artifact not found")
    file_path = _download_artifact_file(artifact)
    return FileResponse(
        file_path,
        media_type="model/gltf-binary",
        headers={
            **_compat_headers(),
            "X-Source": "version-preview",
        },
    )
