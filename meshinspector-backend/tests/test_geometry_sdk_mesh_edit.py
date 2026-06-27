from __future__ import annotations

import numpy as np

from geometry_sdk.mesh_edit import decimate_mesh, make_delone_edge_flips, offset_verts_mesh, subdivide_mesh
from geometry_sdk.types import MeshDocument


def _square_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )


def _square_fan_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.5, 0.5, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]], dtype=np.int64),
    )


def _right_triangle_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )


def _skinny_triangle_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )


def _curved_priority_fixture_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, 10.0, 0.0],
                [2.0, 10.0, 0.0],
                [0.0, 12.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3], [4, 5, 6]], dtype=np.int64),
    )


def _folded_square_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )


def _subdivide_not_flippable_fixture_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [2.6276049261498553, 2.9361648936968914, 0.7212656061740566],
                [2.0369637564197727, 0.16430872643309868, 1.5154317688237702],
                [1.6171991057049149, 0.5114846825888412, 1.9134006098472023],
                [0.7822080654816956, 1.7910118323907225, 0.21890750193281283],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )


def _non_delone_quad_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.0, 2.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )


def _skew_non_delone_quad_mesh() -> MeshDocument:
    return MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 1.0],
                [2.0, 2.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )


def _mesh_has_edge(faces: np.ndarray, edge: tuple[int, int]) -> bool:
    ordered = tuple(sorted(edge))
    for face in np.asarray(faces, dtype=np.int64):
        for first, second in ((face[0], face[1]), (face[1], face[2]), (face[2], face[0])):
            if tuple(sorted((int(first), int(second)))) == ordered:
                return True
    return False


def test_make_delone_edge_flips_matches_meshlib_quadrangle_diagonal_contract() -> None:
    result = make_delone_edge_flips(_non_delone_quad_mesh(), num_iters=1)

    assert result[1] == 1
    np.testing.assert_array_equal(result[0].vertices, _non_delone_quad_mesh().vertices)
    np.testing.assert_array_equal(result[0].faces, np.asarray([[1, 3, 0], [3, 1, 2]], dtype=np.int64))
    assert result[0].metadata["operation"] == "make_delone_edge_flips"
    assert result[0].metadata["meshlib_reference"] == "MR::makeDeloneEdgeFlips"


def test_make_delone_edge_flips_honors_meshlib_not_flippable_constraint() -> None:
    protected = make_delone_edge_flips(
        _non_delone_quad_mesh(),
        num_iters=1,
        not_flippable_edges=np.asarray([[2, 0]], dtype=np.int64),
    )

    assert protected[1] == 0
    np.testing.assert_array_equal(protected[0].faces, _non_delone_quad_mesh().faces)
    assert protected[0].metadata["not_flippable_edges"] == [[2, 0]]


def test_make_delone_edge_flips_honors_meshlib_vert_region_constraint() -> None:
    mesh = _non_delone_quad_mesh()
    mesh_with_unused_vertex = MeshDocument(
        np.vstack([mesh.vertices, np.asarray([[10.0, 10.0, 0.0]], dtype=np.float64)]),
        mesh.faces,
    )

    blocked = make_delone_edge_flips(
        mesh_with_unused_vertex,
        num_iters=1,
        vert_region=np.asarray([4], dtype=np.int64),
    )
    allowed = make_delone_edge_flips(
        mesh_with_unused_vertex,
        num_iters=1,
        vert_region=np.asarray([1], dtype=np.int64),
    )

    assert blocked[1] == 0
    np.testing.assert_array_equal(blocked[0].faces, mesh.faces)
    assert blocked[0].metadata["vert_region"] == [4]
    assert allowed[1] == 1
    np.testing.assert_array_equal(allowed[0].faces, np.asarray([[1, 3, 0], [3, 1, 2]], dtype=np.int64))
    assert allowed[0].metadata["vert_region"] == [1]


def test_make_delone_edge_flips_honors_meshlib_max_deviation_after_flip() -> None:
    unconstrained = make_delone_edge_flips(_skew_non_delone_quad_mesh(), num_iters=1)
    constrained = make_delone_edge_flips(
        _skew_non_delone_quad_mesh(),
        num_iters=1,
        max_deviation_after_flip=0.1,
    )

    assert unconstrained[1] == 1
    np.testing.assert_array_equal(unconstrained[0].faces, np.asarray([[1, 3, 0], [3, 1, 2]], dtype=np.int64))
    assert constrained[1] == 0
    np.testing.assert_array_equal(constrained[0].faces, _skew_non_delone_quad_mesh().faces)
    assert constrained[0].metadata["max_deviation_after_flip"] == 0.1


