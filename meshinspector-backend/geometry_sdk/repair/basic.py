"""Basic mesh healing compatibility wrappers for Rust-owned repair kernels."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import (
    ComponentPruneReport,
    CreaseEdgeDiagnostics,
    CreaseRepairPlanDiagnostics,
    DegenerateFaceDiagnostics,
    DuplicateNonManifoldVerticesReport,
    DuplicateMultiHoleVerticesReport,
    FixMeshCreasesReport,
    MeshDocument,
    MeshHealerReport,
    MultipleEdgeDiagnostics,
    MultipleEdgeRepairReport,
    NonManifoldEdgeRepairReport,
    NotSmoothFaceDiagnostics,
    RepairReport,
    ShortEdgeDiagnostics,
    TunnelDiagnostics,
    TunnelEliminationReport,
)

DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS = 3.0543261909900767


def remove_degenerate_faces(mesh: MeshDocument, *, area_epsilon: float = 1e-12) -> tuple[MeshDocument, int]:
    return _require_rust(rust.remove_degenerate_faces(mesh, area_epsilon=area_epsilon), "remove_degenerate_faces")


def remove_unreferenced_vertices(mesh: MeshDocument) -> tuple[MeshDocument, int]:
    return _require_rust(rust.remove_unreferenced_vertices(mesh), "remove_unreferenced_vertices")


def merge_close_vertices(mesh: MeshDocument, *, tolerance: float = 1e-6) -> tuple[MeshDocument, int]:
    return _require_rust(rust.merge_close_vertices(mesh, tolerance=tolerance), "merge_close_vertices")


def unite_close_vertices(
    mesh: MeshDocument,
    *,
    close_dist: float = 0.0,
    unite_only_boundary: bool = True,
) -> tuple[MeshDocument, int]:
    return _require_rust(
        rust.unite_close_vertices(mesh, close_dist=close_dist, unite_only_boundary=unite_only_boundary),
        "unite_close_vertices",
    )


def orient_faces_outward(mesh: MeshDocument) -> MeshDocument:
    return _require_rust(rust.orient_faces_outward(mesh), "orient_faces_outward")


def flip_normals(mesh: MeshDocument) -> MeshDocument:
    return _require_rust(rust.flip_normals(mesh), "flip_normals")


def find_disoriented_faces(
    mesh: MeshDocument,
    *,
    ray_mode: str = "shallowest",
    epsilon: float = 1e-8,
) -> list[int]:
    return _require_rust(
        rust.find_disoriented_faces(mesh, ray_mode=ray_mode, epsilon=epsilon),
        "find_disoriented_faces",
    )


def basic_repair(mesh: MeshDocument, *, merge_tolerance: float = 1e-6, area_epsilon: float = 1e-12) -> tuple[MeshDocument, RepairReport]:
    """Run deterministic low-risk repair passes.

    This intentionally does not fill holes or repair self-intersections. Those
    passes require the spatial/topology kernels planned for later phases.
    """

    return _require_rust(
        rust.basic_repair(mesh, merge_tolerance=merge_tolerance, area_epsilon=area_epsilon),
        "basic_repair",
    )


def mesh_healer_diagnostics(
    mesh: MeshDocument,
    *,
    merge_tolerance: float = 1e-6,
    area_epsilon: float = 1e-12,
    detect_self_intersections: bool = True,
    max_self_intersection_faces: int | None = 50000,
    epsilon: float = 1e-8,
) -> MeshHealerReport:
    return _require_rust(
        rust.mesh_healer_diagnostics(
            mesh,
            merge_tolerance=merge_tolerance,
            area_epsilon=area_epsilon,
            detect_self_intersections=detect_self_intersections,
            max_self_intersection_faces=max_self_intersection_faces,
            epsilon=epsilon,
        ),
        "mesh_healer_diagnostics",
    )


def tunnel_diagnostics(mesh: MeshDocument) -> TunnelDiagnostics:
    return _require_rust(
        rust.tunnel_diagnostics(mesh),
        "tunnel_diagnostics",
    )


def detect_tunnel_faces(mesh: MeshDocument) -> list[int]:
    return _require_rust(
        rust.detect_tunnel_faces(mesh),
        "detect_tunnel_faces",
    )


def eliminate_tunnels(mesh: MeshDocument) -> tuple[MeshDocument, TunnelEliminationReport]:
    return _require_rust(
        rust.eliminate_tunnels(mesh),
        "eliminate_tunnels",
    )


def prune_small_components(
    mesh: MeshDocument,
    *,
    min_area_mm2: float = 0.0,
) -> tuple[MeshDocument, ComponentPruneReport]:
    return _require_rust(
        rust.prune_small_components(mesh, min_area_mm2=min_area_mm2),
        "prune_small_components",
    )


def weld_coincident_vertices(
    mesh: MeshDocument,
    *,
    eps_mm: float = 1e-5,
) -> tuple[MeshDocument, dict[str, Any]]:
    return _require_rust(
        rust.weld_coincident_vertices(mesh, eps_mm=eps_mm),
        "weld_coincident_vertices",
    )


def repaired_surface_area(mesh: MeshDocument) -> float:
    return _require_rust(rust.repaired_surface_area(mesh), "repaired_surface_area")


def short_edge_diagnostics(mesh: MeshDocument, *, critical_length_mm: float) -> ShortEdgeDiagnostics:
    return _require_rust(
        rust.short_edge_diagnostics(mesh, critical_length_mm=critical_length_mm),
        "short_edge_diagnostics",
    )


def degenerate_face_diagnostics(mesh: MeshDocument, *, critical_aspect_ratio: float) -> DegenerateFaceDiagnostics:
    return _require_rust(
        rust.degenerate_face_diagnostics(mesh, critical_aspect_ratio=critical_aspect_ratio),
        "degenerate_face_diagnostics",
    )


def multiple_edge_diagnostics(mesh: MeshDocument) -> MultipleEdgeDiagnostics:
    return _require_rust(
        rust.multiple_edge_diagnostics(mesh),
        "multiple_edge_diagnostics",
    )


def repair_multiple_edges(mesh: MeshDocument) -> tuple[MeshDocument, MultipleEdgeRepairReport]:
    return _require_rust(
        rust.repair_multiple_edges(mesh),
        "repair_multiple_edges",
    )


def repair_nonmanifold_edges(mesh: MeshDocument) -> tuple[MeshDocument, NonManifoldEdgeRepairReport]:
    return _require_rust(
        rust.repair_nonmanifold_edges(mesh),
        "repair_nonmanifold_edges",
    )


def duplicate_nonmanifold_vertices(
    mesh: MeshDocument,
    *,
    region_face_indices: Sequence[int] | None = None,
) -> tuple[MeshDocument, DuplicateNonManifoldVerticesReport]:
    return _require_rust(
        rust.duplicate_nonmanifold_vertices(mesh, region_face_indices=region_face_indices),
        "duplicate_nonmanifold_vertices",
    )


def duplicate_multi_hole_vertices(mesh: MeshDocument) -> tuple[MeshDocument, DuplicateMultiHoleVerticesReport]:
    return _require_rust(
        rust.duplicate_multi_hole_vertices(mesh),
        "duplicate_multi_hole_vertices",
    )


def not_smooth_face_diagnostics(mesh: MeshDocument, *, min_angle_radians: float = 0.3) -> NotSmoothFaceDiagnostics:
    return _require_rust(
        rust.not_smooth_face_diagnostics(mesh, min_angle_radians=min_angle_radians),
        "not_smooth_face_diagnostics",
    )


def crease_edge_diagnostics(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
    min_component_length_mm: float | None = None,
    min_branch_length_mm: float | None = None,
) -> CreaseEdgeDiagnostics:
    return _require_rust(
        rust.crease_edge_diagnostics(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            min_component_length_mm=min_component_length_mm,
            min_branch_length_mm=min_branch_length_mm,
        ),
        "crease_edge_diagnostics",
    )


def crease_repair_plan_diagnostics(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
    critical_tri_aspect_ratio: float = 1e3,
) -> CreaseRepairPlanDiagnostics:
    return _require_rust(
        rust.crease_repair_plan_diagnostics(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            critical_tri_aspect_ratio=critical_tri_aspect_ratio,
        ),
        "crease_repair_plan_diagnostics",
    )


def fix_mesh_creases(
    mesh: MeshDocument,
    *,
    angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
    critical_tri_aspect_ratio: float = 1e3,
    max_iters: int = 10,
) -> tuple[MeshDocument, FixMeshCreasesReport]:
    return _require_rust(
        rust.fix_mesh_creases(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            critical_tri_aspect_ratio=critical_tri_aspect_ratio,
            max_iters=max_iters,
        ),
        "fix_mesh_creases",
    )


def _require_rust(value: Any, kernel_name: str) -> Any:
    if value is None:
        raise RuntimeError(
            f"Rust kernel {kernel_name} is required for geometry_sdk.repair.basic. "
            "Build the extension with `uv tool run maturin develop --manifest-path "
            "geometry-rs/crates/zennah-geometry-py/Cargo.toml`."
        )
    return value
