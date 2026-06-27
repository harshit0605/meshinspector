from __future__ import annotations

import math
import numpy as np
import os
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.accelerators import rust
from geometry_sdk.core.mesh import signed_volume
from geometry_sdk.repair.basic import (
    basic_repair,
    crease_edge_diagnostics,
    crease_repair_plan_diagnostics,
    duplicate_nonmanifold_vertices,
    duplicate_multi_hole_vertices,
    fix_mesh_creases,
    find_disoriented_faces,
    flip_normals,
    mesh_healer_diagnostics,
    not_smooth_face_diagnostics,
    orient_faces_outward,
    prune_small_components,
    repair_multiple_edges,
    repair_nonmanifold_edges,
    remove_degenerate_faces,
    degenerate_face_diagnostics,
    multiple_edge_diagnostics,
    short_edge_diagnostics,
    unite_close_vertices,
)
from geometry_sdk.repair.holes import (
    fill_planar_holes,
    hole_complicating_faces_diagnostics,
    hole_fill_plan_diagnostics,
    ordered_boundary_loops,
    repeated_hole_boundary_vertices_diagnostics,
    remove_hole_complicating_faces,
    service_fill_holes,
)
from geometry_sdk.repair.self_intersections import fix_self_intersections_relax
from geometry_sdk.repair.voxel import rebuild_via_sdf
from geometry_sdk.analysis.health import compute_mesh_health, service_mesh_health
from geometry_sdk.testing.fixtures import crossing_triangles, cube, meshlib_self_intersecting_torus, open_cube, ring
from geometry_sdk.types import MeshDocument


def connected_crossing_triangles() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, -0.5, -1.0],
                [0.0, -0.5, 1.0],
                [0.0, 1.2, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [3, 4, 5], [0, 2, 3], [2, 3, 5]], dtype=np.int64),
    )


def meshlib_fix_self_intersections_relax_reference(mesh: MeshDocument) -> tuple[np.ndarray, int, int]:
    from meshlib import mrmeshnumpy as mn
    from meshlib import mrmeshpy as mr

    meshlib_mesh = mn.meshFromFacesVerts(
        mesh.faces.astype(np.int32),
        mesh.vertices.astype(np.float32),
    )
    settings = mr.FixSelfIntersectionSettings()
    settings.method = mr.FixSelfIntersectionMethod.Relax
    settings.relaxIterations = 1
    settings.maxExpand = 3
    settings.touchIsIntersection = True
    settings.subdivideEdgeLen = float("inf")
    input_self_intersections = mr.SelfIntersections.getFaces(meshlib_mesh, True).count()
    mr.SelfIntersections.fix(meshlib_mesh, settings)
    output_self_intersections = mr.SelfIntersections.getFaces(meshlib_mesh, True).count()
    return (
        np.asarray(mn.getNumpyVerts(meshlib_mesh), dtype=np.float64),
        int(input_self_intersections),
        int(output_self_intersections),
    )


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