def test_make_delone_edge_flips_honors_meshlib_max_angle_change() -> None:
    unconstrained = make_delone_edge_flips(_skew_non_delone_quad_mesh(), num_iters=1)
    constrained = make_delone_edge_flips(
        _skew_non_delone_quad_mesh(),
        num_iters=1,
        max_angle_change=0.5,
    )

    assert unconstrained[1] == 1
    np.testing.assert_array_equal(unconstrained[0].faces, np.asarray([[1, 3, 0], [3, 1, 2]], dtype=np.int64))
    assert constrained[1] == 0
    np.testing.assert_array_equal(constrained[0].faces, _skew_non_delone_quad_mesh().faces)
    assert constrained[0].metadata["max_angle_change"] == 0.5


def test_make_delone_edge_flips_honors_meshlib_critical_tri_aspect_ratio() -> None:
    angle_constrained = make_delone_edge_flips(
        _skew_non_delone_quad_mesh(),
        num_iters=1,
        max_angle_change=0.5,
    )
    aspect_critical = make_delone_edge_flips(
        _skew_non_delone_quad_mesh(),
        num_iters=1,
        max_angle_change=0.5,
        critical_tri_aspect_ratio=2.0,
    )

    assert angle_constrained[1] == 0
    np.testing.assert_array_equal(angle_constrained[0].faces, _skew_non_delone_quad_mesh().faces)
    assert aspect_critical[1] == 1
    np.testing.assert_array_equal(aspect_critical[0].faces, np.asarray([[1, 3, 0], [3, 1, 2]], dtype=np.int64))
    assert aspect_critical[0].metadata["critical_tri_aspect_ratio"] == 2.0


def test_subdivide_mesh_matches_meshlib_square_region_counts() -> None:
    result = subdivide_mesh(
        _square_mesh(),
        max_edge_len=0.3,
        max_edge_splits=1000,
        region_faces=np.asarray([0], dtype=np.int64),
    )

    assert result.mesh.vertex_count == 26
    assert result.mesh.face_count == 40
    assert result.splits_done == 22
    assert result.region_face_count == 32


def test_subdivide_mesh_honors_meshlib_not_flippable_delone_guard() -> None:
    unprotected = subdivide_mesh(
        _subdivide_not_flippable_fixture_mesh(),
        max_edge_len=0.01,
        max_edge_splits=1,
        region_faces=np.asarray([0, 1], dtype=np.int64),
    )
    protected = subdivide_mesh(
        _subdivide_not_flippable_fixture_mesh(),
        max_edge_len=0.01,
        max_edge_splits=1,
        region_faces=np.asarray([0, 1], dtype=np.int64),
        not_flippable_edges=np.asarray([[0, 2]], dtype=np.int64),
    )

    assert unprotected.splits_done == 1
    assert protected.splits_done == 1
    assert not _mesh_has_edge(unprotected.mesh.faces, (0, 2))
    assert _mesh_has_edge(protected.mesh.faces, (0, 2))
    assert protected.mesh.metadata["region_faces"] == [0, 1]
    assert protected.mesh.metadata["not_flippable_edges"] == [[0, 2]]


def test_subdivide_mesh_honors_meshlib_max_deviation_after_flip() -> None:
    unconstrained = subdivide_mesh(
        _subdivide_not_flippable_fixture_mesh(),
        max_edge_len=0.01,
        max_edge_splits=1,
        region_faces=np.asarray([0, 1], dtype=np.int64),
    )
    constrained = subdivide_mesh(
        _subdivide_not_flippable_fixture_mesh(),
        max_edge_len=0.01,
        max_edge_splits=1,
        region_faces=np.asarray([0, 1], dtype=np.int64),
        max_deviation_after_flip=0.01,
    )

    assert unconstrained.splits_done == 1
    assert constrained.splits_done == 1
    assert not _mesh_has_edge(unconstrained.mesh.faces, (0, 2))
    assert _mesh_has_edge(constrained.mesh.faces, (0, 2))
    assert constrained.mesh.metadata["max_deviation_after_flip"] == 0.01


