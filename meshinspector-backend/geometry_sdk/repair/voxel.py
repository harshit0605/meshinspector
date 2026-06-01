"""Voxel/SDF rebuild repair compatibility wrappers."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_repair
from geometry_sdk.types import MeshDocument, VoxelRebuildReport


def rebuild_via_sdf(
    mesh: MeshDocument,
    *,
    voxel_size_mm: float,
    offset_mm: float = 0.0,
    padding_mm: float | None = None,
    extractor: str = "marching",
    refine: bool = True,
) -> tuple[MeshDocument, VoxelRebuildReport]:
    return _rust_repair.rebuild_via_sdf(
        mesh,
        voxel_size_mm=voxel_size_mm,
        offset_mm=offset_mm,
        padding_mm=padding_mm,
        extractor=extractor,
        refine=refine,
    )