def sliver_open_box() -> MeshDocument:
    length = 1.0e12
    width = 1.0e-8
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [length, 0.0, 0.0],
            [length, width, 0.0],
            [0.0, width, 0.0],
            [0.0, 0.0, 1.0],
            [length, 0.0, 1.0],
            [length, width, 1.0],
            [0.0, width, 1.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 2, 1],
            [0, 3, 2],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def metric_choice_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [2.919047, 0.461774, 0.673408],
            [0.324723, 2.717115, -1.489469],
            [0.468535, 2.068132, -1.758068],
            [0.9, 0.9, 2.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def edge_length_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [3.424361, 0.515909, -0.317714],
            [1.257992, 2.191716, -0.380263],
            [0.567597, 1.422257, -0.093612],
            [0.9, 0.9, 2.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def universal_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [1.452682, 0.165345, -1.239369],
            [4.198573, 1.677396, 0.146909],
            [4.623103, 0.400114, 1.902811],
            [0.207095, 3.185662, 0.391847],
            [-1.122086, 3.021953, -4.709435],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def max_dihedral_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [1.981194, -0.367531, -0.728251],
            [0.532394, 2.162295, -2.433038],
            [-1.572133, 2.289641, -1.347618],
            [-2.881760, -1.364960, -0.166524],
            [0.960472, -1.567687, 0.778782],
            [0.0, 0.0, 2.5],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 5],
            [1, 2, 5],
            [2, 3, 5],
            [3, 4, 5],
            [4, 0, 5],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def parallel_plane_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [1.609692, -0.087843, 2.770195],
            [0.203712, 2.505018, 3.249312],
            [-1.320781, 1.566465, 4.577460],
            [-1.064627, -0.429701, -4.678661],
            [1.896156, -3.739819, -4.482766],
            [0.0, 0.0, 7.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 5],
            [1, 2, 5],
            [2, 3, 5],
            [3, 4, 5],
            [4, 0, 5],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def complex_fill_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [1.830766, 0.505110, -2.595065],
            [-0.265999, 0.890404, -0.237632],
            [-3.207773, -0.167293, -3.545285],
            [0.317084, -2.684187, 3.511188],
            [0.0, 0.0, 4.5],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def min_tri_angle_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [2.055912, 0.314945, 0.665642],
            [0.827621, 0.340184, -0.500982],
            [-0.451904, 3.298288, 0.762371],
            [0.9, 0.9, 2.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def plane_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [2.737994, -1.424968, -1.349824],
            [1.281238, 3.077649, 1.060197],
            [1.176539, 0.804286, -0.468469],
            [0.9, 0.9, 2.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def plane_normalized_metric_open_pyramid() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [4.905726, -0.087403, 0.875388],
            [3.806988, 1.467058, -1.687848],
            [0.205486, 1.048638, -0.471093],
            [0.9, 0.9, 2.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 4],
            [1, 2, 4],
            [2, 3, 4],
            [3, 0, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def mesh_with_close_boundary_seam() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.001, 0.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [4, 2, 3]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def closed_tetra_with_close_vertices() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [0.001, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def cube_with_tiny_component() -> MeshDocument:
    base = open_cube(size=2.0)
    vertices = np.vstack(
        [
            base.vertices,
            np.array(
                [
                    [4.0, 0.0, 0.0],
                    [4.1, 0.0, 0.0],
                    [4.0, 0.1, 0.0],
                ],
                dtype=np.float64,
            ),
        ]
    )
    faces = np.vstack([base.faces, np.array([[8, 9, 10]], dtype=np.int64)])
    return MeshDocument(vertices, faces)


def mesh_with_short_edge() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [0.05, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 3], [1, 2, 3]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def mesh_with_degenerate_aspect_ratio() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, math.sqrt(3.0) / 2.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 0.1, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [0, 3, 4]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def mesh_with_multiple_edge_pair() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.5, -0.5, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [1, 0, 3], [0, 1, 4]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def mesh_with_nonmanifold_edge_fan() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.5, 0.0, 1.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [1, 0, 3], [0, 1, 4]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def mesh_with_two_closed_fans_sharing_vertex() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [0, 4, 5],
            [0, 5, 6],
            [0, 6, 4],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def mesh_with_closed_fans_reusing_neighbor() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [0, 1, 4],
            [0, 4, 5],
            [0, 5, 1],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def mesh_with_three_closed_fans_sharing_vertex() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [2.0, -1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [0, 4, 5],
            [0, 5, 6],
            [0, 6, 4],
            [0, 7, 8],
            [0, 8, 9],
            [0, 9, 7],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def mesh_with_multi_hole_vertex() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [0, 3, 4]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def mesh_with_repeated_hole_boundary_vertex() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 3], [1, 2, 4], [1, 3, 5]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def mesh_with_hole_complicating_face() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 4], [1, 2, 5], [1, 3, 4]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def closed_cube_with_flipped_top_triangle() -> MeshDocument:
    vertices = np.array(
        [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
        dtype=np.float64,
    )
    faces = np.array(
        [
            [0, 3, 2],
            [0, 2, 1],
            [4, 6, 5],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
        dtype=np.int64,
    )
    return MeshDocument(vertices, faces)


def inverted_planar_crease_patch() -> MeshDocument:
    vertices = np.array(
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        dtype=np.float64,
    )
    faces = np.array([[0, 1, 2], [1, 0, 3]], dtype=np.int64)
    return MeshDocument(vertices, faces)


def cube_with_tiny_tetra_crease_component() -> MeshDocument:
    base = cube(size=2.0)
    offset = len(base.vertices)
    tiny_vertices = np.array(
        [
            [4.0, 0.0, 0.0],
            [4.1, 0.0, 0.0],
            [4.05, 0.08660254037844387, 0.0],
            [4.05, 0.02886751345948129, 0.08164965809277261],
        ],
        dtype=np.float64,
    )
    tiny_faces = np.array(
        [
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
        ],
        dtype=np.int64,
    )
    return MeshDocument(
        np.vstack([base.vertices, tiny_vertices]),
        np.vstack([base.faces, tiny_faces + offset]),
    )


def cube_with_short_crease_branch() -> MeshDocument:
    base = cube(size=2.0)
    branch_vertices = np.array(
        [
            [-1.2, -1.0, -1.0],
            [-1.1, -0.9, -1.0],
            [-1.1, -1.0, -0.9],
        ],
        dtype=np.float64,
    )
    branch_faces = np.array(
        [
            [0, 8, 9],
            [8, 0, 10],
        ],
        dtype=np.int64,
    )
    return MeshDocument(
        np.vstack([base.vertices, branch_vertices]),
        np.vstack([base.faces, branch_faces]),
    )


def meshlib_triangle_aspect_ratio(a: np.ndarray, b: np.ndarray, c: np.ndarray) -> float:
    bc = float(np.linalg.norm(c - b))
    ca = float(np.linalg.norm(a - c))
    ab = float(np.linalg.norm(b - a))
    half_perimeter = (bc + ca + ab) / 2.0
    denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab)
    if denominator <= 0.0:
        return float(np.finfo(np.float64).max)
    return bc * ca * ab / denominator


def test_basic_repair_merges_duplicates_and_removes_degenerate_faces() -> None:
    repaired, report = basic_repair(damaged_mesh(), merge_tolerance=1e-8)

    assert repaired.vertex_count == 3
    assert repaired.face_count == 1
    assert report.merged_vertices == 1
    assert report.removed_degenerate_faces == 2
    assert report.removed_unreferenced_vertices == 1


def test_unite_close_vertices_matches_meshlib_boundary_default() -> None:
    repaired, changed = unite_close_vertices(mesh_with_close_boundary_seam(), close_dist=0.01)

    assert changed == 1
    assert repaired.vertex_count == 4
    assert repaired.faces.tolist() == [[0, 1, 2], [0, 2, 3]]

    sdk = GeometrySDK()
    sdk_repaired, sdk_changed = sdk.unite_close_vertices(mesh_with_close_boundary_seam(), close_dist=0.01)
    assert sdk_changed == 1
    assert sdk_repaired.faces.tolist() == [[0, 1, 2], [0, 2, 3]]


def test_unite_close_vertices_boundary_mode_preserves_closed_vertices_like_meshlib() -> None:
    mesh = closed_tetra_with_close_vertices()
    repaired, changed = unite_close_vertices(mesh, close_dist=0.01, unite_only_boundary=True)

    assert changed == 0
    assert repaired.vertex_count == mesh.vertex_count
    assert repaired.faces.tolist() == mesh.faces.tolist()


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


def test_mesh_healer_diagnostics_reports_official_repair_buckets() -> None:
    report = mesh_healer_diagnostics(open_cube(size=2.0))
    issues = {issue.issue_id: issue for issue in report.issues}

    assert report.input_vertex_count == 8
    assert report.input_face_count == 10
    assert report.holes_count == 1
    assert issues["holes"].count == 1
    assert issues["holes"].rust_repair_available is True
    assert issues["holes"].repair_command == "service_fill_holes"
    assert report.auto_repair_ready is True

    damaged_report = mesh_healer_diagnostics(damaged_mesh(), detect_self_intersections=False)
    damaged_issues = {issue.issue_id: issue for issue in damaged_report.issues}
    assert damaged_issues["duplicate_vertices"].count == 1
    assert damaged_issues["duplicate_vertices"].repair_command == "unite_close_vertices"
    assert damaged_issues["degenerate_faces"].count == 2
    assert damaged_issues["unreferenced_vertices"].count == 1
    assert damaged_report.fixable_issue_count >= 4


def test_prune_small_components_matches_meshlib_area_component_contract() -> None:
    repaired, report = prune_small_components(cube_with_tiny_component(), min_area_mm2=0.5)

    assert report.input_component_count == 2
    assert report.output_component_count == 1
    assert report.removed_component_count == 1
    assert report.removed_face_count == 1
    assert report.removed_vertex_count == 3
    assert report.retained_face_count == repaired.face_count
    assert repaired.vertex_count == 8
    assert repaired.face_count == 10
    assert repaired.faces.max() == 7

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.prune_small_components(cube_with_tiny_component(), min_area_mm2=0.5)
    assert sdk_report.removed_component_count == 1
    assert sdk_repaired.vertex_count == 8


def test_prune_small_components_keeps_all_components_below_disabled_threshold() -> None:
    repaired, report = prune_small_components(cube_with_tiny_component(), min_area_mm2=0.0)

    assert report.removed_component_count == 0
    assert report.output_component_count == 2
    assert repaired.vertex_count == cube_with_tiny_component().vertex_count
    assert repaired.face_count == cube_with_tiny_component().face_count


def test_short_edge_diagnostics_matches_meshlib_critical_length_contract() -> None:
    report = short_edge_diagnostics(mesh_with_short_edge(), critical_length_mm=0.05)

    assert report.critical_length_mm == 0.05
    assert report.edge_count == 5
    assert report.short_edge_count == 1
    assert report.min_short_edge_length_mm == pytest.approx(0.05)
    assert report.max_short_edge_length_mm == pytest.approx(0.05)
    assert report.edges[0].edge == (0, 1)
    assert report.edges[0].length_mm == pytest.approx(0.05)

    sdk = GeometrySDK()
    assert sdk.short_edge_diagnostics(mesh_with_short_edge(), critical_length_mm=-0.05).short_edge_count == 1


def test_degenerate_face_diagnostics_matches_meshlib_aspect_ratio_contract() -> None:
    mesh = mesh_with_degenerate_aspect_ratio()
    skinny_ratio = meshlib_triangle_aspect_ratio(mesh.vertices[0], mesh.vertices[3], mesh.vertices[4])

    report = degenerate_face_diagnostics(mesh, critical_aspect_ratio=skinny_ratio)

    assert report.critical_aspect_ratio == pytest.approx(skinny_ratio)
    assert report.face_count == 2
    assert report.degenerate_face_count == 1
    assert report.min_degenerate_aspect_ratio == pytest.approx(skinny_ratio)
    assert report.max_degenerate_aspect_ratio == pytest.approx(skinny_ratio)
    assert report.faces[0].face_index == 1
    assert report.faces[0].face == (0, 3, 4)
    assert report.faces[0].aspect_ratio == pytest.approx(skinny_ratio)

    sdk = GeometrySDK()
    assert sdk.degenerate_face_diagnostics(mesh, critical_aspect_ratio=skinny_ratio).degenerate_face_count == 1


def test_multiple_edge_diagnostics_matches_meshlib_vertex_pair_contract() -> None:
    report = multiple_edge_diagnostics(mesh_with_multiple_edge_pair())

    assert report.edge_count == 7
    assert report.multiple_edge_count == 1
    assert report.edges[0].vertex_pair == (0, 1)
    assert report.edges[0].topology_edge_count == 2
    assert report.edges[0].face_edge_occurrences == 3
    assert report.edges[0].forward_occurrences == 2
    assert report.edges[0].reverse_occurrences == 1

    sdk = GeometrySDK()
    assert sdk.multiple_edge_diagnostics(mesh_with_multiple_edge_pair()).multiple_edge_count == 1


def test_repair_multiple_edges_splits_duplicate_topology_edges_like_meshlib() -> None:
    repaired, report = repair_multiple_edges(mesh_with_multiple_edge_pair())

    assert report.input_multiple_edge_count == 1
    assert report.output_multiple_edge_count == 0
    assert report.split_edge_count == 1
    assert report.split_face_count == 1
    assert report.added_vertex_count == 1
    assert report.input_face_count == 3
    assert report.output_face_count == 4
    assert repaired.vertex_count == 6
    assert repaired.face_count == 4
    assert repaired.vertices[-1].tolist() == pytest.approx([0.5, 0.0, 0.0])
    assert multiple_edge_diagnostics(repaired).multiple_edge_count == 0

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.repair_multiple_edges(mesh_with_multiple_edge_pair())
    assert sdk_report.output_multiple_edge_count == 0
    assert sdk_repaired.vertex_count == 6


def test_mesh_healer_diagnostics_reports_multiple_edges_as_rust_repairable() -> None:
    report = mesh_healer_diagnostics(mesh_with_multiple_edge_pair(), detect_self_intersections=False)
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues["multiple_edges"].count == 1
    assert issues["multiple_edges"].rust_repair_available is True
    assert issues["multiple_edges"].repair_command == "repair_multiple_edges"


def test_repair_nonmanifold_edges_removes_excess_edge_faces_like_meshlib_builder() -> None:
    repaired, report = repair_nonmanifold_edges(mesh_with_nonmanifold_edge_fan())
    health = compute_mesh_health(repaired)

    assert report.input_nonmanifold_edge_count == 1
    assert report.output_nonmanifold_edge_count == 0
    assert report.removed_face_count == 1
    assert report.input_face_count == 3
    assert report.output_face_count == 2
    assert report.input_vertex_count == 5
    assert report.output_vertex_count == 5
    assert repaired.face_count == 2
    assert health.nonmanifold_edge_count == 0

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.repair_nonmanifold_edges(mesh_with_nonmanifold_edge_fan())
    assert sdk_report.output_nonmanifold_edge_count == 0
    assert sdk_repaired.face_count == 2


def test_mesh_healer_diagnostics_reports_nonmanifold_edges_as_rust_repairable() -> None:
    report = mesh_healer_diagnostics(mesh_with_nonmanifold_edge_fan(), detect_self_intersections=False)
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues["nonmanifold_edges"].count == 1
    assert issues["nonmanifold_edges"].rust_repair_available is True
    assert issues["nonmanifold_edges"].repair_command == "repair_nonmanifold_edges"


def test_duplicate_nonmanifold_vertices_splits_disconnected_closed_fans_like_meshlib_builder() -> None:
    repaired, report = duplicate_nonmanifold_vertices(mesh_with_two_closed_fans_sharing_vertex())

    assert report.input_nonmanifold_vertex_count == 1
    assert report.output_nonmanifold_vertex_count == 0
    assert report.duplicated_vertex_count == 1
    assert report.input_vertex_count == 7
    assert report.output_vertex_count == 8
    assert report.input_face_count == 6
    assert report.output_face_count == 6
    assert repaired.vertices[-1].tolist() == pytest.approx([0.0, 0.0, 0.0])
    assert repaired.faces[:3].tolist() == [[0, 1, 2], [0, 2, 3], [0, 3, 1]]
    assert repaired.faces[3:].tolist() == [[7, 4, 5], [7, 5, 6], [7, 6, 4]]

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.duplicate_nonmanifold_vertices(mesh_with_two_closed_fans_sharing_vertex())
    assert sdk_report.output_nonmanifold_vertex_count == 0
    assert sdk_repaired.vertex_count == 8


def test_duplicate_nonmanifold_vertices_splits_repeated_neighbor_path_like_meshlib_builder() -> None:
    repaired, report = duplicate_nonmanifold_vertices(mesh_with_closed_fans_reusing_neighbor())

    assert report.input_nonmanifold_vertex_count == 2
    assert report.output_nonmanifold_vertex_count == 0
    assert report.duplicated_vertex_count == 2
    assert report.input_vertex_count == 6
    assert report.output_vertex_count == 8
    assert repaired.vertices[-2].tolist() == pytest.approx([0.0, 0.0, 0.0])
    assert repaired.vertices[-1].tolist() == pytest.approx([1.0, 0.0, 0.0])
    assert repaired.faces[:3].tolist() == [[0, 1, 2], [0, 2, 3], [0, 3, 1]]
    assert repaired.faces[3:].tolist() == [[6, 7, 4], [6, 4, 5], [6, 5, 7]]


def test_duplicate_nonmanifold_vertices_respects_meshlib_face_region_scope() -> None:
    repaired, report = duplicate_nonmanifold_vertices(
        mesh_with_three_closed_fans_sharing_vertex(),
        region_face_indices=[3, 4, 5, 6, 7, 8],
    )

    assert report.input_nonmanifold_vertex_count == 1
    assert report.output_nonmanifold_vertex_count == 0
    assert report.duplicated_vertex_count == 1
    assert report.input_vertex_count == 10
    assert report.output_vertex_count == 11
    assert repaired.vertices[-1].tolist() == pytest.approx([0.0, 0.0, 0.0])
    assert repaired.faces.tolist() == [
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 1],
        [0, 4, 5],
        [0, 5, 6],
        [0, 6, 4],
        [10, 7, 8],
        [10, 8, 9],
        [10, 9, 7],
    ]

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.duplicate_nonmanifold_vertices(
        mesh_with_three_closed_fans_sharing_vertex(),
        region_face_indices=[3, 4, 5, 6, 7, 8],
    )
    assert sdk_report.duplicated_vertex_count == 1
    assert sdk_repaired.faces[6:].tolist() == [[10, 7, 8], [10, 8, 9], [10, 9, 7]]


def test_mesh_healer_diagnostics_reports_nonmanifold_vertices_as_rust_repairable() -> None:
    report = mesh_healer_diagnostics(mesh_with_two_closed_fans_sharing_vertex(), detect_self_intersections=False)
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues.get("nonmanifold_vertices") is not None
    assert issues["nonmanifold_vertices"].count == 1
    assert issues["nonmanifold_vertices"].rust_repair_available is True
    assert issues["nonmanifold_vertices"].repair_command == "duplicate_nonmanifold_vertices"


def test_duplicate_multi_hole_vertices_splits_disconnected_boundary_fans_like_meshlib() -> None:
    repaired, report = duplicate_multi_hole_vertices(mesh_with_multi_hole_vertex())

    assert report.input_multi_hole_vertex_count == 1
    assert report.output_multi_hole_vertex_count == 0
    assert report.duplicated_vertex_count == 1
    assert report.input_vertex_count == 5
    assert report.output_vertex_count == 6
    assert report.input_face_count == 2
    assert report.output_face_count == 2
    assert repaired.vertices[-1].tolist() == pytest.approx([0.0, 0.0, 0.0])
    assert repaired.faces.tolist() == [[0, 1, 2], [5, 3, 4]]

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.duplicate_multi_hole_vertices(mesh_with_multi_hole_vertex())
    assert sdk_report.output_multi_hole_vertex_count == 0
    assert sdk_repaired.vertex_count == 6


def test_mesh_healer_diagnostics_reports_multi_hole_vertices_as_rust_repairable() -> None:
    report = mesh_healer_diagnostics(mesh_with_multi_hole_vertex(), detect_self_intersections=False)
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues["multi_hole_vertices"].count == 1
    assert issues["multi_hole_vertices"].rust_repair_available is True
    assert issues["multi_hole_vertices"].repair_command == "duplicate_multi_hole_vertices"


def test_not_smooth_face_diagnostics_matches_meshlib_neighbor_angle_contract() -> None:
    report = not_smooth_face_diagnostics(closed_cube_with_flipped_top_triangle(), min_angle_radians=0.3)

    assert report.min_angle_radians == pytest.approx(0.3)
    assert report.face_count == 12
    assert report.not_smooth_face_count == 2
    assert [face.face_index for face in report.faces] == [2, 3]
    assert [face.face for face in report.faces] == [(4, 6, 5), (4, 6, 7)]
    assert report.faces[0].angle_delta_radians == pytest.approx(math.pi / 2.0)
    assert report.faces[1].angle_delta_radians == pytest.approx(math.pi / 2.0)

    sdk = GeometrySDK()
    assert sdk.not_smooth_face_diagnostics(closed_cube_with_flipped_top_triangle()).not_smooth_face_count == 2


def test_find_disoriented_faces_matches_meshlib_ray_count_contract() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [1.0, 1.0, 1.0],
                [-1.0, -1.0, 1.0],
                [-1.0, 1.0, -1.0],
                [1.0, -1.0, -1.0],
            ],
            dtype=np.float64,
        ),
        np.asarray(
            [
                [0, 1, 2],
                [0, 1, 3],
                [0, 3, 2],
                [1, 2, 3],
            ],
            dtype=np.int64,
        ),
    )

    assert find_disoriented_faces(mesh) == [0]
    assert find_disoriented_faces(mesh, ray_mode="positive") == [0]
    assert find_disoriented_faces(mesh, ray_mode="both") == [0]
    assert GeometrySDK().find_disoriented_faces(mesh) == [0]


def test_flip_normals_matches_meshlib_orientation_flip_contract() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [1.0, 1.0, 1.0],
                [-1.0, -1.0, 1.0],
                [-1.0, 1.0, -1.0],
                [1.0, -1.0, -1.0],
            ],
            dtype=np.float64,
        ),
        np.asarray(
            [
                [0, 2, 1],
                [0, 1, 3],
                [0, 3, 2],
                [1, 2, 3],
            ],
            dtype=np.int64,
        ),
    )

    flipped = flip_normals(mesh)

    assert np.array_equal(flipped.vertices, mesh.vertices)
    assert np.array_equal(flipped.faces, mesh.faces[:, [0, 2, 1]])
    assert signed_volume(flipped) == pytest.approx(-signed_volume(mesh))
    assert np.array_equal(GeometrySDK().flip_normals(mesh).faces, flipped.faces)