def test_subdivide_mesh_honors_meshlib_max_angle_change_and_critical_aspect_flip() -> None:
    angle_constrained = subdivide_mesh(
        _subdivide_not_flippable_fixture_mesh(),
        max_edge_len=0.01,
        max_edge_splits=1,
        region_faces=np.asarray([0, 1], dtype=np.int64),
        max_angle_change_after_flip=0.01,
    )
    aspect_critical = subdivide_mesh(
        _subdivide_not_flippable_fixture_mesh(),
        max_edge_len=0.01,
        max_edge_splits=1,
        region_faces=np.asarray([0, 1], dtype=np.int64),
        max_angle_change_after_flip=0.01,
        critical_tri_aspect_ratio_flip=1.0,
    )

    assert angle_constrained.splits_done == 1
    assert aspect_critical.splits_done == 1
    assert _mesh_has_edge(angle_constrained.mesh.faces, (0, 2))
    assert not _mesh_has_edge(aspect_critical.mesh.faces, (0, 2))
    assert angle_constrained.mesh.metadata["max_angle_change_after_flip"] == 0.01
    assert aspect_critical.mesh.metadata["critical_tri_aspect_ratio_flip"] == 1.0


def test_offset_verts_mesh_matches_meshlib_pseudonormal_vertex_offsets() -> None:
    offsets = np.asarray([0.10, 0.20, 0.00, -0.05], dtype=np.float32)
    result = offset_verts_mesh(_square_mesh(), offsets)

    np.testing.assert_array_equal(result.faces, _square_mesh().faces)
    np.testing.assert_allclose(result.vertices[:, :2], _square_mesh().vertices[:, :2], atol=1e-12)
    np.testing.assert_allclose(result.vertices[:, 2], offsets.astype(np.float64), atol=1e-12)
    assert result.metadata["source"] == "rust_offset_verts"
    assert result.metadata["meshlib_reference"] == "MR::offsetVerts"
    assert result.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MROffsetVerts.*"


def test_decimate_mesh_shortest_edge_first_collapses_and_reports_meshlib_counters() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        max_deleted_vertices=1,
        pack_mesh=True,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    assert result.cancelled is False
    assert result.mesh.vertex_count == 3
    assert result.mesh.face_count == 1
    np.testing.assert_allclose(result.mesh.vertices[0], [0.05, 0.0, 0.0], atol=1e-12)
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 1, 2]], dtype=np.int64))
    assert result.mesh.metadata["operation"] == "decimate_mesh"
    assert result.mesh.metadata["strategy"] == "shortest_edge_first"
    assert result.mesh.metadata["meshlib_reference"] == "MR::decimateMesh"


def test_decimate_mesh_minimize_error_uses_qem_deviation_not_edge_length() -> None:
    result = decimate_mesh(
        _square_mesh(),
        strategy="minimize_error",
        max_error=0.9,
        max_deleted_vertices=1,
        max_deleted_faces=2,
        pack_mesh=True,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    assert result.error_introduced <= 0.9
    assert result.cancelled is False
    assert result.mesh.face_count <= 1
    assert result.mesh.metadata["strategy"] == "minimize_error"
    assert result.mesh.metadata["meshlib_reference"] == "MR::decimateMesh"


def test_decimate_mesh_honors_meshlib_angle_weighted_face_plane_qem() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.705869, 0.340508, 0.212305],
                [-0.187224, 1.464488, 0.373414],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 1, 3], [1, 4, 2], [0, 3, 4]], dtype=np.int64),
    )

    unweighted = decimate_mesh(
        mesh,
        strategy="minimize_error",
        max_deleted_vertices=1,
        max_deleted_faces=2,
        angle_weighted_dist_to_plane=False,
        pack_mesh=False,
    )
    weighted = decimate_mesh(
        mesh,
        strategy="minimize_error",
        max_deleted_vertices=1,
        max_deleted_faces=2,
        angle_weighted_dist_to_plane=True,
        pack_mesh=False,
    )

    assert unweighted.verts_deleted == 1
    assert weighted.verts_deleted == 1
    assert unweighted.mesh.metadata["angle_weighted_dist_to_plane"] is False
    assert weighted.mesh.metadata["angle_weighted_dist_to_plane"] is True
    assert not np.array_equal(unweighted.mesh.faces, weighted.mesh.faces)
    np.testing.assert_allclose(unweighted.mesh.vertices[2], [-0.073394, 1.267286, 0.219714], atol=1e-5)
    np.testing.assert_allclose(weighted.mesh.vertices[0], [-0.02175, 0.950335, 0.066332], atol=1e-5)


