"""Section contour compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_analysis
from geometry_sdk.types import MeshDocument, SectionContourPayload


def section_contour(
    mesh: MeshDocument,
    *,
    section_constant: float,
    plane_axis: tuple[float, float, float] = (0.0, 1.0, 0.0),
    selected_vertex_indices: Any = None,
    epsilon: float = 1e-5,
) -> SectionContourPayload:
    contour = _rust_analysis.section_contour(
        mesh,
        section_constant=section_constant,
        plane_axis=plane_axis,
        selected_vertex_indices=selected_vertex_indices,
        epsilon=epsilon,
    )
    if contour is None:
        raise RuntimeError("Rust section_contour kernel is unavailable")
    return contour
