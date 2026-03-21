"""Compatibility processing and file delivery endpoints."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import FileResponse
from sqlalchemy.orm import Session

from core.db import get_db
from domain.schemas import HollowRequest, MaterialType as NewMaterialType, ResizeRequest
from models.schemas import ProcessRequest, ProcessResponse
from storage.object_store import object_store
from storage.repositories import create_job, get_artifact_by_type, get_latest_version, get_snapshot
from workers.dispatch import dispatch_operation_task

router = APIRouter()


def _download_artifact_file(artifact_storage_key: str) -> Path:
    if object_store.driver == "local":
        return object_store.get_local_path(artifact_storage_key)
    temp_path = Path("temp") / "compat" / Path(artifact_storage_key).name
    return object_store.download_to_path(artifact_storage_key, temp_path)


@router.post("/process", response_model=ProcessResponse)
async def process_model(request: ProcessRequest, db: Session = Depends(get_db)):
    source_version = get_latest_version(db, request.model_id)
    if source_version is None:
        raise HTTPException(status_code=404, detail="Model not found")

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
        latest = get_latest_version(db, request.model_id)
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
        latest = get_latest_version(db, request.model_id)
        current_version_id = latest.id if latest else current_version_id

    final_version = get_latest_version(db, request.model_id)
    if final_version is None:
        raise HTTPException(status_code=500, detail="Processed version not found")
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
async def download_model(model_id: str, format: str, db: Session = Depends(get_db)):
    latest = get_latest_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Model not found")

    artifact_type = "preview_glb_high" if format.lower() == "glb" else "manufacturing_stl"
    media_type = "model/gltf-binary" if format.lower() == "glb" else "application/sla"
    artifact = get_artifact_by_type(db, latest.id, artifact_type)
    if artifact is None:
        raise HTTPException(status_code=404, detail="Artifact not found")
    file_path = _download_artifact_file(artifact.storage_key)
    return FileResponse(path=file_path, media_type=media_type, filename=file_path.name)


@router.get("/preview/{model_id}")
async def preview_model(model_id: str, db: Session = Depends(get_db)):
    latest = get_latest_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Model not found")
    artifact = get_artifact_by_type(db, latest.id, "preview_glb_high") or get_artifact_by_type(db, latest.id, "preview_glb_low")
    if artifact is None:
        raise HTTPException(status_code=404, detail="Preview artifact not found")
    file_path = _download_artifact_file(artifact.storage_key)
    return FileResponse(file_path, media_type="model/gltf-binary", headers={"X-Source": "version-preview"})