def test_mesh_healer_diagnostics_reports_not_smooth_faces_as_rust_diagnostic_only() -> None:
    report = mesh_healer_diagnostics(closed_cube_with_flipped_top_triangle(), detect_self_intersections=False)
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues["not_smooth_faces"].count == 2
    assert issues["not_smooth_faces"].rust_repair_available is False
    assert issues["not_smooth_faces"].repair_command is None
    assert report.auto_repair_ready is False


def test_crease_edge_diagnostics_matches_meshlib_dihedral_cos_contract() -> None:
    report = crease_edge_diagnostics(cube(size=2.0), angle_from_planar_radians=0.3)

    assert report.angle_from_planar_radians == pytest.approx(0.3)
    assert report.min_component_length_mm is None
    assert report.edge_count == 18
    assert report.raw_crease_edge_count == 12
    assert report.crease_edge_count == 12
    assert (0, 2) not in {entry.edge for entry in report.edges}
    assert (0, 1) in {entry.edge for entry in report.edges}
    assert all(entry.dihedral_cosine == pytest.approx(0.0) for entry in report.edges)

    default_report = crease_edge_diagnostics(closed_cube_with_flipped_top_triangle())
    assert default_report.angle_from_planar_radians == pytest.approx(math.radians(175.0))
    assert default_report.crease_edge_count == 1
    assert default_report.edges[0].edge == (4, 6)
    assert default_report.edges[0].dihedral_cosine == pytest.approx(-1.0)

    sdk = GeometrySDK()
    assert sdk.crease_edge_diagnostics(cube(size=2.0), angle_from_planar_radians=0.3).crease_edge_count == 12


