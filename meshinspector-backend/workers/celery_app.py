"""Celery application."""

from __future__ import annotations

from celery import Celery

from core.config import settings

celery_app = Celery(
    "meshinspector",
    broker=settings.effective_broker_url,
    backend=settings.effective_result_backend,
)
celery_app.conf.update(
    task_always_eager=settings.CELERY_TASK_ALWAYS_EAGER,
    task_track_started=True,
    accept_content=["json"],
    task_serializer="json",
    result_serializer="json",
)
