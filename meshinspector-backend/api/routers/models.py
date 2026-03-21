"""Model creation and lookup routes."""

from __future__ import annotations

import shutil
from pathlib import Path

from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from sqlalchemy.orm import Session

from api.serializers import serialize_job, serialize_model, serialize_version
from core.config import settings
from core.db import get_db
from domain.models import ModelRecord
from domain.schemas import CreateModelResponse, ModelSummary, ModelVersionSummary
from storage.repositories import create_job, create_model, create_version, get_latest_version, list_model_versions
from utils.file_io import validate_file_extension
from workers.dispatch import dispatch_ingest_task

router = APIRouter()


@router.post("/models", response_model=CreateModelResponse)
async def create_model_from_upload(
    file: UploadFile = File(...),
    db: Session = Depends(get_db),
) -> CreateModelResponse:
    ext = validate_file_extension(file.filename or "")
    if not ext:
        raise HTTPException(status_code=400, detail=f"Invalid file type. Allowed: {', '.join(sorted(settings.ALLOWED_EXTENSIONS))}")

    file.file.seek(0, 2)
    size_mb = file.file.tell() / (1024 * 1024)
    file.file.seek(0)
    if size_mb > settings.MAX_FILE_SIZE_MB:
        raise HTTPException(status_code=400, detail=f"File too large. Maximum: {settings.MAX_FILE_SIZE_MB}MB")

    model = create_model(db, file.filename or "unknown", ext.lstrip("."))
    version = create_version(
        db,
        model_id=model.id,
        operation_type="ingest",
        operation_label="Initial Upload",
        status="processing",
    )
    job = create_job(db, version.id, "ingest", {"source_filename": file.filename or "unknown"})

    source_path = settings.TEMP_DIR / f"{version.id}{ext}"
    with source_path.open("wb") as buffer:
        shutil.copyfileobj(file.file, buffer)

    dispatch_ingest_task(db, model.id, version.id, job.id, str(source_path))
    db.commit()
    db.refresh(version)
    db.refresh(job)
    return CreateModelResponse(
        model=serialize_model(model, latest_version_id=version.id),
        version=serialize_version(version),
        job=serialize_job(job),
    )


@router.get("/models/{model_id}", response_model=ModelSummary)
async def get_model(model_id: str, db: Session = Depends(get_db)) -> ModelSummary:
    model = db.get(ModelRecord, model_id)
    if model is None:
        raise HTTPException(status_code=404, detail="Model not found")
    latest_version = get_latest_version(db, model_id)
    return serialize_model(model, latest_version_id=latest_version.id if latest_version else None)


@router.get("/models/{model_id}/versions", response_model=list[ModelVersionSummary])
async def get_model_versions(model_id: str, db: Session = Depends(get_db)) -> list[ModelVersionSummary]:
    model = db.get(ModelRecord, model_id)
    if model is None:
        raise HTTPException(status_code=404, detail="Model not found")
    return [serialize_version(version) for version in list_model_versions(db, model_id)]
