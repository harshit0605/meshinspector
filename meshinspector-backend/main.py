"""
MeshInspector Backend API.

Production-grade entrypoint with versioned ingest, manufacturability snapshots,
and asynchronous geometry operations.
"""

from __future__ import annotations

from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from sqlalchemy import text

from api.routers import jobs, models, operations, versions
from api.routes import analyze, health, process, upload
from core.config import settings
from core.db import Base, engine
from core.logging import setup_logging
from storage.object_store import object_store
from workers.dev_queue import DatabaseQueueRunner

setup_logging()


@asynccontextmanager
async def lifespan(_: FastAPI):
    queue_runner = None
    if settings.AUTO_CREATE_SCHEMA and settings.DATABASE_URL.startswith("sqlite"):
        Base.metadata.create_all(bind=engine)
    object_store.ensure_bucket()
    if settings.queue_uses_database and settings.DEV_DB_QUEUE_RUNNER_ENABLED:
        queue_runner = DatabaseQueueRunner()
        queue_runner.start()
    try:
        yield
    finally:
        if queue_runner is not None:
            queue_runner.stop()


app = FastAPI(
    title="MeshInspector API",
    description="3D jewelry manufacturability analysis",
    version="0.2.0",
    lifespan=lifespan,
)
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.CORS_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(models.router, prefix="/api", tags=["models"])
app.include_router(versions.router, prefix="/api", tags=["versions"])
app.include_router(operations.router, prefix="/api", tags=["operations"])
app.include_router(jobs.router, prefix="/api", tags=["jobs"])

# Compatibility routers preserved for the migration window.
app.include_router(upload.router, prefix="/api", tags=["compat-upload"])
app.include_router(analyze.router, prefix="/api", tags=["compat-analyze"])
app.include_router(process.router, prefix="/api", tags=["compat-process"])
app.include_router(health.router, prefix="/api", tags=["compat-health"])


@app.get("/health")
async def health_check():
    return {"status": "healthy", "version": "0.2.0"}


@app.get("/health/ready")
async def readiness_check():
    with engine.connect() as connection:
        connection.execute(text("select 1"))
    return {
        "status": "ready",
        "database": "ok",
        "object_store_driver": settings.OBJECT_STORE_DRIVER,
        "queue_backend": settings.QUEUE_BACKEND,
    }