def test_crease_edge_diagnostics_filters_short_components_like_meshlib_filter_crease_edges() -> None:
    report = crease_edge_diagnostics(
        cube_with_tiny_tetra_crease_component(),
        angle_from_planar_radians=0.3,
        min_component_length_mm=1.0,
    )

    assert report.raw_crease_edge_count == 18
    assert report.crease_edge_count == 12
    assert all(edge.edge[0] < 8 and edge.edge[1] < 8 for edge in report.edges)

    sdk = GeometrySDK()
    assert (
        sdk.crease_edge_diagnostics(
            cube_with_tiny_tetra_crease_component(),
            angle_from_planar_radians=0.3,
            min_component_length_mm=1.0,
        ).raw_crease_edge_count
        == 18
    )


def test_crease_edge_diagnostics_filters_short_branches_like_meshlib_filter_crease_edges() -> None:
    unfiltered = crease_edge_diagnostics(cube_with_short_crease_branch(), angle_from_planar_radians=0.3)
    assert unfiltered.raw_crease_edge_count == 13
    assert (0, 8) in {edge.edge for edge in unfiltered.edges}

    report = crease_edge_diagnostics(
        cube_with_short_crease_branch(),
        angle_from_planar_radians=0.3,
        min_branch_length_mm=0.5,
    )

    assert report.min_branch_length_mm == pytest.approx(0.5)
    assert report.raw_crease_edge_count == 13
    assert report.crease_edge_count == 12
    assert (0, 8) not in {edge.edge for edge in report.edges}
    assert (0, 1) in {edge.edge for edge in report.edges}

    sdk = GeometrySDK()
    assert (
        sdk.crease_edge_diagnostics(
            cube_with_short_crease_branch(),
            angle_from_planar_radians=0.3,
            min_branch_length_mm=0.5,
        ).crease_edge_count
        == 12
    )