def test_decimate_mesh_honors_meshlib_qem_stabilizer() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-0.355003, 0.768589, -0.355005],
                [0.584114, -0.325361, -0.291144],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 1, 3], [1, 4, 2], [0, 3, 4]], dtype=np.int64),
    )

    default_stabilizer = decimate_mesh(
        mesh,
        strategy="minimize_error",
        max_deleted_vertices=1,
        max_deleted_faces=2,
        stabilizer=0.001,
        pack_mesh=False,
    )
    strong_stabilizer = decimate_mesh(
        mesh,
        strategy="minimize_error",
        max_deleted_vertices=1,
        max_deleted_faces=2,
        stabilizer=1.0,
        pack_mesh=False,
    )

    assert default_stabilizer.verts_deleted == 1
    assert strong_stabilizer.verts_deleted == 1
    assert default_stabilizer.mesh.metadata["stabilizer"] == 0.001
    assert strong_stabilizer.mesh.metadata["stabilizer"] == 1.0
    assert not np.array_equal(default_stabilizer.mesh.faces, strong_stabilizer.mesh.faces)
    np.testing.assert_allclose(default_stabilizer.mesh.vertices[0], [0.096007, 0.129607, -0.067989], atol=1e-5)
    np.testing.assert_allclose(strong_stabilizer.mesh.vertices[0], [-0.08609, 0.337451, -0.137335], atol=1e-5)


def test_decimate_mesh_default_settings_match_meshlib_half_face_guard() -> None:
    mesh = _square_fan_mesh()

    result = decimate_mesh(mesh)

    assert result.faces_deleted == mesh.face_count // 2
    assert result.mesh.face_count == mesh.face_count - mesh.face_count // 2
    assert result.mesh.metadata["meshlib_default_half_face_limit"] is True


def test_decimate_mesh_target_face_count_maps_to_meshlib_deleted_face_limit() -> None:
    mesh = _square_fan_mesh()

    result = decimate_mesh(mesh, target_face_count=3)

    assert result.faces_deleted == 0
    assert result.mesh.face_count == mesh.face_count
    assert result.mesh.metadata["target_face_count"] == 3
    assert result.mesh.metadata["target_face_ratio"] is None
    assert result.mesh.metadata["meshlib_default_half_face_limit"] is False


def test_decimate_mesh_target_face_ratio_maps_to_meshlib_deleted_face_limit() -> None:
    mesh = _square_fan_mesh()

    result = decimate_mesh(mesh, target_face_ratio=0.5)

    assert result.faces_deleted == 2
    assert result.mesh.face_count == 2
    assert result.mesh.metadata["target_face_count"] is None
    assert result.mesh.metadata["target_face_ratio"] == 0.5
    assert result.mesh.metadata["meshlib_default_half_face_limit"] is False


def test_decimate_mesh_subdivide_parts_can_preserve_part_boundaries() -> None:
    mesh = _square_fan_mesh()

    result = decimate_mesh(
        mesh,
        max_deleted_faces=2,
        subdivide_parts=2,
        decimate_between_parts=False,
    )

    assert result.faces_deleted == 0
    assert result.mesh.face_count == mesh.face_count
    assert result.mesh.metadata["subdivide_parts"] == 2
    assert result.mesh.metadata["decimate_between_parts"] is False


def test_decimate_mesh_subdivide_parts_final_between_parts_pass_decimates_boundary() -> None:
    mesh = _square_fan_mesh()

    result = decimate_mesh(
        mesh,
        max_deleted_faces=2,
        subdivide_parts=2,
        decimate_between_parts=True,
    )

    assert result.faces_deleted == 2
    assert result.mesh.face_count == 2
    assert result.mesh.metadata["subdivide_parts"] == 2
    assert result.mesh.metadata["decimate_between_parts"] is True


