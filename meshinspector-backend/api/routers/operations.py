"""Operation submission routes."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session

from api.serializers import serialize_job
from core.db import get_db
from domain.models import ModelVersionRecord
from domain.schemas import (
    CompareRequest,
    HollowRequest,
    JobResponse,
    MakeManufacturableRequest,
    ResizeRequest,
    ScoopRequest,
    SmoothRequest,
    ThickenRequest,
)
from storage.repositories import create_job
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
async def submit_repair(version_id: str, db: Session = Depends(get_db)) -> JobResponse:
    return _submit_operation(db, version_id, "repair", {})


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