def test_crease_repair_plan_diagnostics_matches_meshlib_fix_mesh_creases_face_selection() -> None:
    report = crease_repair_plan_diagnostics(inverted_planar_crease_patch())

    assert report.angle_from_planar_radians == pytest.approx(math.radians(175.0))
    assert report.critical_tri_aspect_ratio == pytest.approx(1e3)
    assert report.crease_edge_count == 1
    assert report.planned_region_count == 1
    assert report.planned_face_count == 2
    assert report.regions[0].crease_edge == (0, 1)
    assert report.regions[0].selected_face_indices == [0, 1]

    sdk = GeometrySDK()
    sdk_report = sdk.crease_repair_plan_diagnostics(inverted_planar_crease_patch())
    assert sdk_report.planned_face_count == 2


def test_fix_mesh_creases_retriangulates_flipped_cube_patch_like_meshlib() -> None:
    repaired, report = fix_mesh_creases(closed_cube_with_flipped_top_triangle())

    assert report.input_crease_edge_count == 1
    assert report.output_crease_edge_count == 0
    assert report.repaired_region_count == 1
    assert report.removed_face_count == 1
    assert report.added_face_count == 1
    assert report.input_face_count == 12
    assert report.output_face_count == 12
    assert repaired.vertex_count == 8
    assert repaired.face_count == 12
    assert crease_edge_diagnostics(repaired).crease_edge_count == 0
    assert compute_mesh_health(repaired).is_closed is True

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.fix_mesh_creases(closed_cube_with_flipped_top_triangle())
    assert sdk_report.output_crease_edge_count == 0
    assert sdk_repaired.face_count == 12


def test_mesh_healer_diagnostics_reports_crease_edges_as_rust_repairable() -> None:
    report = mesh_healer_diagnostics(closed_cube_with_flipped_top_triangle(), detect_self_intersections=False)
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues["crease_edges"].count == 1
    assert issues["crease_edges"].rust_repair_available is True
    assert issues["crease_edges"].repair_command == "fix_mesh_creases"


def test_mesh_healer_diagnostics_reports_self_intersections_as_rebuild_repairable() -> None:
    report = mesh_healer_diagnostics(crossing_triangles())
    issues = {issue.issue_id: issue for issue in report.issues}

    assert issues["self_intersections"].count == 2
    assert issues["self_intersections"].rust_repair_available is True
    assert issues["self_intersections"].repair_command == "rebuild_via_sdf"
    assert report.auto_repair_ready is True

    sdk = GeometrySDK()
    assert sdk.mesh_healer_diagnostics(crossing_triangles()).self_intersections == 2


