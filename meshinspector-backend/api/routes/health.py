"""Compatibility health, thickness, and repair endpoints."""

from __future__ import annotations

import numpy as np
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy import select
from sqlalchemy.orm import Session

from core.db import get_db
from domain.models import JobEventRecord, JobRecord, ModelVersionRecord
from storage.object_store import object_store
from storage.repositories import create_job, get_artifact_by_type, get_latest_version, get_snapshot
from workers.dispatch import dispatch_operation_task

router = APIRouter()


class HealthResponse(BaseModel):
    is_closed: bool
    self_intersections: int
    self_intersection_faces: list[int] = []
    holes_count: int
    degenerate_faces: int = 0
    health_score: int


class ThicknessResponse(BaseModel):
    min_thickness: float
    max_thickness: float
    avg_thickness: float
    violation_count: int
    vertex_thickness: list[float]
    threshold_used: float
    total_vertices: int


class RepairResponse(BaseModel):
    success: bool
    repairs_made: list[str]
    issues_fixed: int = 0
    model_id: str | None = None
    output_path: str | None = None
    error: str | None = None


@router.get("/health/{model_id}", response_model=HealthResponse)
async def get_mesh_health(model_id: str, db: Session = Depends(get_db)):
    latest = get_latest_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Model not found")
    snapshot = get_snapshot(db, latest.id)
    if snapshot is None:
        raise HTTPException(status_code=404, detail="Manufacturability snapshot not ready")
    health = snapshot.payload_json["mesh_health"]
    return HealthResponse(**health)


@router.get("/thickness/{model_id}", response_model=ThicknessResponse)
async def get_thickness_analysis(model_id: str, min_thickness: float = 0.6, db: Session = Depends(get_db)):
    latest = get_latest_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Model not found")
    snapshot = get_snapshot(db, latest.id)
    thickness_artifact = get_artifact_by_type(db, latest.id, "analysis_thickness_npz")
    if snapshot is None or thickness_artifact is None:
        raise HTTPException(status_code=404, detail="Thickness analysis not ready")

    if object_store.driver == "local":
        npz_path = object_store.get_local_path(thickness_artifact.storage_key)
    else:
        from pathlib import Path

        npz_path = object_store.download_to_path(thickness_artifact.storage_key, Path("temp") / f"{thickness_artifact.id}.npz")
    data = np.load(npz_path)
    thickness = data["thickness"].astype(np.float32)
    valid = thickness[np.isfinite(thickness)]
    thickness_summary = snapshot.payload_json["thickness"]
    return ThicknessResponse(
        min_thickness=float(thickness_summary["min_mm"] or 0.0),
        max_thickness=float(thickness_summary["max_mm"] or 0.0),
        avg_thickness=float(thickness_summary["avg_mm"] or 0.0),
        violation_count=int(thickness_summary["violation_count"]),
        vertex_thickness=np.nan_to_num(thickness, nan=0.0).tolist(),
        threshold_used=float(thickness_summary["threshold_mm"]),
        total_vertices=int(thickness.shape[0]),
    )


@router.post("/repair/{model_id}", response_model=RepairResponse)
async def repair_mesh(model_id: str, db: Session = Depends(get_db)):
    latest = get_latest_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Model not found")
    job = create_job(db, latest.id, "repair", {})
    dispatch_operation_task(db, "repair", latest.id, job.id, {})
    db.commit()
    new_latest = get_latest_version(db, model_id)
    return RepairResponse(
        success=True,
        repairs_made=["Repair job submitted"],
        issues_fixed=1,
        model_id=model_id,
        output_path=f"/api/preview/{model_id}" if new_latest else None,
    )
