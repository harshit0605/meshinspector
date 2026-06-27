"""Self-intersection repair compatibility wrappers."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_repair
from geometry_sdk.types import FixSelfIntersectionsRelaxReport, MeshDocument


def fix_self_intersections_relax(
    mesh: MeshDocument,
    *,
    relax_iterations: int = 5,
    max_expand: int = 3,
    touch_is_intersection: bool = True,
    force: float = 0.5,
    epsilon: float = 1e-8,
) -> tuple[MeshDocument, FixSelfIntersectionsRelaxReport]:
    result = _rust_repair.fix_self_intersections_relax(
        mesh,
        relax_iterations=relax_iterations,
        max_expand=max_expand,
        touch_is_intersection=touch_is_intersection,
        force=force,
        epsilon=epsilon,
    )
    if result is None:
        raise RuntimeError("Rust kernel fix_self_intersections_relax is required")
    return result
