"""Rust-backed output-quality verdicts for mesh operations.

Each returns the human-readable failure clauses (empty => acceptable). The
threshold/acceptance logic lives in the Rust `mesh_quality` kernel; callers only
join the clauses and reject the job when the list is non-empty.
"""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_mesh_quality
from geometry_sdk.types import MeshDocument


def decimate_output_failures(source: MeshDocument, output: MeshDocument) -> list[str]:
    return _rust_mesh_quality.decimate_output_failures(source, output)


def hollow_output_failures(source: MeshDocument, output: MeshDocument, wall_thickness_mm: float) -> list[str]:
    return _rust_mesh_quality.hollow_output_failures(source, output, wall_thickness_mm)


def offset_shell_failures(source: MeshDocument, output: MeshDocument) -> list[str]:
    return _rust_mesh_quality.offset_shell_failures(source, output)


def boolean_output_failures(
    output: MeshDocument,
    operation: str,
    source_volume_mm3: float,
    target_volume_mm3: float,
) -> list[str]:
    return _rust_mesh_quality.boolean_output_failures(
        output, operation, source_volume_mm3, target_volume_mm3
    )
