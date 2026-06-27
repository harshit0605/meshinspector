"""
MeshInspector Backend API.

Production-grade entrypoint with versioned ingest, manufacturability snapshots,
and asynchronous geometry operations.
"""

from __future__ import annotations

from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi import Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from sqlalchemy import text

from api.routers import jobs, models, operations, versions
from api.routes import analyze, health, process, upload
from core.config import settings
from core.db import Base, engine
from core.logging import get_logger, setup_logging
from storage.object_store import object_store
from workers.dev_queue import DatabaseQueueRunner, reconcile_database_queue_state

setup_logging()
logger = get_logger(__name__)


@asynccontextmanager
async def lifespan(_: FastAPI):
    queue_runner = None
    if settings.AUTO_CREATE_SCHEMA and settings.DATABASE_URL.startswith("sqlite"):
        Base.metadata.create_all(bind=engine)
    object_store.ensure_bucket()
    if settings.queue_uses_database and settings.DEV_DB_QUEUE_RUNNER_ENABLED:
        reconcile_database_queue_state()
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
if settings.COMPAT_PROCESS_ROUTE_ENABLED:
    app.include_router(process.router, prefix="/api", tags=["compat-process"])
app.include_router(health.router, prefix="/api", tags=["compat-health"])


@app.exception_handler(Exception)
async def unhandled_exception_handler(request: Request, exc: Exception):
    logger.exception("Unhandled API error on %s %s: %s", request.method, request.url.path, exc)
    return JSONResponse(status_code=500, content={"detail": "Internal server error"})


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


@app.post("/internal/drain")
async def drain_queue():
    """Drain the database job queue until empty, then return.

    The API POSTs here (on the worker service) after enqueueing a job, so the worker
    can scale to zero between jobs instead of polling 24/7. Gated by WORKER_DRAIN_ENABLED
    (only the worker sets it) and, in production, by Cloud Run IAM (run.invoker). Jobs run
    off the event loop; the loop is bounded so a flood can't hold the request open forever
    (the scheduler backstop and subsequent wakes pick up any remainder).
    """
    if not settings.WORKER_DRAIN_ENABLED:
        return JSONResponse(status_code=403, content={"detail": "drain disabled on this service"})
    from starlette.concurrency import run_in_threadpool

    from workers.dev_queue import run_database_queue_once

    processed = 0
    while processed < 500:
        did_work = await run_in_threadpool(run_database_queue_once)
        if not did_work:
            break
        processed += 1
    return {"processed": processed}
