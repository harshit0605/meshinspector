"""Compatibility mesh analysis endpoint."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session

from core.db import get_db
from domain.schemas import MaterialType as NewMaterialType
from models.schemas import AnalysisResult, MaterialType
from storage.repositories import get_latest_version, get_snapshot

router = APIRouter()


@router.get("/analyze/{model_id}", response_model=AnalysisResult)
async def analyze_model(
    model_id: str,
    material: MaterialType = Query(default=MaterialType.GOLD_18K),
    db: Session = Depends(get_db),
):
    latest = get_latest_version(db, model_id)
    if latest is None:
        raise HTTPException(status_code=404, detail="Model not found")

    snapshot_record = get_snapshot(db, latest.id)
    if snapshot_record is None:
        raise HTTPException(status_code=404, detail="Manufacturability snapshot not ready")

    snapshot = snapshot_record.payload_json
    weight_entry = snapshot["material_weight"][material.value]
    dimensions = snapshot["dimensions"]
    mesh_health = snapshot["mesh_health"]
    return AnalysisResult(
        volume_mm3=weight_entry["volume_mm3"],
        weight_g=weight_entry["weight_g"],
        bbox_mm=tuple(dimensions["bbox_mm"]),
        is_watertight=mesh_health["is_closed"],
        vertex_count=0,
        face_count=0,
    )
