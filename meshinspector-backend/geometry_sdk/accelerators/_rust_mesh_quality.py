"""Rust-backed output-quality verdicts for mesh operations."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def decimate_output_failures(source: MeshDocument, output: MeshDocument) -> list[str]:
    kernel = _require_rust_kernel("decimate_output_failures")
    return list(kernel(source.vertices, source.faces, output.vertices, output.faces))


def hollow_output_failures(source: MeshDocument, output: MeshDocument, wall_thickness_mm: float) -> list[str]:
    kernel = _require_rust_kernel("hollow_output_failures")
    return list(kernel(source.vertices, source.faces, output.vertices, output.faces, float(wall_thickness_mm)))


def offset_shell_failures(source: MeshDocument, output: MeshDocument) -> list[str]:
    kernel = _require_rust_kernel("offset_shell_failures")
    return list(kernel(source.vertices, source.faces, output.vertices, output.faces))


def boolean_output_failures(
    output: MeshDocument,
    operation: str,
    source_volume_mm3: float,
    target_volume_mm3: float,
) -> list[str]:
    kernel = _require_rust_kernel("boolean_output_failures")
    return list(
        kernel(
            output.vertices,
            output.faces,
            str(operation),
            float(source_volume_mm3),
            float(target_volume_mm3),
        )
    )