def test_decimate_mesh_honors_meshlib_max_triangle_aspect_ratio_guard() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.090_871_211_463_571_5, 0.0],
                [2.0, 0.0, 0.0],
                [1.5, 0.866_025_403_784_438_6, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [1, 3, 4]], dtype=np.int64),
    )

    blocked = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=1.1,
        max_triangle_aspect_ratio=1.05,
        max_deleted_vertices=1,
        region_faces=np.asarray([0], dtype=np.int64),
        pack_mesh=True,
    )
    allowed = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=1.1,
        max_triangle_aspect_ratio=2.0,
        max_deleted_vertices=1,
        region_faces=np.asarray([0], dtype=np.int64),
        pack_mesh=True,
    )

    assert blocked.verts_deleted == 0
    assert blocked.faces_deleted == 0
    assert blocked.mesh.metadata["region_faces"] == [0]
    assert allowed.verts_deleted == 1
    assert allowed.faces_deleted == 1
    assert allowed.mesh.vertex_count == 3
    assert allowed.mesh.face_count == 1
    assert allowed.mesh.metadata["region_faces"] == [0]
    assert allowed.mesh.metadata["max_triangle_aspect_ratio"] == 2.0


def test_decimate_mesh_honors_meshlib_critical_triangle_aspect_ratio_relaxation() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.090_871_211_463_571_5, 0.0],
                [2.0, 0.0, 0.0],
                [1.5, 0.866_025_403_784_438_6, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [1, 3, 4]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=1.1,
        max_triangle_aspect_ratio=1.05,
        critical_tri_aspect_ratio=1.0,
        max_deleted_vertices=1,
        region_faces=np.asarray([0], dtype=np.int64),
        pack_mesh=True,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    assert result.mesh.vertex_count == 3
    assert result.mesh.face_count == 1
    assert result.mesh.metadata["critical_tri_aspect_ratio"] == 1.0


def test_decimate_mesh_honors_meshlib_tiny_edge_length_aspect_bypass() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 1.090_871_211_463_571_5, 0.0],
                [2.0, 0.0, 0.0],
                [1.5, 0.866_025_403_784_438_6, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [1, 3, 4]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=1.1,
        max_triangle_aspect_ratio=1.05,
        tiny_edge_length=1.1,
        optimize_vertex_pos=False,
        max_deleted_vertices=1,
        region_faces=np.asarray([0], dtype=np.int64),
        pack_mesh=True,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    assert result.mesh.vertex_count == 3
    assert result.mesh.face_count == 1
    assert result.mesh.metadata["tiny_edge_length"] == 1.1


def test_decimate_mesh_honors_meshlib_max_angle_change_delone_flip() -> None:
    mesh = _non_delone_quad_mesh()

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.1,
        max_angle_change=0.0,
        max_deleted_vertices=1,
        max_deleted_faces=2,
    )

    assert result.verts_deleted == 0
    assert result.faces_deleted == 0
    np.testing.assert_array_equal(result.mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[1, 3, 0], [3, 1, 2]], dtype=np.int64))
    assert result.mesh.metadata["max_angle_change"] == 0.0


