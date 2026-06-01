"""Analysis entrypoints."""

from __future__ import annotations

from geometry_sdk.accelerators import _rust_stats
from geometry_sdk.types import MeshDocument, MeshStats


def compute_mesh_stats(mesh: MeshDocument) -> MeshStats:
    return _rust_stats.mesh_stats(mesh)