def test_fix_self_intersections_relax_exposes_meshlib_relax_subset() -> None:
    mesh = connected_crossing_triangles()
    expected_vertices, expected_input_intersections, expected_output_intersections = (
        meshlib_fix_self_intersections_relax_reference(mesh)
    )

    repaired, report = fix_self_intersections_relax(
        mesh,
        relax_iterations=1,
        max_expand=3,
        touch_is_intersection=True,
        force=0.5,
    )

    assert repaired.faces.tolist() == mesh.faces.tolist()
    assert report.input_self_intersections == expected_input_intersections == 2
    assert expected_output_intersections == 0
    assert report.output_self_intersections == expected_output_intersections
    assert report.relaxed_face_count == 4
    assert report.moved_vertex_count == 6
    assert report.method == "relax"
    assert report.subdivide_edge_len_disabled is True
    assert report.topology_changed is False
    np.testing.assert_allclose(repaired.vertices, expected_vertices, atol=1e-7)

    sdk_repaired, sdk_report = GeometrySDK().fix_self_intersections_relax(mesh, relax_iterations=1)
    np.testing.assert_allclose(sdk_repaired.vertices, repaired.vertices)
    assert sdk_report.input_self_intersections == report.input_self_intersections

    disconnected = crossing_triangles()
    disconnected_repaired, disconnected_report = fix_self_intersections_relax(
        disconnected,
        relax_iterations=1,
        max_expand=3,
        touch_is_intersection=True,
        force=0.5,
    )
    np.testing.assert_allclose(disconnected_repaired.vertices, disconnected.vertices)
    assert disconnected_report.input_self_intersections == 0
    assert disconnected_report.moved_vertex_count == 0


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


def test_hole_fill_plan_diagnostics_exposes_meshlib_representative_edge_contract() -> None:
    report = hole_fill_plan_diagnostics(open_cube(size=2.0))

    assert report.input_holes == 1
    assert report.planned_holes == 1
    assert report.skipped_holes == 0
    assert report.total_planned_triangles == 2
    assert report.total_boundary_edges == 4
    assert report.plans[0].hole_index == 0
    assert report.plans[0].representative_edge == (0, 3)
    assert report.plans[0].boundary_vertex_indices == [0, 3, 7, 4]
    assert report.plans[0].boundary_edge_count == 4
    assert report.plans[0].planned_triangles == 2
    assert report.plans[0].skipped is False
    assert report.plans[0].skip_reason is None

    sdk = GeometrySDK()
    sdk_report = sdk.hole_fill_plan_diagnostics(open_cube(size=2.0))
    assert sdk_report.total_planned_triangles == 2


def test_repeated_hole_boundary_vertices_diagnostics_matches_meshlib_hole_ring_contract() -> None:
    mesh = mesh_with_repeated_hole_boundary_vertex()
    loops = ordered_boundary_loops(mesh)
    report = repeated_hole_boundary_vertices_diagnostics(mesh)

    assert loops == [[0, 1, 2, 4, 1, 5, 3]]
    assert report.input_holes == 1
    assert report.repeated_vertex_count == 1
    assert report.vertices[0].vertex_index == 1
    assert report.vertices[0].hole_indices == [0]
    assert report.vertices[0].occurrences == 2

    open_report = repeated_hole_boundary_vertices_diagnostics(open_cube(size=2.0))
    assert open_report.input_holes == 1
    assert open_report.repeated_vertex_count == 0
    assert open_report.vertices == []

    sdk = GeometrySDK()
    sdk_report = sdk.repeated_hole_boundary_vertices_diagnostics(mesh)
    assert sdk_report.vertices[0].vertex_index == 1


def test_mesh_healer_diagnostics_reports_repeated_hole_boundary_vertices_as_diagnostic_only() -> None:
    report = mesh_healer_diagnostics(
        mesh_with_repeated_hole_boundary_vertex(),
        detect_self_intersections=False,
    )

    issues = {issue.issue_id: issue for issue in report.issues}
    assert issues["repeated_hole_boundary_vertices"].count == 1
    assert issues["repeated_hole_boundary_vertices"].rust_repair_available is False
    assert issues["repeated_hole_boundary_vertices"].repair_command is None


def test_hole_complicating_faces_diagnostics_reports_smaller_wedge_faces_like_meshlib() -> None:
    mesh = mesh_with_hole_complicating_face()
    loops = ordered_boundary_loops(mesh)
    report = hole_complicating_faces_diagnostics(mesh)

    assert loops == [[0, 1, 2, 5, 1, 3, 4]]
    assert report.input_repeated_vertex_count == 1
    assert report.complicating_face_count == 1
    assert report.faces[0].repeated_vertex_index == 1
    assert report.faces[0].face_index == 1

    sdk = GeometrySDK()
    sdk_report = sdk.hole_complicating_faces_diagnostics(mesh)
    assert sdk_report.faces[0].face_index == 1


def test_remove_hole_complicating_faces_deletes_meshlib_reported_faces() -> None:
    mesh = mesh_with_hole_complicating_face()
    repaired, report = remove_hole_complicating_faces(mesh)

    assert report.input_face_count == 3
    assert report.output_face_count == 2
    assert report.removed_face_count == 1
    assert report.input_repeated_vertex_count == 1
    assert report.output_repeated_vertex_count == 0
    assert repaired.faces.tolist() == [[0, 1, 4], [1, 3, 4]]
    assert repeated_hole_boundary_vertices_diagnostics(repaired).repeated_vertex_count == 0

    sdk = GeometrySDK()
    sdk_repaired, sdk_report = sdk.remove_hole_complicating_faces(mesh)
    assert sdk_report.removed_face_count == 1
    assert sdk_repaired.faces.tolist() == [[0, 1, 4], [1, 3, 4]]


def test_mesh_healer_diagnostics_reports_hole_complicating_faces_as_rust_repairable() -> None:
    report = mesh_healer_diagnostics(
        mesh_with_hole_complicating_face(),
        detect_self_intersections=False,
    )

    issues = {issue.issue_id: issue for issue in report.issues}
    assert issues["hole_complicating_faces"].count == 1
    assert issues["hole_complicating_faces"].rust_repair_available is True
    assert issues["hole_complicating_faces"].repair_command == "remove_hole_complicating_faces"


