"""Application configuration."""

from __future__ import annotations

from pathlib import Path
from typing import Literal
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


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
    DEV_DB_QUEUE_RUNNER_ENABLED: bool = True
    DEV_DB_QUEUE_POLL_INTERVAL_MS: int = 1000
    DEV_DB_QUEUE_BATCH_SIZE: int = 1
    DEV_DB_QUEUE_STALE_LOCK_MS: int = 120000

    REDIS_URL: str = "redis://localhost:6379/0"
    CELERY_BROKER_URL: str | None = None
    CELERY_RESULT_BACKEND: str | None = None
    CELERY_TASK_ALWAYS_EAGER: bool = True

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

    CORS_ORIGINS: list[str] = [
        "http://localhost:3000",
        "http://127.0.0.1:3000",
        "http://localhost:3001",
        "http://127.0.0.1:3001",
        "http://localhost:3002",
        "http://127.0.0.1:3002",
    ]

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
