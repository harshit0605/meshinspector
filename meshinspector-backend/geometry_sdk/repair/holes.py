"""Hole repair compatibility wrappers for Rust-owned repair kernels."""

from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import rust
from geometry_sdk.types import (
    HoleComplicatingFacesDiagnostics,
    HoleFillPlanDiagnostics,
    HoleFillReport,
    MeshDocument,
    RepeatedHoleBoundaryVerticesDiagnostics,
    RemoveHoleComplicatingFacesReport,
)


def ordered_boundary_loops(mesh: MeshDocument) -> list[list[int]]:
    return _require_rust(rust.ordered_boundary_loops(mesh), "ordered_boundary_loops")


def hole_fill_plan_diagnostics(mesh: MeshDocument, *, max_edges: int | None = None) -> HoleFillPlanDiagnostics:
    return _require_rust(rust.hole_fill_plan_diagnostics(mesh, max_edges=max_edges), "hole_fill_plan_diagnostics")


def repeated_hole_boundary_vertices_diagnostics(mesh: MeshDocument) -> RepeatedHoleBoundaryVerticesDiagnostics:
    return _require_rust(
        rust.repeated_hole_boundary_vertices_diagnostics(mesh),
        "repeated_hole_boundary_vertices_diagnostics",
    )


def hole_complicating_faces_diagnostics(mesh: MeshDocument) -> HoleComplicatingFacesDiagnostics:
    return _require_rust(
        rust.hole_complicating_faces_diagnostics(mesh),
        "hole_complicating_faces_diagnostics",
    )


def remove_hole_complicating_faces(mesh: MeshDocument) -> tuple[MeshDocument, RemoveHoleComplicatingFacesReport]:
    return _require_rust(
        rust.remove_hole_complicating_faces(mesh),
        "remove_hole_complicating_faces",
    )


def fill_planar_holes(mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport]:
    return _require_rust(rust.fill_planar_holes(mesh, max_edges=max_edges), "fill_planar_holes")


def service_fill_holes(
    mesh: MeshDocument,
    *,
    max_edges: int | None = None,
    max_polygon_subdivisions: int | None = None,
    multiple_edges_resolve_mode: str | None = None,
    make_degenerate_band: bool = False,
    stop_before_bad_triangulation: bool = False,
    smooth_bd: bool = True,
    fill_metric: str | None = None,
    fill_metric_up_dir: tuple[float, float, float] | list[float] | None = None,
) -> tuple[MeshDocument, HoleFillReport]:
    return _require_rust(
        rust.service_fill_holes(
            mesh,
            max_edges=max_edges,
            max_polygon_subdivisions=max_polygon_subdivisions,
            multiple_edges_resolve_mode=multiple_edges_resolve_mode,
            make_degenerate_band=make_degenerate_band,
            stop_before_bad_triangulation=stop_before_bad_triangulation,
            smooth_bd=smooth_bd,
            fill_metric=fill_metric,
            fill_metric_up_dir=fill_metric_up_dir,
        ),
        "service_fill_holes",
    )


def _require_rust(value: Any, kernel_name: str) -> Any:
    if value is None:
        raise RuntimeError(
            f"Rust kernel {kernel_name} is required for geometry_sdk.repair.holes. "
            "Build the extension with `uv tool run maturin develop --manifest-path "
            "geometry-rs/crates/zennah-geometry-py/Cargo.toml`."
        )
    return value
