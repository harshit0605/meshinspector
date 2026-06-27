"""Object storage abstraction."""

from __future__ import annotations

import logging
import mimetypes
import shutil
from pathlib import Path

import boto3
from botocore.config import Config

from core.config import settings

logger = logging.getLogger(__name__)


def _s3_client_config() -> Config:
    """boto3 config tuned for non-AWS S3 (GCS interop, MinIO).

    botocore >= 1.36 adds automatic CRC32 integrity checksums to uploads that GCS's
    S3-interoperability XML API rejects with SignatureDoesNotMatch; restrict checksums
    to operations that require them. Falls back gracefully on older botocore.
    """
    kwargs = {"signature_version": "s3v4"}
    try:
        return Config(
            request_checksum_calculation="when_required",
            response_checksum_validation="when_required",
            **kwargs,
        )
    except (TypeError, ValueError):  # botocore too old for the checksum knobs
        return Config(**kwargs)


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
                config=_s3_client_config(),
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
        bucket = settings.OBJECT_STORE_BUCKET
        # head_bucket is the cheapest existence/permission check and, unlike
        # list_buckets, works on GCS's S3-interoperability XML API with a single-
        # bucket-scoped HMAC key.
        try:
            self._client.head_bucket(Bucket=bucket)
            return  # exists and reachable
        except Exception:  # noqa: BLE001 - missing, or list/head not permitted
            pass
        # Best-effort create. The bucket is usually pre-created out of band (GCS's
        # S3-compat layer may not support create-by-API), so tolerate failures here
        # rather than crashing startup — a genuine misconfiguration surfaces on the
        # first put/get instead.
        try:
            self._client.create_bucket(Bucket=bucket)
        except Exception as exc:  # noqa: BLE001
            logger.warning(
                "ensure_bucket: could not verify/create bucket %r (%s); "
                "assuming it is pre-provisioned. Object ops will fail if it is not.",
                bucket,
                exc,
            )

    @staticmethod
    def guess_content_type(path: Path) -> str:
        return mimetypes.guess_type(str(path))[0] or "application/octet-stream"


object_store = ObjectStore()