def test_hole_fill_plan_diagnostics_marks_oversized_holes_skipped() -> None:
    report = hole_fill_plan_diagnostics(open_cube(size=2.0), max_edges=3)

    assert report.input_holes == 1
    assert report.planned_holes == 0
    assert report.skipped_holes == 1
    assert report.total_planned_triangles == 0
    assert report.plans[0].skipped is True
    assert report.plans[0].skip_reason == "max_edges_exceeded"


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


def test_service_fill_holes_exposes_meshlib_max_polygon_subdivisions() -> None:
    repaired, report = service_fill_holes(open_cube(size=2.0), max_polygon_subdivisions=2)

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert compute_mesh_health(repaired).is_closed

    with pytest.raises(ValueError, match="max_polygon_subdivisions must be at least 2"):
        service_fill_holes(open_cube(size=2.0), max_polygon_subdivisions=1)


def test_service_fill_holes_exposes_meshlib_multiple_edges_resolve_mode() -> None:
    repaired, report = service_fill_holes(open_cube(size=2.0), multiple_edges_resolve_mode="simple")

    assert report.filled_holes == 1
    assert compute_mesh_health(repaired).is_closed

    strong_repaired, strong_report = service_fill_holes(open_cube(size=2.0), multiple_edges_resolve_mode="strong")
    assert strong_report.filled_holes == 1
    assert compute_mesh_health(strong_repaired).is_closed

    with pytest.raises(ValueError, match="multiple_edges_resolve_mode"):
        service_fill_holes(open_cube(size=2.0), multiple_edges_resolve_mode="stronger")


def test_service_fill_holes_exposes_meshlib_make_degenerate_band() -> None:
    repaired, report = service_fill_holes(open_cube(size=2.0), make_degenerate_band=True)

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_vertices == 4
    assert report.added_faces == 10
    assert repaired.vertex_count == open_cube(size=2.0).vertex_count + 4


def test_service_fill_holes_exposes_meshlib_stop_before_bad_triangulation() -> None:
    mesh = sliver_open_box()
    repaired, report = service_fill_holes(mesh, stop_before_bad_triangulation=True)

    assert report.input_holes == 1
    assert report.filled_holes == 0
    assert report.skipped_holes == 1
    assert report.added_vertices == 0
    assert report.added_faces == 0
    assert repaired.vertex_count == mesh.vertex_count
    assert repaired.face_count == mesh.face_count


def test_service_fill_holes_exposes_meshlib_min_area_metric() -> None:
    mesh = metric_choice_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="min_area")

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    assert report.new_face_indices == [mesh.face_count, mesh.face_count + 1]
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3)}


def test_service_fill_holes_exposes_meshlib_edge_length_metric() -> None:
    mesh = edge_length_metric_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="edge_length")

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 2), (0, 2, 3)}


def test_service_fill_holes_exposes_meshlib_smooth_bd_param() -> None:
    mesh = edge_length_metric_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="edge_length", smooth_bd=False)

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    assert compute_mesh_health(repaired).is_closed


def test_service_fill_holes_exposes_meshlib_universal_metric() -> None:
    mesh = universal_metric_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="universal")

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3)}


def test_service_fill_holes_exposes_meshlib_max_dihedral_angle_metric() -> None:
    mesh = max_dihedral_metric_open_pyramid()
    repaired, report = service_fill_holes(
        mesh,
        fill_metric="max_dihedral_angle",
        smooth_bd=False,
    )

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 3
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-3:]}
    assert added_faces == {(0, 1, 2), (0, 2, 3), (0, 3, 4)}


def test_service_fill_holes_exposes_meshlib_parallel_plane_metric() -> None:
    mesh = parallel_plane_metric_open_pyramid()
    repaired, report = service_fill_holes(
        mesh,
        fill_metric="parallel_plane",
        smooth_bd=False,
    )

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 3
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-3:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3), (0, 3, 4)}


def test_service_fill_holes_exposes_meshlib_complex_fill_metric() -> None:
    mesh = complex_fill_metric_open_pyramid()
    repaired, report = service_fill_holes(
        mesh,
        fill_metric="complex_fill",
        smooth_bd=False,
    )

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3)}


def test_service_fill_holes_exposes_meshlib_min_tri_angle_metric() -> None:
    mesh = min_tri_angle_metric_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="min_tri_angle")

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3)}


def test_service_fill_holes_exposes_meshlib_plane_metric() -> None:
    mesh = plane_metric_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="plane")

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3)}


def test_service_fill_holes_exposes_meshlib_plane_normalized_metric() -> None:
    mesh = plane_normalized_metric_open_pyramid()
    repaired, report = service_fill_holes(mesh, fill_metric="plane_normalized")

    assert report.input_holes == 1
    assert report.filled_holes == 1
    assert report.added_faces == 2
    added_faces = {tuple(sorted(face.tolist())) for face in repaired.faces[-2:]}
    assert added_faces == {(0, 1, 3), (1, 2, 3)}


def test_service_fill_holes_accepts_meshlib_stitch_metric_modes() -> None:
    metric_aliases = [
        "complex_stitch",
        "edge_length_stitch",
        "vertical_stitch",
        "vertical_stitch_edge_based",
    ]

    for fill_metric in metric_aliases:
        repaired, report = service_fill_holes(open_cube(size=2.0), fill_metric=fill_metric)

        assert report.input_holes == 1
        assert report.filled_holes == 1
        assert report.added_faces == 2
        assert compute_mesh_health(repaired).is_closed


def test_service_fill_holes_exposes_meshlib_vertical_stitch_up_dir_param() -> None:
    for fill_metric in ["vertical_stitch", "vertical_stitch_edge_based"]:
        repaired, report = service_fill_holes(
            open_cube(size=2.0),
            fill_metric=fill_metric,
            fill_metric_up_dir=(1.0, 0.0, 0.0),
        )

        assert report.input_holes == 1
        assert report.filled_holes == 1
        assert compute_mesh_health(repaired).is_closed

    with pytest.raises(ValueError, match="fill_metric_up_dir"):
        service_fill_holes(
            open_cube(size=2.0),
            fill_metric="vertical_stitch",
            fill_metric_up_dir=(0.0, 0.0, 0.0),
        )


