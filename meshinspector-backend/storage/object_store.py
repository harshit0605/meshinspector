"""Object storage abstraction."""

from __future__ import annotations

import mimetypes
import shutil
from pathlib import Path

import boto3

from core.config import settings


class ObjectStore:
    """Simple object store wrapper with local and S3 implementations."""

    def __init__(self) -> None:
        self.driver = settings.OBJECT_STORE_DRIVER
        self.base_dir = settings.STORAGE_DIR
        self.base_dir.mkdir(parents=True, exist_ok=True)
        self._client = None
        if self.driver == "s3":
            self._client = boto3.client(
                "s3",
                endpoint_url=settings.S3_ENDPOINT_URL,
                aws_access_key_id=settings.S3_ACCESS_KEY_ID,
                aws_secret_access_key=settings.S3_SECRET_ACCESS_KEY,
                region_name=settings.S3_REGION,
            )

    def put_file(self, source: Path, key: str, content_type: str | None = None) -> int:
        source = Path(source)
        if self.driver == "s3":
            self._client.upload_file(
                str(source),
                settings.OBJECT_STORE_BUCKET,
                key,
                ExtraArgs={"ContentType": content_type or self.guess_content_type(source)},
            )
        else:
            dest = self.base_dir / key
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, dest)
        return source.stat().st_size

    def get_local_path(self, key: str) -> Path:
        if self.driver != "local":
            raise RuntimeError("Local path is only available for local object storage driver")
        return self.base_dir / key

    def download_to_path(self, key: str, destination: Path) -> Path:
        destination = Path(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if self.driver == "s3":
            self._client.download_file(settings.OBJECT_STORE_BUCKET, key, str(destination))
        else:
            shutil.copy2(self.base_dir / key, destination)
        return destination

    def ensure_bucket(self) -> None:
        if self.driver != "s3":
            return
        existing = {bucket["Name"] for bucket in self._client.list_buckets().get("Buckets", [])}
        if settings.OBJECT_STORE_BUCKET not in existing:
            self._client.create_bucket(Bucket=settings.OBJECT_STORE_BUCKET)

    @staticmethod
    def guess_content_type(path: Path) -> str:
        return mimetypes.guess_type(str(path))[0] or "application/octet-stream"


object_store = ObjectStore()
