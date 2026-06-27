"""Job routes."""

from __future__ import annotations

import asyncio

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import StreamingResponse
from sqlalchemy import select
from sqlalchemy.orm import Session

from api.serializers import serialize_job, serialize_job_event
from core.db import get_db
from core.db import SessionLocal
from domain.models import JobRecord, ModelVersionRecord
from domain.schemas import JobResponse
from storage.repositories import get_job_events

router = APIRouter()


@router.get("/jobs/{job_id}", response_model=JobResponse)
async def get_job(job_id: str, db: Session = Depends(get_db)):
    job = db.get(JobRecord, job_id)
    if job is None:
        raise HTTPException(status_code=404, detail="Job not found")
    return serialize_job(job)


@router.get("/versions/{version_id}/jobs", response_model=list[JobResponse])
async def list_version_jobs(version_id: str, db: Session = Depends(get_db)) -> list[JobResponse]:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    jobs = db.scalars(
        select(JobRecord)
        .where(JobRecord.version_id == version_id)
        .order_by(JobRecord.created_at.desc())
    ).all()
    return [serialize_job(job) for job in jobs]


@router.get("/jobs/{job_id}/events")
async def stream_job_events(job_id: str, db: Session = Depends(get_db)):
    job = db.get(JobRecord, job_id)
    if job is None:
        raise HTTPException(status_code=404, detail="Job not found")

    async def event_stream():
        last_seen = None
        while True:
            with SessionLocal() as stream_db:
                current_job = stream_db.get(JobRecord, job_id)
                if current_job is None:
                    break
                events = get_job_events(stream_db, job_id, after_created_at=last_seen)
                for event in events:
                    last_seen = event.created_at
                    payload = serialize_job_event(event).model_dump_json()
                    yield f"data: {payload}\n\n"
                if current_job.status in {"succeeded", "failed"}:
                    yield f"event: status\ndata: {serialize_job(current_job).model_dump_json()}\n\n"
                    break
            await asyncio.sleep(1.0)

    return StreamingResponse(
        event_stream(),
        media_type="text/event-stream",
        headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"},
    )
