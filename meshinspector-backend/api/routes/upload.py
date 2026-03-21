"""Compatibility upload endpoint."""

from __future__ import annotations

import shutil

from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from sqlalchemy.orm import Session

from api.serializers import serialize_job
from core.config import settings
from core.db import get_db
from models.schemas import UploadResponse
from storage.repositories import create_job, create_model, create_version
from utils.file_io import validate_file_extension
from workers.dispatch import dispatch_ingest_task

router = APIRouter()


@router.post("/upload", response_model=UploadResponse)
async def upload_model(file: UploadFile = File(...), db: Session = Depends(get_db)):
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

    return UploadResponse(
        model_id=model.id,
        filename=file.filename or "unknown",
        file_format=ext.lstrip(".").upper(),
        preview_url=f"/api/preview/{model.id}",
    )
