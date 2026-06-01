"""Global thickening compatibility wrapper."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_mesh_ops
from geometry_sdk.types import MeshDocument


def global_thicken(mesh: MeshDocument, *, min_target_thickness_mm: float) -> MeshDocument:
    return _rust_mesh_ops.global_thicken_mesh(
        mesh,
        min_target_thickness_mm=min_target_thickness_mm,
    )