def test_decimate_mesh_flips_meshlib_twin_edge_with_max_angle_change() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.0, 2.0, 0.0],
                [0.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [6.0, 0.0, 0.0],
                [6.0, 2.0, 0.0],
                [4.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.1,
        max_angle_change=0.0,
        max_deleted_vertices=1,
        max_deleted_faces=2,
        twin_map=np.asarray([[[0, 2], [4, 6]], [[4, 6], [0, 2]]], dtype=np.int64),
    )

    assert result.verts_deleted == 0
    assert result.faces_deleted == 0
    np.testing.assert_array_equal(result.mesh.vertices, mesh.vertices)
    np.testing.assert_array_equal(
        result.mesh.faces,
        np.asarray([[1, 3, 0], [3, 1, 2], [5, 7, 4], [7, 5, 6]], dtype=np.int64),
    )
    assert result.mesh.metadata["remapped_twin_map"] == [[[1, 3], [5, 7]], [[5, 7], [1, 3]]]


def test_decimate_mesh_honors_meshlib_touch_near_boundary_edges_false() -> None:
    blocked = decimate_mesh(
        _square_mesh(),
        strategy="shortest_edge_first",
        max_error=2.0,
        touch_near_bd_edges=False,
        max_deleted_vertices=1,
        pack_mesh=True,
    )
    allowed = decimate_mesh(
        _square_mesh(),
        strategy="shortest_edge_first",
        max_error=2.0,
        touch_near_bd_edges=True,
        max_deleted_vertices=1,
        pack_mesh=True,
    )

    assert blocked.verts_deleted == 0
    assert blocked.faces_deleted == 0
    np.testing.assert_allclose(blocked.mesh.vertices, _square_mesh().vertices, atol=1e-12)
    np.testing.assert_array_equal(blocked.mesh.faces, _square_mesh().faces)
    assert blocked.mesh.metadata["touch_near_bd_edges"] is False
    assert allowed.verts_deleted == 1
    assert allowed.faces_deleted == 1
    assert allowed.mesh.vertex_count == 3
    assert allowed.mesh.face_count == 1
    assert allowed.mesh.metadata["touch_near_bd_edges"] is True


def test_decimate_mesh_honors_meshlib_touch_boundary_vertices_false() -> None:
    preserve_boundary = decimate_mesh(
        _square_fan_mesh(),
        strategy="shortest_edge_first",
        max_error=1.0,
        touch_bd_verts=False,
        max_deleted_vertices=1,
        pack_mesh=False,
    )
    move_boundary = decimate_mesh(
        _square_fan_mesh(),
        strategy="shortest_edge_first",
        max_error=1.0,
        touch_bd_verts=True,
        max_deleted_vertices=1,
        pack_mesh=False,
    )

    assert preserve_boundary.verts_deleted == 1
    assert preserve_boundary.faces_deleted == 2
    np.testing.assert_allclose(preserve_boundary.mesh.vertices[0], [0.0, 0.0, 0.0], atol=1e-12)
    np.testing.assert_allclose(move_boundary.mesh.vertices[0], [0.25, 0.25, 0.0], atol=1e-12)
    assert preserve_boundary.mesh.metadata["touch_bd_verts"] is False
    assert move_boundary.mesh.metadata["touch_bd_verts"] is True


def test_decimate_mesh_honors_meshlib_max_boundary_shift_guard() -> None:
    blocked = decimate_mesh(
        _square_fan_mesh(),
        strategy="shortest_edge_first",
        max_error=0.8,
        max_bd_shift=0.2,
        max_deleted_vertices=1,
        pack_mesh=False,
    )
    allowed = decimate_mesh(
        _square_fan_mesh(),
        strategy="shortest_edge_first",
        max_error=0.8,
        max_bd_shift=0.3,
        max_deleted_vertices=1,
        pack_mesh=False,
    )

    assert blocked.verts_deleted == 0
    assert blocked.faces_deleted == 0
    np.testing.assert_allclose(blocked.mesh.vertices, _square_fan_mesh().vertices, atol=1e-12)
    np.testing.assert_array_equal(blocked.mesh.faces, _square_fan_mesh().faces)
    assert blocked.mesh.metadata["max_bd_shift"] == 0.2
    assert allowed.verts_deleted == 1
    assert allowed.faces_deleted == 2
    np.testing.assert_allclose(allowed.mesh.vertices[0], [0.25, 0.25, 0.0], atol=1e-12)
    assert allowed.mesh.metadata["max_bd_shift"] == 0.3


def test_decimate_mesh_honors_meshlib_not_flippable_adjacent_collapse_guard() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
    )

    blocked = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        not_flippable_edges=np.asarray([[1, 3]], dtype=np.int64),
        collapse_near_not_flippable=False,
        max_deleted_vertices=1,
        pack_mesh=True,
    )
    allowed = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        not_flippable_edges=np.asarray([[1, 3]], dtype=np.int64),
        collapse_near_not_flippable=True,
        max_deleted_vertices=1,
        pack_mesh=True,
    )

    assert blocked.verts_deleted == 0
    assert blocked.faces_deleted == 0
    assert blocked.mesh.metadata["not_flippable_edges"] == [[1, 3]]
    assert blocked.mesh.metadata["collapse_near_not_flippable"] is False
    assert allowed.verts_deleted == 1
    assert allowed.faces_deleted == 1
    assert allowed.mesh.vertex_count == 3
    assert allowed.mesh.face_count == 1
    assert allowed.mesh.metadata["collapse_near_not_flippable"] is True


def test_decimate_mesh_reports_meshlib_remapped_not_flippable_edges_after_collapse() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        not_flippable_edges=np.asarray([[1, 3]], dtype=np.int64),
        collapse_near_not_flippable=True,
        max_deleted_vertices=1,
        pack_mesh=False,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 2, 3]], dtype=np.int64))
    assert result.mesh.metadata["not_flippable_edges"] == [[1, 3]]
    assert result.mesh.metadata["remapped_not_flippable_edges"] == [[0, 3]]


