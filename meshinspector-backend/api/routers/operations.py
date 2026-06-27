"""Operation submission routes."""

from __future__ import annotations

import shutil

from fastapi import APIRouter, Depends, File, Form, HTTPException, UploadFile
from sqlalchemy.orm import Session

from api.serializers import serialize_job
from core.config import settings
from core.db import get_db
from domain.models import ModelVersionRecord
from domain.schemas import (
    BrushReplayRequest,
    CompareRequest,
    DecimateRequest,
    HollowRequest,
    InteractiveCommitRequest,
    MakeDeloneRequest,
    JobResponse,
    MakeManufacturableRequest,
    RepairRequest,
    ResizeRequest,
    ScoopRequest,
    SmoothRequest,
    SubdivideRequest,
    ThickenRequest,
)
from storage.object_store import object_store
from storage.repositories import create_job
from utils.file_io import validate_file_extension
from workers.dispatch import dispatch_operation_task

router = APIRouter()


def _submit_operation(
    db: Session,
    version_id: str,
    operation_type: str,
    payload: dict,
) -> JobResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for a new operation")
    job = create_job(db, version_id, operation_type, payload)
    dispatch_operation_task(db, operation_type, version_id, job.id, payload)
    db.commit()
    db.refresh(job)
    return serialize_job(job)


@router.post("/versions/{version_id}/repair", response_model=JobResponse)
async def submit_repair(
    version_id: str,
    request: RepairRequest | None = None,
    db: Session = Depends(get_db),
) -> JobResponse:
    # Body is optional: no body => default gentle repair (back-compat). A body with
    # voxel_remesh=true opts into the aggressive voxel-remesh heal.
    payload = (request or RepairRequest()).model_dump(mode="json")
    return _submit_operation(db, version_id, "repair", payload)


@router.post("/versions/{version_id}/resize", response_model=JobResponse)
async def submit_resize(version_id: str, request: ResizeRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "resize", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/hollow", response_model=JobResponse)
async def submit_hollow(version_id: str, request: HollowRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "hollow", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/thicken", response_model=JobResponse)
async def submit_thicken(version_id: str, request: ThickenRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "thicken", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/compare", response_model=JobResponse)
async def submit_compare(version_id: str, request: CompareRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "compare", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/make-manufacturable", response_model=JobResponse)
async def submit_make_manufacturable(
    version_id: str,
    request: MakeManufacturableRequest,
    db: Session = Depends(get_db),
) -> JobResponse:
    return _submit_operation(db, version_id, "make_manufacturable", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/scoop", response_model=JobResponse)
async def submit_scoop(version_id: str, request: ScoopRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "scoop", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/smooth", response_model=JobResponse)
async def submit_smooth(version_id: str, request: SmoothRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "smooth", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/decimate", response_model=JobResponse)
async def submit_decimate(version_id: str, request: DecimateRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "decimate", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/subdivide", response_model=JobResponse)
async def submit_subdivide(version_id: str, request: SubdivideRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "subdivide", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/make-delone", response_model=JobResponse)
async def submit_make_delone(version_id: str, request: MakeDeloneRequest, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "make_delone", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/brush-replay", response_model=JobResponse)
async def submit_brush_replay(
    version_id: str,
    request: BrushReplayRequest,
    db: Session = Depends(get_db),
) -> JobResponse:
    return _submit_operation(db, version_id, "interactive_brush_replay", request.model_dump(mode="json"))


@router.post("/versions/{version_id}/interactive-commit", response_model=JobResponse)
async def submit_interactive_commit(
    version_id: str,
    request_json: str = Form(...),
    mesh_file: UploadFile = File(...),
    db: Session = Depends(get_db),
) -> JobResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for a new operation")

    request = InteractiveCommitRequest.model_validate_json(request_json)
    filename = mesh_file.filename or "interactive-edit.ply"
    ext = validate_file_extension(filename)
    if not ext:
        allowed = ", ".join(sorted(settings.ALLOWED_EXTENSIONS))
        raise HTTPException(status_code=400, detail=f"Invalid file type. Allowed: {allowed}")

    mesh_file.file.seek(0, 2)
    size_mb = mesh_file.file.tell() / (1024 * 1024)
    mesh_file.file.seek(0)
    if size_mb > settings.MAX_FILE_SIZE_MB:
        raise HTTPException(status_code=400, detail=f"File too large. Maximum: {settings.MAX_FILE_SIZE_MB}MB")

    payload = request.model_dump(mode="json")
    job = create_job(db, version_id, "interactive_commit", payload)
    upload_dir = settings.TEMP_DIR / "interactive_uploads" / job.id
    upload_dir.mkdir(parents=True, exist_ok=True)
    upload_path = upload_dir / f"interactive-edit{ext}"
    with upload_path.open("wb") as buffer:
        shutil.copyfileobj(mesh_file.file, buffer)

    upload_storage_key = f"interactive_uploads/{job.id}/interactive-edit{ext}"
    object_store.put_file(upload_path, upload_storage_key)
    dispatch_payload = {
        **payload,
        "upload_storage_key": upload_storage_key,
        "uploaded_filename": filename,
    }
    if job.operation_request is not None:
        job.operation_request.payload_json = dispatch_payload
    dispatch_operation_task(db, "interactive_commit", version_id, job.id, dispatch_payload)
    db.commit()
    db.refresh(job)
    return serialize_job(job)