def test_engine_exposes_planar_hole_fill() -> None:
    sdk = GeometrySDK()
    repaired, report = sdk.fill_planar_holes(open_cube(size=2.0))

    assert report.filled_holes == 1
    assert sdk.health(repaired).is_closed

    service_repaired, service_report = sdk.service_fill_holes(
        open_cube(size=2.0),
        max_polygon_subdivisions=2,
    )
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


def test_rebuild_via_sdf_repairs_meshlib_self_intersecting_torus() -> None:
    source = meshlib_self_intersecting_torus()

    rebuilt, report = rebuild_via_sdf(source, voxel_size_mm=0.1, padding_mm=0.2, refine=False)
    health = compute_mesh_health(rebuilt)

    assert report.input_self_intersections == 256
    assert report.output_self_intersections == 0
    assert health.self_intersections == 0
    assert health.nonmanifold_edge_count == 0
    assert rebuilt.face_count > 0


def test_tunnel_diagnostics_detects_torus_handle_like_meshlib_tunnel_detector() -> None:
    tunnel_diagnostics = getattr(rust, "tunnel_diagnostics", None)
    assert tunnel_diagnostics is not None, (
        "Rust accelerator must expose MeshLib MRTunnelDetector-style tunnel diagnostics"
    )

    torus_report = tunnel_diagnostics(ring(radial_segments=24, tube_segments=8))
    cube_report = tunnel_diagnostics(cube(size=2.0))

    assert torus_report.tunnel_count == 1
    assert torus_report.genus == 1
    assert torus_report.closed is True
    assert torus_report.nonmanifold_edge_count == 0
    assert cube_report.tunnel_count == 0
    assert cube_report.genus == 0
    assert cube_report.closed is True

    sdk_report = GeometrySDK().tunnel_diagnostics(ring(radial_segments=24, tube_segments=8))
    assert sdk_report == torus_report


def test_tunnel_face_band_matches_meshlib_detect_tunnel_faces_on_torus() -> None:
    detect_tunnel_faces = getattr(rust, "detect_tunnel_faces", None)
    assert detect_tunnel_faces is not None, (
        "Rust accelerator must expose MeshLib detectTunnelFaces-style face-band selection"
    )

    selected_faces = detect_tunnel_faces(ring(radial_segments=24, tube_segments=8))
    sdk_selected_faces = GeometrySDK().detect_tunnel_faces(ring(radial_segments=24, tube_segments=8))

    assert selected_faces == [
        10,
        11,
        26,
        27,
        42,
        43,
        58,
        59,
        74,
        75,
        90,
        91,
        106,
        107,
        122,
        123,
        138,
        139,
        154,
        155,
        170,
        171,
        186,
        187,
        202,
        203,
        218,
        219,
        234,
        235,
        250,
        251,
        266,
        267,
        282,
        283,
        298,
        299,
        314,
        315,
        330,
        331,
        346,
        347,
        362,
        363,
        378,
        379,
    ]
    assert sdk_selected_faces == selected_faces


def test_tunnel_face_band_matches_meshlib_detect_tunnel_faces_on_24x12_torus() -> None:
    detect_tunnel_faces = getattr(rust, "detect_tunnel_faces", None)
    assert detect_tunnel_faces is not None, (
        "Rust accelerator must expose MeshLib detectTunnelFaces-style face-band selection"
    )

    selected_faces = detect_tunnel_faces(ring(radial_segments=24, tube_segments=12))
    expected_faces = [
        face_index
        for radial_index in range(24)
        for face_index in (2 * (radial_index * 12 + 2), 2 * (radial_index * 12 + 2) + 1)
    ]

    assert selected_faces == expected_faces
    assert GeometrySDK().detect_tunnel_faces(ring(radial_segments=24, tube_segments=12)) == expected_faces


def test_tunnel_face_band_matches_meshlib_detect_tunnel_faces_on_24x10_torus() -> None:
    detect_tunnel_faces = getattr(rust, "detect_tunnel_faces", None)
    assert detect_tunnel_faces is not None, (
        "Rust accelerator must expose MeshLib detectTunnelFaces-style face-band selection"
    )

    selected_faces = detect_tunnel_faces(ring(radial_segments=24, tube_segments=10))
    expected_faces = [
        face_index
        for radial_index in range(24)
        for face_index in (2 * (radial_index * 10 + 1), 2 * (radial_index * 10 + 1) + 1)
    ]

    assert selected_faces == expected_faces
    assert GeometrySDK().detect_tunnel_faces(ring(radial_segments=24, tube_segments=10)) == expected_faces


def test_eliminate_tunnels_matches_meshlib_delete_and_fill_counts_on_torus() -> None:
    eliminate_tunnels = getattr(rust, "eliminate_tunnels", None)
    assert eliminate_tunnels is not None, (
        "Rust accelerator must expose MeshLib eliminateTunnels-style delete-and-fill repair"
    )

    repaired, report = eliminate_tunnels(ring(radial_segments=24, tube_segments=8))
    sdk_repaired, sdk_report = GeometrySDK().eliminate_tunnels(ring(radial_segments=24, tube_segments=8))
    diagnostics = rust.tunnel_diagnostics(repaired)

    assert repaired.vertex_count == 192
    assert repaired.face_count == 380
    assert report.input_face_count == 384
    assert report.detected_tunnel_face_count == 48
    assert report.removed_face_count == 48
    assert report.filled_holes == 2
    assert report.added_faces == 44
    assert report.output_face_count == 380
    assert report.output_boundary_edge_count == 0
    assert report.output_tunnel_count == 0
    assert report.tunnel_face_indices == rust.detect_tunnel_faces(ring(radial_segments=24, tube_segments=8))
    assert diagnostics.closed is True
    assert diagnostics.genus == 0
    assert diagnostics.tunnel_count == 0
    assert len(rust.detect_tunnel_faces(repaired)) == 0
    assert sdk_report == report
    assert sdk_repaired.vertex_count == repaired.vertex_count
    assert sdk_repaired.face_count == repaired.face_count


def test_engine_exposes_sdf_rebuild() -> None:
    sdk = GeometrySDK()
    rebuilt, report = sdk.rebuild_via_sdf(open_cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)

    assert sdk.health(rebuilt).is_closed
    assert report.voxel_size_mm == 0.5


def test_voxel_rebuild_is_rust_owned(monkeypatch) -> None:
    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel rebuild_via_sdf is required"):
        rebuild_via_sdf(open_cube(size=2.0), voxel_size_mm=0.5, padding_mm=0.5)