def test_decimate_mesh_honors_meshlib_edges_to_collapse_subset_and_remaps_it() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3], [2, 4, 3]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=2.0,
        edges_to_collapse=np.asarray([[1, 2]], dtype=np.int64),
        max_deleted_vertices=1,
        pack_mesh=False,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 1, 3], [1, 4, 3]], dtype=np.int64))
    np.testing.assert_allclose(result.mesh.vertices[1], [0.525, 0.0, 0.0], atol=1e-12)
    assert result.mesh.metadata["edges_to_collapse"] == [[1, 2]]
    assert result.mesh.metadata["remapped_edges_to_collapse"] == []


def test_decimate_mesh_honors_empty_meshlib_edges_to_collapse_subset() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=2.0,
        edges_to_collapse=np.empty((0, 2), dtype=np.int64),
        max_deleted_vertices=1,
        pack_mesh=False,
    )

    assert result.verts_deleted == 0
    assert result.faces_deleted == 0
    np.testing.assert_array_equal(result.mesh.faces, mesh.faces)
    assert result.mesh.metadata["edges_to_collapse"] == []
    assert result.mesh.metadata["remapped_edges_to_collapse"] == []


def test_decimate_mesh_remaps_meshlib_twin_map_after_collapse() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        twin_map=np.asarray([[[1, 3], [1, 2]], [[1, 2], [1, 3]]], dtype=np.int64),
        max_deleted_vertices=1,
        pack_mesh=False,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 2, 3]], dtype=np.int64))
    assert result.mesh.metadata["twin_map"] == [[[1, 3], [1, 2]], [[1, 2], [1, 3]]]
    assert result.mesh.metadata["remapped_twin_map"] == [[[0, 2], [0, 3]], [[0, 3], [0, 2]]]


def test_decimate_mesh_collapses_meshlib_twin_edge_with_same_position() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.15, 0.0, 0.0],
                [11.0, 0.0, 0.0],
                [10.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3], [4, 5, 7], [5, 6, 7]], dtype=np.int64),
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        twin_map=np.asarray([[[0, 1], [4, 5]], [[4, 5], [0, 1]]], dtype=np.int64),
        max_deleted_vertices=1,
        max_triangle_aspect_ratio=1_000_000.0,
        pack_mesh=False,
    )

    assert result.verts_deleted == 2
    assert result.faces_deleted == 2
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 2, 3], [4, 6, 7]], dtype=np.int64))
    np.testing.assert_allclose(result.mesh.vertices[0], [0.05, 0.0, 0.0], atol=1e-12)
    np.testing.assert_allclose(result.mesh.vertices[4], [0.05, 0.0, 0.0], atol=1e-12)
    assert result.mesh.metadata["remapped_twin_map"] == []


def test_decimate_mesh_interpolates_vertex_uvs_with_meshlib_pre_collapse_callback() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
        metadata={"vertex_uvs": [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [0.0, 1.0]]},
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        max_deleted_vertices=1,
        pack_mesh=True,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    assert result.mesh.vertex_count == 3
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 1, 2]], dtype=np.int64))
    np.testing.assert_allclose(result.mesh.metadata["vertex_uvs"], [[0.5, 0.0], [2.0, 0.0], [0.0, 1.0]])
    assert result.mesh.metadata["pre_collapse_vertex_attributes"] == ["vertex_uvs"]


def test_decimate_mesh_interpolates_vertex_colors_with_meshlib_pre_collapse_truncation() -> None:
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
        metadata={
            "vertex_colors": [
                [1, 10, 100, 255],
                [3, 20, 200, 127],
                [5, 30, 210, 255],
                [7, 40, 220, 255],
            ]
        },
    )

    result = decimate_mesh(
        mesh,
        strategy="shortest_edge_first",
        max_error=0.2,
        max_deleted_vertices=1,
        pack_mesh=True,
    )

    assert result.verts_deleted == 1
    assert result.faces_deleted == 1
    assert result.mesh.vertex_count == 3
    np.testing.assert_array_equal(result.mesh.faces, np.asarray([[0, 1, 2]], dtype=np.int64))
    assert result.mesh.metadata["vertex_colors"] == [[1, 15, 150, 190], [5, 30, 210, 255], [7, 40, 220, 255]]
    assert result.mesh.metadata["pre_collapse_vertex_attributes"] == ["vertex_colors"]


def test_subdivide_mesh_stops_at_meshlib_max_edge_splits_limit() -> None:
    result = subdivide_mesh(
        _square_mesh(),
        max_edge_len=0.3,
        max_edge_splits=10,
        region_faces=np.asarray([0], dtype=np.int64),
    )

    assert result.mesh.vertex_count == 14
    assert result.mesh.face_count == 18
    assert result.splits_done == 10
    assert result.region_face_count == 14


