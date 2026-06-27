"""Application configuration."""

from __future__ import annotations

from pathlib import Path
from typing import Annotated, Literal
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, NoDecode, SettingsConfigDict


class Settings(BaseSettings):
    """Application settings loaded from environment."""

    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore",
    )

    BASE_DIR: Path = Field(default_factory=lambda: Path(__file__).resolve().parent.parent)
    DATA_DIR: Path = Field(default_factory=lambda: Path(__file__).resolve().parent.parent / "data")
    TEMP_DIR: Path = Field(default_factory=lambda: Path(__file__).resolve().parent.parent / "temp")
    STORAGE_DIR: Path = Field(default_factory=lambda: Path(__file__).resolve().parent.parent / "storage")
    MODELS_DIR: Path = Field(default_factory=lambda: Path(__file__).resolve().parent.parent / "models")
    UPLOAD_DIR: Path = Field(default_factory=lambda: Path(__file__).resolve().parent.parent / "uploads")

    DATABASE_URL: str = "sqlite:///./meshinspector.db"
    DIRECT_URL: str | None = None
    DATABASE_ECHO: bool = False
    AUTO_CREATE_SCHEMA: bool = True

    QUEUE_BACKEND: Literal["celery", "database"] = "database"
    COMPAT_PROCESS_ROUTE_ENABLED: bool = False
    DEV_DB_QUEUE_RUNNER_ENABLED: bool = True
    DEV_DB_QUEUE_POLL_INTERVAL_MS: int = 1000
    DEV_DB_QUEUE_BATCH_SIZE: int = 1
    DEV_DB_QUEUE_STALE_LOCK_MS: int = 120000

    # Scale-to-zero worker: instead of polling continuously, the API "wakes" the worker
    # by POSTing to its /internal/drain endpoint after enqueueing a DB job. The worker
    # drains the queue within that request, then Cloud Run scales it back to zero.
    WORKER_WAKE_URL: str | None = None   # set on the API service to the worker's base URL
    WORKER_DRAIN_ENABLED: bool = False   # set true only on the worker service (gates /internal/drain)

    REDIS_URL: str = "redis://localhost:6379/0"
    CELERY_BROKER_URL: str | None = None
    CELERY_RESULT_BACKEND: str | None = None
    CELERY_TASK_ALWAYS_EAGER: bool = False

    OBJECT_STORE_DRIVER: Literal["local", "s3"] = "local"
    OBJECT_STORE_BUCKET: str = "meshinspector"
    OBJECT_STORE_PREFIX: str = "artifacts"
    S3_ENDPOINT_URL: str | None = None
    S3_ACCESS_KEY_ID: str | None = None
    S3_SECRET_ACCESS_KEY: str | None = None
    S3_REGION: str | None = None

    ALLOWED_EXTENSIONS: set[str] = {".glb", ".gltf", ".obj", ".stl", ".ply"}
    MAX_FILE_SIZE_MB: int = 100

    DEFAULT_MATERIAL: str = "gold_18k"
    DEFAULT_WALL_THICKNESS_MM: float = 0.8
    DEFAULT_MIN_THICKNESS_MM: float = 0.6
    MANUFACTURABILITY_THICKNESS_MAX_VERTICES: int = 25_000
    # Decimation runs as an async job on the fast Rust QEM kernel, which produces
    # clean, watertight, volume-preserving output on dense curved meshes (verified
    # on a 994k-face organic snake: 994k->20k, watertight, no spurious geometry).
    # The ceiling guards worker memory/time for absurdly large inputs, not quality.
    MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES: int = 1_500_000
    MESH_EDIT_SUBDIVIDE_MAX_FACES: int = 100_000
    MESH_EDIT_EXACT_BOOLEAN_MAX_INTERACTIVE_FACES: int = 100_000
    MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES: int = 512
    MESH_EDIT_HOLLOW_MAX_FACES: int = 100_000
    MESH_EDIT_HOLLOW_FULL_RESOLUTION_MAX_FACES: int = 100_000

    # Annotated[..., NoDecode] disables pydantic-settings' JSON pre-parsing so the
    # validator below can accept a plain comma-separated string from the environment
    # (e.g. CORS_ORIGINS=https://app.example.com), which is friendlier for Cloud Run
    # `--set-env-vars` than JSON. A JSON array is still accepted.
    CORS_ORIGINS: Annotated[list[str], NoDecode] = [
        "http://localhost:3000",
        "http://127.0.0.1:3000",
        "http://localhost:3001",
        "http://127.0.0.1:3001",
        "http://localhost:3002",
        "http://127.0.0.1:3002",
    ]

    @field_validator("CORS_ORIGINS", mode="before")
    @classmethod
    def _parse_cors_origins(cls, value: object) -> object:
        if isinstance(value, str):
            text = value.strip()
            if not text:
                return []
            if text.startswith("["):
                import json

                return json.loads(text)
            return [origin.strip() for origin in text.split(",") if origin.strip()]
        return value

    @property
    def effective_broker_url(self) -> str:
        return self.CELERY_BROKER_URL or self.REDIS_URL

    @property
    def effective_result_backend(self) -> str:
        return self.CELERY_RESULT_BACKEND or self.REDIS_URL

    @property
    def migration_database_url(self) -> str:
        return _normalize_postgres_url(self.DIRECT_URL or self.DATABASE_URL)

    @property
    def effective_database_url(self) -> str:
        return _normalize_postgres_url(self.DATABASE_URL)

    @property
    def queue_uses_database(self) -> bool:
        return self.QUEUE_BACKEND == "database"

    def ensure_directories(self) -> None:
        self.DATA_DIR.mkdir(parents=True, exist_ok=True)
        self.TEMP_DIR.mkdir(parents=True, exist_ok=True)
        self.STORAGE_DIR.mkdir(parents=True, exist_ok=True)
        self.MODELS_DIR.mkdir(parents=True, exist_ok=True)
        self.UPLOAD_DIR.mkdir(parents=True, exist_ok=True)


settings = Settings()
settings.ensure_directories()


def _normalize_postgres_url(url: str) -> str:
    normalized = url
    if normalized.startswith("postgresql://"):
        normalized = normalized.replace("postgresql://", "postgresql+psycopg://", 1)
    elif normalized.startswith("postgres://"):
        normalized = normalized.replace("postgres://", "postgresql+psycopg://", 1)

    if not normalized.startswith("postgresql+psycopg://"):
        return normalized

    split = urlsplit(normalized)
    filtered_query = [(key, value) for key, value in parse_qsl(split.query, keep_blank_values=True) if key.lower() != "pgbouncer"]
    return urlunsplit((split.scheme, split.netloc, split.path, urlencode(filtered_query), split.fragment))
