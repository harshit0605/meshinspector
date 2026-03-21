"""Persistent domain models."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from sqlalchemy import JSON, DateTime, ForeignKey, Integer, String, Text, func
from sqlalchemy.orm import Mapped, mapped_column, relationship

from core.db import Base


class ModelRecord(Base):
    __tablename__ = "models"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    source_filename: Mapped[str] = mapped_column(String(255))
    source_type: Mapped[str] = mapped_column(String(32))
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    versions: Mapped[list["ModelVersionRecord"]] = relationship(back_populates="model", cascade="all, delete-orphan")


class ModelVersionRecord(Base):
    __tablename__ = "model_versions"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    model_id: Mapped[str] = mapped_column(ForeignKey("models.id"), index=True)
    parent_version_id: Mapped[str | None] = mapped_column(ForeignKey("model_versions.id"), nullable=True)
    operation_type: Mapped[str] = mapped_column(String(64))
    operation_label: Mapped[str] = mapped_column(String(255))
    status: Mapped[str] = mapped_column(String(32), default="ready", index=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    model: Mapped[ModelRecord] = relationship(back_populates="versions", foreign_keys=[model_id])
    parent_version: Mapped["ModelVersionRecord | None"] = relationship(remote_side=[id], foreign_keys=[parent_version_id])
    artifacts: Mapped[list["ModelArtifactRecord"]] = relationship(back_populates="version", cascade="all, delete-orphan")
    snapshots: Mapped[list["AnalysisSnapshotRecord"]] = relationship(back_populates="version", cascade="all, delete-orphan")
    jobs: Mapped[list["JobRecord"]] = relationship(back_populates="version", cascade="all, delete-orphan")


class ModelArtifactRecord(Base):
    __tablename__ = "model_artifacts"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    version_id: Mapped[str] = mapped_column(ForeignKey("model_versions.id"), index=True)
    artifact_type: Mapped[str] = mapped_column(String(64), index=True)
    mime_type: Mapped[str] = mapped_column(String(128))
    storage_key: Mapped[str] = mapped_column(String(512), unique=True)
    size_bytes: Mapped[int] = mapped_column(Integer, default=0)
    metadata_json: Mapped[dict[str, Any]] = mapped_column(JSON, default=dict)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    version: Mapped[ModelVersionRecord] = relationship(back_populates="artifacts")


class AnalysisSnapshotRecord(Base):
    __tablename__ = "analysis_snapshots"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    version_id: Mapped[str] = mapped_column(ForeignKey("model_versions.id"), index=True)
    snapshot_type: Mapped[str] = mapped_column(String(64), index=True)
    payload_json: Mapped[dict[str, Any]] = mapped_column(JSON)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    version: Mapped[ModelVersionRecord] = relationship(back_populates="snapshots")


class JobRecord(Base):
    __tablename__ = "jobs"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    version_id: Mapped[str] = mapped_column(ForeignKey("model_versions.id"), index=True)
    operation_type: Mapped[str] = mapped_column(String(64), index=True)
    status: Mapped[str] = mapped_column(String(32), default="queued", index=True)
    progress_pct: Mapped[int] = mapped_column(Integer, default=0)
    error_code: Mapped[str | None] = mapped_column(String(64), nullable=True)
    error_message: Mapped[str | None] = mapped_column(Text, nullable=True)
    started_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    finished_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    version: Mapped[ModelVersionRecord] = relationship(back_populates="jobs")
    events: Mapped[list["JobEventRecord"]] = relationship(back_populates="job", cascade="all, delete-orphan")
    operation_request: Mapped["OperationRequestRecord | None"] = relationship(back_populates="job", cascade="all, delete-orphan", uselist=False)


class JobEventRecord(Base):
    __tablename__ = "job_events"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    job_id: Mapped[str] = mapped_column(ForeignKey("jobs.id"), index=True)
    level: Mapped[str] = mapped_column(String(16), default="info")
    message: Mapped[str] = mapped_column(Text)
    progress_pct: Mapped[int | None] = mapped_column(Integer, nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    job: Mapped[JobRecord] = relationship(back_populates="events")


class OperationRequestRecord(Base):
    __tablename__ = "operation_requests"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    job_id: Mapped[str] = mapped_column(ForeignKey("jobs.id"), unique=True, index=True)
    payload_json: Mapped[dict[str, Any]] = mapped_column(JSON)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())

    job: Mapped[JobRecord] = relationship(back_populates="operation_request")


class DevTaskQueueRecord(Base):
    __tablename__ = "dev_task_queue"

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    task_name: Mapped[str] = mapped_column(String(64), index=True)
    job_id: Mapped[str | None] = mapped_column(ForeignKey("jobs.id"), unique=True, index=True, nullable=True)
    payload_json: Mapped[dict[str, Any]] = mapped_column(JSON)
    status: Mapped[str] = mapped_column(String(32), default="queued", index=True)
    attempts: Mapped[int] = mapped_column(Integer, default=0)
    available_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now(), index=True)
    locked_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    locked_by: Mapped[str | None] = mapped_column(String(128), nullable=True)
    error_message: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), server_default=func.now())
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True),
        server_default=func.now(),
        onupdate=func.now(),
    )