def test_subdivide_mesh_honors_meshlib_max_tri_aspect_ratio_stop() -> None:
    result = subdivide_mesh(
        _right_triangle_mesh(),
        max_edge_len=0.0,
        max_edge_splits=10,
        max_tri_aspect_ratio=1.3,
    )

    assert result.mesh.vertex_count == 3
    assert result.mesh.face_count == 1
    assert result.splits_done == 0


def test_subdivide_mesh_honors_meshlib_max_splittable_tri_aspect_ratio_gate() -> None:
    blocked = subdivide_mesh(
        _skinny_triangle_mesh(),
        max_edge_len=1.0,
        max_edge_splits=10,
        max_splittable_tri_aspect_ratio=5.0,
    )
    allowed = subdivide_mesh(
        _skinny_triangle_mesh(),
        max_edge_len=1.0,
        max_edge_splits=10,
        max_splittable_tri_aspect_ratio=6.0,
    )

    assert blocked.mesh.vertex_count == 3
    assert blocked.mesh.face_count == 1
    assert blocked.splits_done == 0
    assert allowed.mesh.vertex_count == 13
    assert allowed.mesh.face_count == 12
    assert allowed.splits_done == 10


def test_subdivide_mesh_honors_meshlib_curvature_priority_edge_ranking() -> None:
    flat_priority = subdivide_mesh(
        _curved_priority_fixture_mesh(),
        max_edge_len=0.0,
        max_edge_splits=1,
        curvature_priority=0.0,
    )
    curved_priority = subdivide_mesh(
        _curved_priority_fixture_mesh(),
        max_edge_len=0.0,
        max_edge_splits=1,
        curvature_priority=5.0,
    )

    assert flat_priority.splits_done == 1
    np.testing.assert_allclose(flat_priority.mesh.vertices[-1], [1.0, 11.0, 0.0])
    assert flat_priority.mesh.metadata["curvature_priority"] == 0.0
    assert curved_priority.splits_done == 1
    np.testing.assert_allclose(curved_priority.mesh.vertices[-1], [0.0, 0.5, 0.5])
    assert curved_priority.mesh.metadata["curvature_priority"] == 5.0


def test_subdivide_mesh_honors_meshlib_project_on_original_mesh() -> None:
    unprojected = subdivide_mesh(
        _folded_square_mesh(),
        max_edge_len=0.0,
        max_edge_splits=3,
        project_on_original_mesh=False,
    )
    projected = subdivide_mesh(
        _folded_square_mesh(),
        max_edge_len=0.0,
        max_edge_splits=3,
        project_on_original_mesh=True,
    )

    assert unprojected.splits_done == 3
    np.testing.assert_allclose(unprojected.mesh.vertices[-1], [0.75, 0.5, 0.25])
    assert unprojected.mesh.metadata["project_on_original_mesh"] is False
    assert projected.splits_done == 3
    np.testing.assert_allclose(projected.mesh.vertices[-1], [0.75, 0.5, 0.0])
    np.testing.assert_array_equal(projected.mesh.faces, unprojected.mesh.faces)
    assert projected.mesh.metadata["project_on_original_mesh"] is True


def test_subdivide_mesh_honors_meshlib_smooth_mode_without_sharp_constraints() -> None:
    unsmoothed = subdivide_mesh(
        _folded_square_mesh(),
        max_edge_len=0.0,
        max_edge_splits=3,
        smooth_mode=False,
        min_sharp_dihedral_angle=999.0,
    )
    smoothed = subdivide_mesh(
        _folded_square_mesh(),
        max_edge_len=0.0,
        max_edge_splits=3,
        smooth_mode=True,
        min_sharp_dihedral_angle=999.0,
    )

    assert unsmoothed.splits_done == 3
    np.testing.assert_allclose(unsmoothed.mesh.vertices[-1], [0.75, 0.5, 0.25])
    assert smoothed.splits_done == 3
    np.testing.assert_allclose(
        smoothed.mesh.vertices[-1],
        [0.873372078, 0.47443521, 0.031970274],
        rtol=0,
        atol=1e-6,
    )
    np.testing.assert_array_equal(smoothed.mesh.faces, unsmoothed.mesh.faces)
    assert smoothed.mesh.metadata["smooth_mode"] is True
    assert smoothed.mesh.metadata["min_sharp_dihedral_angle"] == 999.0
