"""File I/O utilities."""

import uuid
import shutil
from pathlib import Path
from typing import Optional

from core.config import settings


def generate_model_id() -> str:
    """Generate a unique model identifier."""
    return str(uuid.uuid4())[:8]


def get_upload_path(model_id: str, extension: str) -> Path:
    """Get the path for an uploaded file."""
    return settings.UPLOAD_DIR / f"{model_id}{extension}"


def get_temp_path(model_id: str, suffix: str = "") -> Path:
    """Get a temporary file path."""
    return settings.TEMP_DIR / f"{model_id}{suffix}"


def get_stl_path(model_id: str) -> Path:
    """Get path for the canonical STL file."""
    return settings.TEMP_DIR / f"{model_id}.stl"


def get_output_glb_path(model_id: str) -> Path:
    """Get path for output GLB file."""
    return settings.TEMP_DIR / f"{model_id}_output.glb"


def get_output_stl_path(model_id: str) -> Path:
    """Get path for output STL file."""
    return settings.TEMP_DIR / f"{model_id}_output.stl"


def cleanup_temp_files(model_id: str) -> None:
    """Remove all temporary files for a model."""
    patterns = [
        f"{model_id}*",
    ]
    for pattern in patterns:
        for file in settings.TEMP_DIR.glob(pattern):
            try:
                file.unlink()
            except Exception:
                pass


def copy_to_uploads(source: Path, model_id: str) -> Path:
    """Copy a file to the uploads directory."""
    dest = settings.UPLOAD_DIR / f"{model_id}{source.suffix}"
    shutil.copy2(source, dest)
    return dest


def validate_file_extension(filename: str) -> Optional[str]:
    """Validate file extension. Returns normalized extension or None if invalid."""
    ext = Path(filename).suffix.lower()
    if ext in settings.ALLOWED_EXTENSIONS:
        return ext
    return None
