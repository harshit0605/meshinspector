from __future__ import annotations

import numpy as np
import os
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.accelerators import rust
from geometry_sdk.core.mesh import signed_volume
from geometry_sdk.repair.basic import basic_repair, orient_faces_outward, remove_degenerate_faces
from geometry_sdk.repair.holes import fill_planar_holes, ordered_boundary_loops, service_fill_holes
from geometry_sdk.repair.voxel import rebuild_via_sdf
from geometry_sdk.analysis.health import compute_mesh_health, service_mesh_health
from geometry_sdk.testing.fixtures import crossing_triangles, open_cube, ring
from geometry_sdk.types import MeshDocument


def damaged_mesh() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [10.0, 10.0, 10.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 2],
            [0, 3, 1],
            [0, 0, 1],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def test_basic_repair_merges_duplicates_and_removes_degenerate_faces() -> None:
    repaired, report = basic_repair(damaged_mesh(), merge_tolerance=1e-8)

    assert repaired.vertex_count == 3
    assert repaired.face_count == 1
    assert report.merged_vertices == 1
    assert report.removed_degenerate_faces == 2
    assert report.removed_unreferenced_vertices == 1


def test_basic_repair_module_is_rust_owned(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = damaged_mesh()
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")
    repaired, removed = remove_degenerate_faces(mesh, area_epsilon=1e-12)
    assert removed == 2
    assert repaired.face_count == 1

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    repaired_again, report = basic_repair(mesh)
    assert repaired_again.vertex_count == 3
    assert report.merged_vertices == 1


def test_engine_exposes_basic_repair() -> None:
    sdk = GeometrySDK()
    repaired, report = sdk.basic_repair(damaged_mesh())

    assert repaired.vertex_count == report.output_vertex_count
    assert repaired.face_count == report.output_face_count


def test_service_mesh_health_matches_current_meshlib_payload_contract(monkeypatch) -> None:
    health = service_mesh_health(crossing_triangles(), max_listed_faces=1)

    assert not health.is_closed
    assert health.self_intersections == 2
    assert health.self_intersection_faces == [0]
    assert health.holes_count == 2
    assert health.degenerate_faces == 0
    assert health.health_score == 56

    sdk = GeometrySDK()
    assert sdk.service_health(crossing_triangles(), max_listed_faces=1).health_score == 56

    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel service_mesh_health is required"):
        service_mesh_health(crossing_triangles())


def test_orient_faces_outward_flips_negative_signed_volume_mesh() -> None:
    inward = ring(radial_segments=16, tube_segments=8)
    oriented = orient_faces_outward(inward)

    assert signed_volume(inward) < 0.0
    assert signed_volume(oriented) > 0.0
    assert oriented.vertex_count == inward.vertex_count
    assert oriented.face_count == inward.face_count
    assert compute_mesh_health(oriented).is_closed


def test_engine_exposes_face_orientation_repair() -> None:
    sdk = GeometrySDK()
    oriented = sdk.orient_faces_outward(ring(radial_segments=16, tube_segments=8))

    assert signed_volume(oriented) > 0.0


def test_ordered_boundary_loops_find_open_cube_hole() -> None:
    loops = ordered_boundary_loops(open_cube(size=2.0))

    assert len(loops) == 1
    assert len(loops[0]) == 4


def test_fill_planar_holes_closes_open_cube() -> None:
    repaired, report = fill_planar_holes(open_cube(size=2.0))
    health = compute_mesh_health(repaired)

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_vertices == 1
    assert report.added_faces == 4
    assert health.is_closed
    assert health.holes_count == 0


def test_service_fill_holes_matches_meshlib_style_existing_vertex_patch() -> None:
    repaired, report = service_fill_holes(open_cube(size=2.0))
    health = compute_mesh_health(repaired)

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_vertices == 0
    assert report.added_faces == 2
    assert repaired.vertex_count == open_cube(size=2.0).vertex_count
    assert health.is_closed
    assert health.holes_count == 0


def test_engine_exposes_planar_hole_fill() -> None:
    sdk = GeometrySDK()
    repaired, report = sdk.fill_planar_holes(open_cube(size=2.0))

    assert report.filled_holes == 1
    assert sdk.health(repaired).is_closed

    service_repaired, service_report = sdk.service_fill_holes(open_cube(size=2.0))
    assert service_report.added_faces == 2
    assert sdk.health(service_repaired).is_closed


def test_hole_repair_module_is_rust_owned(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = open_cube(size=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    assert len(ordered_boundary_loops(mesh)) == 1
    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel fill_planar_holes is required"):
        fill_planar_holes(mesh)
    with pytest.raises(RuntimeError, match="Rust kernel service_fill_holes is required"):
        service_fill_holes(mesh)


def test_rebuild_via_sdf_reports_topology_rebuild() -> None:
    rebuilt, report = rebuild_via_sdf(open_cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
    health = compute_mesh_health(rebuilt)

    assert report.input_boundary_edge_count > report.output_boundary_edge_count
    assert report.output_boundary_edge_count == 0
    assert health.is_closed
    assert report.output_vertex_count == rebuilt.vertex_count
    assert report.output_face_count == rebuilt.face_count


def test_engine_exposes_sdf_rebuild() -> None:
    sdk = GeometrySDK()
    rebuilt, report = sdk.rebuild_via_sdf(open_cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)

    assert sdk.health(rebuilt).is_closed
    assert report.voxel_size_mm == 0.5


def test_voxel_rebuild_is_rust_owned(monkeypatch) -> None:
    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel rebuild_via_sdf is required"):
        rebuild_via_sdf(open_cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
