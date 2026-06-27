from __future__ import annotations

import os
from dataclasses import asdict
from typing import Any

import numpy as np
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.adapters.meshlib_reference import boolean_mesh as meshlib_boolean_mesh
from geometry_sdk.adapters.meshlib_reference import health_metrics
from geometry_sdk.adapters.meshlib_reference import offset_mesh as meshlib_offset_mesh
from geometry_sdk.adapters.meshlib_reference import save_compare_npz as meshlib_save_compare_npz
from geometry_sdk.adapters.meshlib_reference import save_thickness_npz as meshlib_save_thickness_npz
from geometry_sdk.adapters.meshlib_reference import signed_distance_summary as meshlib_signed_distance_summary
from geometry_sdk.adapters.meshlib_reference import signed_distance_values as meshlib_signed_distance_values
from geometry_sdk.adapters.meshlib_reference import thickness_summary as meshlib_thickness_summary
from geometry_sdk.analysis.artifacts import load_compare_npz, load_thickness_npz
from geometry_sdk.analysis.compare import signed_compare_summary
from geometry_sdk.accelerators import _rust_common
from geometry_sdk.io.trimesh_adapter import from_trimesh, to_trimesh
from geometry_sdk.io.trimesh_adapter import load_mesh, save_mesh
from geometry_sdk.jewelry.hollow import drain_hole_cutters_mesh, plan_drain_holes
from geometry_sdk.repair.basic import orient_faces_outward
from geometry_sdk.testing.fixtures import box, crossing_triangles, cube, hollowed_ring, open_cube, pendant, ring, ring_with_head, thin_wall_ring
from geometry_sdk.testing.goldens import assert_metric_dict_close, load_golden
from geometry_sdk.types import MeshDocument
from geometry_sdk.voxel.mesh_ops import voxel_boolean_mesh, voxel_offset_mesh, voxel_shell_mesh


MESH_LIB_GOLDEN_FIXTURES = {
    "cube_2mm": lambda: cube(size=2.0),
    "open_cube_2mm": lambda: open_cube(size=2.0),
    "ring_default": ring,
    "ring_with_head": ring_with_head,
    "thin_wall_ring": thin_wall_ring,
    "hollowed_ring": hollowed_ring,
    "pendant": pendant,
    "crossing_triangles": crossing_triangles,
}


def _combine_operation_meshes(meshes: list[MeshDocument], *, source: str) -> MeshDocument:
    vertices: list[np.ndarray] = []
    faces: list[np.ndarray] = []
    offset = 0
    for mesh in meshes:
        vertices.append(mesh.vertices)
        faces.append(mesh.faces + offset)
        offset += mesh.vertex_count
    return MeshDocument(np.vstack(vertices), np.vstack(faces), metadata={"fixture": source, "count": len(meshes)})


def _ring_16x8_drain_hole_cutters():
    source = ring(radial_segments=16, tube_segments=8)
    sdk = GeometrySDK()
    measurement = sdk.measure_ring(source)
    regions = sdk.detect_ring_regions(source, measurement)
    plans = plan_drain_holes(
        source,
        regions,
        measurement.ring_axis,
        wall_thickness_mm=0.8,
        hole_diameter_mm=1.0,
    )
    return drain_hole_cutters_mesh(plans, sections=16)


def _ring_16x8_shell_0_8mm_voxel_0_5mm():
    return voxel_shell_mesh(
        ring(radial_segments=16, tube_segments=8),
        wall_thickness_mm=0.8,
        voxel_size_mm=0.5,
    )


def _ring_16x8_shell_1_0mm_voxel_0_5mm():
    return voxel_shell_mesh(
        ring(radial_segments=16, tube_segments=8),
        wall_thickness_mm=1.0,
        voxel_size_mm=0.5,
    )


def _ring_with_head_16x8_prong_box_cutters_4x():
    centers = [
        (-1.15, -0.58, 11.85),
        (1.15, -0.58, 11.85),
        (-1.15, 0.58, 11.85),
        (1.15, 0.58, 11.85),
    ]
    return _combine_operation_meshes(
        [box(0.48, 0.46, 2.9, center=center) for center in centers],
        source="prong_box_cutters",
    )


def _meshlib_id_value(value: Any) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        pass
    text = str(value)
    if "(" not in text or ")" not in text:
        return None
    body = text.split("(", 1)[1].split(")", 1)[0]
    if not body:
        return None
    return int(body)


def _meshlib_required_id_value(value: Any) -> int:
    parsed = _meshlib_id_value(value)
    assert parsed is not None, f"could not parse MeshLib id from {value!r}"
    return parsed


def _meshlib_mapped_face_count(mr: Any, face_map: Any) -> int:
    return sum(
        1
        for index in range(face_map.size())
        if (value := _meshlib_id_value(face_map[mr.FaceId(index)])) is not None
        and value >= 0
    )


def _meshlib_face_map_values(mr: Any, face_map: Any) -> list[int]:
    values: list[int] = []
    for index in range(face_map.size()):
        value = _meshlib_id_value(face_map[mr.FaceId(index)])
        if value is not None and value >= 0:
            values.append(value)
    return values


def _meshlib_mapped_face_indices(mr: Any, face_map: Any) -> list[int]:
    indices: list[int] = []
    for index in range(face_map.size()):
        value = _meshlib_id_value(face_map[mr.FaceId(index)])
        if value is not None and value >= 0:
            indices.append(index)
    return indices


def _meshlib_face_map_histogram(mr: Any, face_map: Any) -> list[list[int]]:
    values = _meshlib_face_map_values(mr, face_map)
    return _source_face_counts(values)


def _source_face_counts(values: list[int]) -> list[list[int]]:
    return [
        [source_face, values.count(source_face)]
        for source_face in sorted(set(values))
    ]


def _source_face_count_deltas(
    materialized_counts: list[list[int]],
    source_preserving_counts: list[list[int]],
) -> list[list[int]]:
    deltas: dict[int, int] = {}
    for source_face, count in source_preserving_counts:
        deltas[source_face] = deltas.get(source_face, 0) - count
    for source_face, count in materialized_counts:
        deltas[source_face] = deltas.get(source_face, 0) + count
    return [
        [source_face, delta]
        for source_face, delta in sorted(deltas.items())
        if delta
    ]


def _source_face_mismatch_details(
    materialized_values: list[int],
    source_preserving_values: list[int],
) -> list[list[int]]:
    return [
        [index, materialized, source_preserving]
        for index, (materialized, source_preserving) in enumerate(
            zip(materialized_values, source_preserving_values)
        )
        if materialized != source_preserving
    ]


def _source_face_runs(values: list[int]) -> list[list[int]]:
    runs: list[list[int]] = []
    for value in values:
        if runs and runs[-1][0] == value:
            runs[-1][1] += 1
        else:
            runs.append([value, 1])
    return runs


def _slots_by_lifecycle_runs(
    lifecycle_slot_paths: list[list[list[int]]],
    face_slots: list[int],
) -> list[list[int]]:
    if not lifecycle_slot_paths:
        return []
    return [
        [
            slot
            for slot in face_slots
            if lifecycle_slot[6] <= slot < lifecycle_slot[7]
        ]
        for lifecycle_slot in lifecycle_slot_paths[0]
    ]


def _slot_group_deltas(
    expected: list[list[int]],
    actual: list[list[int]],
) -> tuple[list[list[int]], list[list[int]]]:
    missing: list[list[int]] = []
    extra: list[list[int]] = []
    for index, expected_group in enumerate(expected):
        actual_group = actual[index] if index < len(actual) else []
        missing.append([slot for slot in expected_group if slot not in actual_group])
        extra.append([slot for slot in actual_group if slot not in expected_group])
    for actual_group in actual[len(expected) :]:
        extra.append(actual_group)
    return missing, extra


def _rotate_list(values: list[Any], rotation: int) -> list[Any]:
    if not values:
        return []
    offset = rotation % len(values)
    return values[offset:] + values[:offset]


def _meshlib_mapped_vert_count(mr: Any, vert_map: Any) -> int:
    return sum(
        1
        for index in range(vert_map.size())
        if (value := _meshlib_id_value(vert_map[mr.VertId(index)])) is not None
        and value >= 0
    )


def _meshlib_in_memory_boolean_mapper_summary(
    first: MeshDocument,
    second: MeshDocument,
    *,
    operation: str,
) -> dict[str, Any]:
    from meshlib import mrmeshpy as mr
    from meshlib import mrmeshnumpy as mn

    operations = {
        "union": mr.BooleanOperation.Union,
        "intersection": mr.BooleanOperation.Intersection,
        "difference": mr.BooleanOperation.DifferenceAB,
    }
    mapper = mr.BooleanResultMapper()
    params = mr.BooleanParameters()
    params.mapper = mapper
    result = mr.boolean(
        mn.meshFromFacesVerts(
            first.faces.astype(np.int32),
            first.vertices.astype(np.float32),
        ),
        mn.meshFromFacesVerts(
            second.faces.astype(np.int32),
            second.vertices.astype(np.float32),
        ),
        operations[operation],
        params,
    )
    assert result.valid(), result.errorString
    topology = result.mesh.topology
    maps_a = mapper.maps[0]
    maps_b = mapper.maps[1]
    cut2origin_a_values = _meshlib_face_map_values(mr, maps_a.cut2origin)
    cut2origin_b_values = _meshlib_face_map_values(mr, maps_b.cut2origin)
    first_face_count = int(first.faces.shape[0])
    second_face_count = int(second.faces.shape[0])
    return {
        "face_size": topology.faceSize(),
        "valid_faces": topology.numValidFaces(),
        "vert_size": topology.vertSize(),
        "valid_verts": topology.numValidVerts(),
        "cut2origin_a_size": maps_a.cut2origin.size(),
        "cut2origin_b_size": maps_b.cut2origin.size(),
        "cut2origin_a_valid": len(cut2origin_a_values),
        "cut2origin_b_valid": len(cut2origin_b_values),
        "cut2origin_a_unique_origins": len(set(cut2origin_a_values)),
        "cut2origin_b_unique_origins": len(set(cut2origin_b_values)),
        "cut2origin_a_duplicate_cut_faces": len(cut2origin_a_values)
        - len(set(cut2origin_a_values)),
        "cut2origin_b_duplicate_cut_faces": len(cut2origin_b_values)
        - len(set(cut2origin_b_values)),
        "cut2origin_a_values": cut2origin_a_values,
        "cut2origin_b_values": cut2origin_b_values,
        "cut2origin_a_appended_values": cut2origin_a_values[first_face_count:],
        "cut2origin_b_appended_values": cut2origin_b_values[second_face_count:],
        "cut2origin_a_appended_runs": _source_face_runs(
            cut2origin_a_values[first_face_count:]
        ),
        "cut2origin_b_appended_runs": _source_face_runs(
            cut2origin_b_values[second_face_count:]
        ),
        "cut2origin_a_histogram": _meshlib_face_map_histogram(mr, maps_a.cut2origin),
        "cut2origin_b_histogram": _meshlib_face_map_histogram(mr, maps_b.cut2origin),
        "mapped_a_faces": _meshlib_mapped_face_count(mr, maps_a.cut2newFaces),
        "mapped_b_faces": _meshlib_mapped_face_count(mr, maps_b.cut2newFaces),
        "mapped_a_cut_faces": _meshlib_mapped_face_indices(mr, maps_a.cut2newFaces),
        "mapped_b_cut_faces": _meshlib_mapped_face_indices(mr, maps_b.cut2newFaces),
        "mapped_a_verts": _meshlib_mapped_vert_count(mr, maps_a.old2newVerts),
        "mapped_b_verts": _meshlib_mapped_vert_count(mr, maps_b.old2newVerts),
    }


def _meshlib_pre_cut_cutmesh_mapper_summary(
    first: MeshDocument,
    second: MeshDocument,
    *,
    operation: str,
) -> dict[str, Any]:
    from meshlib import mrmeshpy as mr
    from meshlib import mrmeshnumpy as mn

    operations = {
        "union": mr.BooleanOperation.Union,
        "intersection": mr.BooleanOperation.Intersection,
        "difference": mr.BooleanOperation.DifferenceAB,
    }
    pre_a = mr.BooleanPreCutResult()
    pre_b = mr.BooleanPreCutResult()
    params = mr.BooleanParameters()
    params.outPreCutA = pre_a
    params.outPreCutB = pre_b
    params.forceCut = True
    result = mr.boolean(
        mn.meshFromFacesVerts(
            first.faces.astype(np.int32),
            first.vertices.astype(np.float32),
        ),
        mn.meshFromFacesVerts(
            second.faces.astype(np.int32),
            second.vertices.astype(np.float32),
        ),
        operations[operation],
        params,
    )
    assert result.valid(), result.errorString
    pre_cut_contour_lengths_a = [len(contour.intersections) for contour in pre_a.contours]
    pre_cut_contour_lengths_b = [len(contour.intersections) for contour in pre_b.contours]
    pre_cut_contour_closed_a = [bool(contour.closed) for contour in pre_a.contours]
    pre_cut_contour_closed_b = [bool(contour.closed) for contour in pre_b.contours]

    def primitive_segment_start_values(pre_cut: Any) -> tuple[
        list[list[int]],
        list[list[int]],
    ]:
        contour_kinds: list[list[int]] = []
        contour_faces: list[list[int]] = []
        for contour in pre_cut.contours:
            kinds: list[int] = []
            faces: list[int] = []
            for inter in list(contour.intersections)[:-1]:
                primitive = inter.primitiveId
                primitive_type = str(primitive.current_type())
                if "Vert" in primitive_type:
                    kinds.append(0)
                    faces.append(-1)
                elif "Edge" in primitive_type:
                    kinds.append(1)
                    faces.append(-1)
                elif "Face" in primitive_type:
                    kinds.append(2)
                    faces.append(_meshlib_required_id_value(primitive.get_Id_FaceTag()))
                else:
                    raise AssertionError(f"unknown MeshLib primitive type: {primitive_type}")
            contour_kinds.append(kinds)
            contour_faces.append(faces)
        return contour_kinds, contour_faces

    (
        pre_cut_start_primitive_kinds_a,
        pre_cut_start_primitive_faces_a,
    ) = primitive_segment_start_values(pre_a)
    (
        pre_cut_start_primitive_kinds_b,
        pre_cut_start_primitive_faces_b,
    ) = primitive_segment_start_values(pre_b)

    def cut_values(pre_cut: Any) -> tuple[
        list[int],
        int,
        int,
        int,
        list[int],
        list[list[int]],
        list[list[int]],
        list[list[list[int]]],
    ]:
        face_map = mr.FaceMap()
        edge_map = mr.NewEdgesMap()
        cut_params = mr.CutMeshParameters()
        cut_params.new2OldMap = face_map
        cut_params.new2oldEdgesMap = edge_map
        cut_params.forceFillMode = mr.CutMeshParameters.ForceFill.All
        cut_result = mr.cutMesh(pre_cut.mesh, pre_cut.contours, cut_params)
        result_cut_edges: list[list[int]] = []
        result_cut_old_faces: list[list[int]] = []
        result_cut_old_face_runs: list[list[list[int]]] = []
        for path_index in range(cut_result.resultCut.size()):
            path = cut_result.resultCut[path_index]
            path_edges: list[int] = []
            path_old_faces: list[int] = []
            for edge_index in range(path.size()):
                edge_id = _meshlib_required_id_value(path[edge_index])
                path_edges.append(edge_id)
                path_old_faces.append(
                    _meshlib_required_id_value(
                        edge_map.map[mr.UndirectedEdgeId(edge_id // 2)]
                    )
                )
            result_cut_edges.append(path_edges)
            result_cut_old_faces.append(path_old_faces)
            result_cut_old_face_runs.append(_source_face_runs(path_old_faces))
        return (
            _meshlib_face_map_values(mr, face_map),
            cut_result.resultCut.size(),
            cut_result.fbsWithContourIntersections.count(),
            pre_cut.mesh.topology.faceSize(),
            [
                face
                for face in range(pre_cut.mesh.topology.faceSize())
                if pre_cut.mesh.topology.hasFace(mr.FaceId(face))
            ],
            result_cut_edges,
            result_cut_old_faces,
            result_cut_old_face_runs,
        )

    (
        cut2origin_a_values,
        result_cut_a,
        bad_a,
        face_size_a,
        valid_cut_faces_a,
        result_cut_edges_a,
        result_cut_old_faces_a,
        result_cut_old_face_runs_a,
    ) = cut_values(pre_a)
    (
        cut2origin_b_values,
        result_cut_b,
        bad_b,
        face_size_b,
        valid_cut_faces_b,
        result_cut_edges_b,
        result_cut_old_faces_b,
        result_cut_old_face_runs_b,
    ) = cut_values(pre_b)
    first_face_count = int(first.faces.shape[0])
    second_face_count = int(second.faces.shape[0])
    return {
        "cut2origin_a_values": cut2origin_a_values,
        "cut2origin_b_values": cut2origin_b_values,
        "cut2origin_a_appended_values": cut2origin_a_values[first_face_count:],
        "cut2origin_b_appended_values": cut2origin_b_values[second_face_count:],
        "cut2origin_a_appended_runs": _source_face_runs(
            cut2origin_a_values[first_face_count:]
        ),
        "cut2origin_b_appended_runs": _source_face_runs(
            cut2origin_b_values[second_face_count:]
        ),
        "result_cut_paths": [result_cut_a, result_cut_b],
        "bad_contour_faces": [bad_a, bad_b],
        "face_sizes": [face_size_a, face_size_b],
        "valid_cut_faces": [valid_cut_faces_a, valid_cut_faces_b],
        "pre_cut_contour_lengths": [
            pre_cut_contour_lengths_a,
            pre_cut_contour_lengths_b,
        ],
        "pre_cut_contour_closed": [
            pre_cut_contour_closed_a,
            pre_cut_contour_closed_b,
        ],
        "pre_cut_contour_start_primitive_kinds": [
            pre_cut_start_primitive_kinds_a,
            pre_cut_start_primitive_kinds_b,
        ],
        "pre_cut_contour_start_primitive_faces": [
            pre_cut_start_primitive_faces_a,
            pre_cut_start_primitive_faces_b,
        ],
        "cut2origin_a_result_cut_edges": result_cut_edges_a,
        "cut2origin_b_result_cut_edges": result_cut_edges_b,
        "cut2origin_a_result_cut_old_faces": result_cut_old_faces_a,
        "cut2origin_b_result_cut_old_faces": result_cut_old_faces_b,
        "cut2origin_a_result_cut_old_face_runs": result_cut_old_face_runs_a,
        "cut2origin_b_result_cut_old_face_runs": result_cut_old_face_runs_b,
    }


OPERATION_FIXTURES = {
    "box_3_2x2_4x2_6_head_top_z_12": lambda: box(3.2, 2.4, 2.6, center=(0.0, 0.0, 12.0)),
    "box_3x2x8_center": lambda: box(3.0, 2.0, 8.0),
    "box_4x2x6_left_x_neg_3_5": lambda: box(4.0, 2.0, 6.0, center=(-3.5, 0.0, 0.0)),
    "box_5x3x5_side_x_9": lambda: box(5.0, 3.0, 5.0, center=(9.0, 0.0, 0.0)),
    "box_5x3x5_top_z_9_5": lambda: box(5.0, 3.0, 5.0, center=(0.0, 0.0, 9.5)),
    "cube_2mm": lambda: cube(size=2.0),
    "cube_2mm_shifted_x_1mm": lambda: cube(size=2.0).copy(vertices=cube(size=2.0).vertices + np.array([1.0, 0.0, 0.0])),
    "pendant": pendant,
    "ring_16x8": lambda: ring(radial_segments=16, tube_segments=8),
    "ring_16x8_drain_hole_cutters_1mm_16_sections": _ring_16x8_drain_hole_cutters,
    "ring_16x8_shell_0_8mm_voxel_0_5mm": _ring_16x8_shell_0_8mm_voxel_0_5mm,
    "ring_16x8_shell_1_0mm_voxel_0_5mm": _ring_16x8_shell_1_0mm_voxel_0_5mm,
    "ring_with_head_16x8": lambda: ring_with_head(radial_segments=16, tube_segments=8),
    "ring_with_head_16x8_prong_box_cutters_4x": _ring_with_head_16x8_prong_box_cutters_4x,
}

RING_CUTTER_BOOLEAN_GOLDENS = {
    "ring_16x8_side_box_difference",
    "ring_16x8_side_box_intersection",
    "ring_16x8_top_box_difference",
    "ring_16x8_top_box_intersection",
}
PENDANT_CUTTER_BOOLEAN_GOLDENS = {
    "pendant_center_box_difference",
    "pendant_center_box_intersection",
    "pendant_center_box_union",
    "pendant_left_box_difference",
    "pendant_left_box_intersection",
    "pendant_left_box_union",
}
DRAIN_HOLE_BOOLEAN_GOLDENS = {
    "ring_16x8_drain_hole_difference",
    "ring_16x8_shell_drain_hole_difference",
    "ring_16x8_shell_1_0mm_drain_hole_difference",
}
HEAD_LOCAL_BOOLEAN_GOLDENS = {
    "ring_with_head_16x8_head_top_box_difference",
    "ring_with_head_16x8_head_top_box_intersection",
    "ring_with_head_16x8_head_top_box_union",
}
PRONG_LIKE_BOOLEAN_GOLDENS = {
    "ring_with_head_16x8_prong_box_difference",
    "ring_with_head_16x8_prong_box_intersection",
    "ring_with_head_16x8_prong_box_union",
}
JEWELRY_CUTTER_BOOLEAN_GOLDENS = (
    RING_CUTTER_BOOLEAN_GOLDENS
    | PENDANT_CUTTER_BOOLEAN_GOLDENS
    | DRAIN_HOLE_BOOLEAN_GOLDENS
    | HEAD_LOCAL_BOOLEAN_GOLDENS
    | PRONG_LIKE_BOOLEAN_GOLDENS
)


def _forced_python_accelerator() -> bool:
    return os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python"


def _operation_fixture(name: str):
    return OPERATION_FIXTURES[name]()


def _operation_source_mesh(expected: dict[str, Any]):
    mesh = _operation_fixture(expected["source_fixture"])
    if expected.get("source_preprocess") == "orient_faces_outward":
        return orient_faces_outward(mesh)
    return mesh


def _operation_metric_payload(mesh) -> dict[str, Any]:
    stats = asdict(compute_mesh_stats(mesh))
    health = asdict(compute_mesh_health(mesh))
    return {
        "mesh": {"vertices": int(mesh.vertex_count), "faces": int(mesh.face_count)},
        "stats": stats,
        "health": health,
    }


def _assert_near_stitch_candidate_ring_details(details: list[dict[str, Any]]) -> None:
    for detail in details:
        for face_key, edge_key in (
            ("previous_source_halfedge_key_face", "previous_source_halfedge_key_edge"),
            ("next_source_halfedge_key_face", "next_source_halfedge_key_edge"),
        ):
            assert detail[face_key] is None or isinstance(detail[face_key], int)
            source_key_edge = detail[edge_key]
            assert source_key_edge is None or (
                isinstance(source_key_edge, list)
                and len(source_key_edge) == 2
                and all(isinstance(vertex, int) for vertex in source_key_edge)
            )
        candidate_diagnostics = detail["candidate_diagnostics"]
        if candidate_diagnostics is None:
            continue
        assert isinstance(candidate_diagnostics["attempt"], str)
        fallback_from = candidate_diagnostics["fallback_from"]
        if fallback_from is not None:
            assert set(fallback_from) == {
                "attempt",
                "error",
                "previous_candidates",
                "next_candidates",
                "failure_count",
            }
            assert isinstance(fallback_from["attempt"], str)
            assert isinstance(fallback_from["error"], str)
            assert isinstance(fallback_from["previous_candidates"], int)
            assert isinstance(fallback_from["next_candidates"], int)
            assert isinstance(fallback_from["failure_count"], int)
        for lookup_key in ("previous_source_lookup", "next_source_lookup"):
            lookup = candidate_diagnostics[lookup_key]
            if lookup is None:
                continue
            assert set(lookup) == {
                "requested_halfedge",
                "requested_key_face",
                "requested_key_edge",
                "requested_source_edge",
                "fallback_edge",
                "exact_key_candidates",
                "same_edge_key_candidates",
                "halfedge_candidates",
                "source_edge_candidates",
                "topology_candidates",
                "total_candidates",
                "copied_source_edge",
            }
            assert lookup["requested_halfedge"] is None or isinstance(
                lookup["requested_halfedge"], int
            )
            assert lookup["requested_key_face"] is None or isinstance(
                lookup["requested_key_face"], int
            )
            for edge_key in (
                "requested_key_edge",
                "requested_source_edge",
                "fallback_edge",
            ):
                edge = lookup[edge_key]
                if edge is None:
                    assert edge_key != "fallback_edge"
                    continue
                assert isinstance(edge, list) and len(edge) == 2
                assert all(isinstance(vertex, int) for vertex in edge)
            for count_key in (
                "exact_key_candidates",
                "same_edge_key_candidates",
                "halfedge_candidates",
                "source_edge_candidates",
                "topology_candidates",
                "total_candidates",
            ):
                assert isinstance(lookup[count_key], int)
            copied_source_edge = lookup["copied_source_edge"]
            if copied_source_edge is not None:
                assert set(copied_source_edge) == {
                    "status",
                    "matched_source_edge",
                    "source_halfedge",
                    "source_origin",
                    "source_left",
                    "source_right",
                    "source_left_mapped_face",
                    "source_right_mapped_face",
                    "source_next_halfedge",
                    "source_prev_halfedge",
                    "output_edge_id",
                    "output_origin",
                    "output_left",
                    "output_right",
                    "output_next_edge_id",
                    "output_prev_edge_id",
                    "matching_statuses",
                }
                assert copied_source_edge["status"] in {
                    "mapped-contour",
                    "copied",
                    "missing-output-vertices",
                    "not-prepared-source-edge",
                }
                matched_source_edge = copied_source_edge["matched_source_edge"]
                assert matched_source_edge is None or (
                    isinstance(matched_source_edge, list)
                    and len(matched_source_edge) == 2
                    and all(isinstance(vertex, int) for vertex in matched_source_edge)
                )
                assert copied_source_edge["source_halfedge"] is None or isinstance(
                    copied_source_edge["source_halfedge"], int
                )
                assert copied_source_edge["output_edge_id"] is None or isinstance(
                    copied_source_edge["output_edge_id"], int
                )
                for edge_record_key in (
                    "source_origin",
                    "source_left",
                    "source_right",
                    "source_left_mapped_face",
                    "source_right_mapped_face",
                    "source_next_halfedge",
                    "source_prev_halfedge",
                    "output_origin",
                    "output_left",
                    "output_right",
                    "output_next_edge_id",
                    "output_prev_edge_id",
                ):
                    assert copied_source_edge[edge_record_key] is None or isinstance(
                        copied_source_edge[edge_record_key], int
                    )
                assert isinstance(copied_source_edge["matching_statuses"], int)
        for failure in candidate_diagnostics["failures"]:
            assert isinstance(failure["previous_candidate_source"], str)
            assert isinstance(failure["next_candidate_source"], str)
            assert failure["previous_candidate_key"] is None or isinstance(
                failure["previous_candidate_key"], int
            )
            assert failure["next_candidate_key"] is None or isinstance(
                failure["next_candidate_key"], int
            )
            for source_edge_key in (
                "previous_candidate_source_edge",
                "next_candidate_source_edge",
            ):
                source_edge = failure[source_edge_key]
                assert source_edge is None or (
                    isinstance(source_edge, list)
                    and len(source_edge) == 2
                    and all(isinstance(vertex, int) for vertex in source_edge)
                )
            assert isinstance(failure["previous_next_edge_id"], int)
            assert isinstance(failure["next_prev_edge_id"], int)
            for linked_edge_key in ("previous_next_edge", "next_prev_edge"):
                linked_edge = failure[linked_edge_key]
                assert set(linked_edge) == {"edge_id", "origin", "left", "right"}
                assert isinstance(linked_edge["edge_id"], int)
                assert linked_edge["origin"] is None or isinstance(linked_edge["origin"], int)
                assert linked_edge["left"] is None or isinstance(linked_edge["left"], int)
                assert linked_edge["right"] is None or isinstance(linked_edge["right"], int)
            for snapshot_key in ("previous_target_snapshot", "next_target_snapshot"):
                snapshot = failure[snapshot_key]
                if snapshot is None:
                    continue
                assert set(snapshot) == {
                    "edge_id",
                    "origin",
                    "left",
                    "right",
                    "next_edge_id",
                    "prev_edge_id",
                }
                assert isinstance(snapshot["edge_id"], int)
                assert snapshot["origin"] is None or isinstance(snapshot["origin"], int)
                assert snapshot["left"] is None or isinstance(snapshot["left"], int)
                assert snapshot["right"] is None or isinstance(snapshot["right"], int)
                assert isinstance(snapshot["next_edge_id"], int)
                assert isinstance(snapshot["prev_edge_id"], int)
            assert isinstance(failure["captured_open_target_reopened_previous"], bool)
            assert isinstance(failure["captured_open_target_reopened_next"], bool)
            assert failure["captured_open_target_retry_error"] is None or isinstance(
                failure["captured_open_target_retry_error"], str
            )
            for ring_key in ("previous_left_ring", "next_right_ring"):
                ring = failure[ring_key]
                assert set(ring) >= {"edge_ids", "origins", "left_faces", "error"}
                assert (
                    len(ring["edge_ids"])
                    == len(ring["origins"])
                    == len(ring["left_faces"])
                )
                assert ring["error"] is None or isinstance(ring["error"], str)


def _assert_mapped_source_record_replay_details(
    details: list[dict[str, Any]], *, expected_attempts: int, expected_applied: int
) -> None:
    assert len(details) == expected_attempts
    assert sum(1 for detail in details if detail["applied"]) == expected_applied
    for detail in details:
        assert set(detail) == {
            "target_edge_id",
            "target_was_near_stitch_target",
            "target_origin_before",
            "target_left_before",
            "target_right_before",
            "target_origin_after",
            "target_left_after",
            "target_right_after",
            "record_next_edge_id",
            "record_left",
            "record_sym_prev_edge_id",
            "applied",
            "skipped_reason",
        }
        assert isinstance(detail["target_edge_id"], int)
        assert isinstance(detail["target_was_near_stitch_target"], bool)
        for nullable_int_key in (
            "target_origin_before",
            "target_left_before",
            "target_right_before",
            "target_origin_after",
            "target_left_after",
            "target_right_after",
            "record_left",
        ):
            assert detail[nullable_int_key] is None or isinstance(
                detail[nullable_int_key], int
            )
        assert isinstance(detail["record_next_edge_id"], int)
        assert isinstance(detail["record_sym_prev_edge_id"], int)
        assert isinstance(detail["applied"], bool)
        assert detail["skipped_reason"] is None or isinstance(
            detail["skipped_reason"], str
        )
        if detail["applied"]:
            assert detail["skipped_reason"] is None
        else:
            assert detail["skipped_reason"]


def _assert_copied_prev_next_edge_update_details(
    details: list[dict[str, Any]], *, expected_attempts: int, expected_applied: int
) -> None:
    assert len(details) == expected_attempts
    assert sum(1 for detail in details if detail["applied"]) == expected_applied
    for detail in details:
        assert set(detail) == {
            "source_contour_edge_id",
            "target_contour_edge_id",
            "walked_source_edge_id",
            "update_kind",
            "previous_edge_id",
            "next_edge_id",
            "previous_origin",
            "next_origin",
            "previous_left",
            "next_right",
            "applied",
            "skipped_reason",
        }
        assert isinstance(detail["previous_edge_id"], int)
        assert isinstance(detail["next_edge_id"], int)
        for nullable_int_key in (
            "source_contour_edge_id",
            "target_contour_edge_id",
            "walked_source_edge_id",
            "previous_origin",
            "next_origin",
            "previous_left",
            "next_right",
        ):
            assert detail[nullable_int_key] is None or isinstance(
                detail[nullable_int_key], int
            )
        assert isinstance(detail["applied"], bool)
        assert detail["skipped_reason"] is None or isinstance(
            detail["skipped_reason"], str
        )
        assert detail["update_kind"] in {"next", "previous"}
        if detail["applied"]:
            assert detail["skipped_reason"] is None
        else:
            assert detail["skipped_reason"]


def _assert_copied_face_record_details(
    details: list[dict[str, Any]], *, expected_records: int
) -> None:
    assert len(details) == expected_records
    for detail in details:
        assert set(detail) == {
            "output_face",
            "cut_face",
            "source_face",
            "selected_edge_id",
            "selected_source_edge_id",
            "selected_source_edge_vertices",
            "selected_by_valid_left_ring",
            "selected_left_ring_valid",
            "selected_left_ring_error",
            "candidates",
        }
        for int_key in (
            "output_face",
            "cut_face",
            "selected_edge_id",
            "selected_source_edge_id",
        ):
            assert isinstance(detail[int_key], int)
        assert detail["source_face"] is None or isinstance(detail["source_face"], int)
        selected_vertices = detail["selected_source_edge_vertices"]
        assert selected_vertices is None or (
            isinstance(selected_vertices, list)
            and len(selected_vertices) == 2
            and all(isinstance(vertex, int) for vertex in selected_vertices)
        )
        assert isinstance(detail["selected_by_valid_left_ring"], bool)
        assert isinstance(detail["selected_left_ring_valid"], bool)
        assert detail["selected_left_ring_error"] is None or isinstance(
            detail["selected_left_ring_error"], str
        )
        if detail["selected_left_ring_valid"]:
            assert detail["selected_left_ring_error"] is None
        else:
            assert detail["selected_left_ring_error"]
        candidates = detail["candidates"]
        assert candidates
        for candidate in candidates:
            assert set(candidate) == {
                "source_edge_id",
                "source_edge_vertices",
                "source_edge_left",
                "source_edge_right",
                "source_next_edge_id",
                "source_prev_edge_id",
                "mapped_edge_id",
                "face_edge_id",
                "face_edge_origin",
                "face_edge_destination",
                "face_edge_left",
                "face_edge_right",
                "face_edge_next_edge_id",
                "face_edge_prev_edge_id",
                "face_edge_sym_next_edge_id",
                "face_edge_sym_prev_edge_id",
                "face_edge_left_ring_next_edge_id",
                "left_ring_valid",
                "left_ring_error",
            }
            assert isinstance(candidate["source_edge_id"], int)
            vertices = candidate["source_edge_vertices"]
            assert vertices is None or (
                isinstance(vertices, list)
                and len(vertices) == 2
                and all(isinstance(vertex, int) for vertex in vertices)
            )
            for nullable_int_key in (
                "mapped_edge_id",
                "face_edge_id",
                "face_edge_origin",
                "face_edge_destination",
                "face_edge_left",
                "face_edge_right",
                "face_edge_next_edge_id",
                "face_edge_prev_edge_id",
                "face_edge_sym_next_edge_id",
                "face_edge_sym_prev_edge_id",
                "face_edge_left_ring_next_edge_id",
                "source_edge_left",
                "source_edge_right",
            ):
                assert candidate[nullable_int_key] is None or isinstance(
                    candidate[nullable_int_key], int
                )
            assert isinstance(candidate["source_next_edge_id"], int)
            assert isinstance(candidate["source_prev_edge_id"], int)
            assert isinstance(candidate["left_ring_valid"], bool)
            assert candidate["left_ring_error"] is None or isinstance(
                candidate["left_ring_error"], str
            )


def _assert_record_rewrite_target_details(
    details: list[dict[str, Any]], *, expected_applied: int
) -> None:
    assert len(details) == expected_applied
    for detail in details:
        assert set(detail) == {
            "stitch_pair_index",
            "target_edge_id",
            "target_was_near_stitch_target",
            "target_origin_before",
            "target_left_before",
            "target_right_before",
            "target_next_edge_id_before",
            "target_prev_edge_id_before",
            "target_origin_after",
            "target_left_after",
            "target_right_after",
            "target_next_edge_id_after",
            "target_prev_edge_id_after",
            "record_next_edge_id",
            "record_left",
            "record_sym_prev_edge_id",
        }
        assert isinstance(detail["stitch_pair_index"], int)
        assert isinstance(detail["target_edge_id"], int)
        assert isinstance(detail["target_was_near_stitch_target"], bool)
        for nullable_int_key in (
            "target_origin_before",
            "target_left_before",
            "target_right_before",
            "target_origin_after",
            "target_left_after",
            "target_right_after",
            "record_left",
        ):
            assert detail[nullable_int_key] is None or isinstance(
                detail[nullable_int_key], int
            )
        for edge_id_key in (
            "target_next_edge_id_before",
            "target_prev_edge_id_before",
            "target_next_edge_id_after",
            "target_prev_edge_id_after",
            "record_next_edge_id",
            "record_sym_prev_edge_id",
        ):
            assert isinstance(detail[edge_id_key], int)


def _near_stitch_missing_source_lookups(
    missing_candidate_details: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    lookups = []
    for detail in missing_candidate_details:
        candidate_diagnostics = detail["candidate_diagnostics"]
        lookup = (
            candidate_diagnostics["previous_source_lookup"]
            or candidate_diagnostics["next_source_lookup"]
        )
        if lookup is not None:
            lookups.append(lookup)
    return lookups


def _assert_near_stitch_missing_edges_are_not_prepared(
    missing_candidate_details: list[dict[str, Any]],
) -> None:
    source_lookups = _near_stitch_missing_source_lookups(missing_candidate_details)
    assert len(source_lookups) == len(missing_candidate_details)
    assert all(
        lookup["copied_source_edge"] is not None
        and lookup["copied_source_edge"]["status"] == "not-prepared-source-edge"
        and lookup["copied_source_edge"]["matching_statuses"] == 0
        and lookup["copied_source_edge"]["matched_source_edge"] is None
        and lookup["copied_source_edge"]["source_halfedge"] is None
        and lookup["copied_source_edge"]["source_origin"] is None
        and lookup["copied_source_edge"]["source_left"] is None
        and lookup["copied_source_edge"]["source_right"] is None
        and lookup["copied_source_edge"]["source_left_mapped_face"] is None
        and lookup["copied_source_edge"]["source_right_mapped_face"] is None
        and lookup["copied_source_edge"]["source_next_halfedge"] is None
        and lookup["copied_source_edge"]["source_prev_halfedge"] is None
        and lookup["copied_source_edge"]["output_edge_id"] is None
        and lookup["copied_source_edge"]["output_origin"] is None
        and lookup["copied_source_edge"]["output_left"] is None
        and lookup["copied_source_edge"]["output_right"] is None
        and lookup["copied_source_edge"]["output_next_edge_id"] is None
        and lookup["copied_source_edge"]["output_prev_edge_id"] is None
        for lookup in source_lookups
    )


def _mesh_from_rust_payload(payload: dict[str, Any], *, source: str) -> MeshDocument:
    vertices = np.asarray(payload["vertices"], dtype=np.float64).reshape((-1, 3))
    faces = np.asarray(payload["faces"], dtype=np.int64).reshape((-1, 3))
    return MeshDocument(vertices, faces, metadata={"source": source})


def _assert_nested_metric_close(actual: Any, expected: Any, *, abs_tol: float = 1e-6) -> None:
    if isinstance(expected, dict):
        assert isinstance(actual, dict)
        assert set(actual) >= set(expected)
        for key, value in expected.items():
            _assert_nested_metric_close(actual[key], value, abs_tol=abs_tol)
        return
    if isinstance(expected, list):
        assert np.allclose(actual, expected, atol=abs_tol)
        return
    if isinstance(expected, float):
        assert np.isclose(float(actual), expected, atol=abs_tol)
        return
    assert actual == expected


def test_sdk_volume_matches_trimesh_for_cube() -> None:
    mesh = cube(size=2.0)
    trimesh_mesh = to_trimesh(mesh, process=False)

    assert np.isclose(compute_mesh_stats(mesh).volume_mm3, abs(float(trimesh_mesh.volume)))


def test_sdk_boundary_closed_flag_matches_trimesh_watertight() -> None:
    closed = cube(size=1.0)
    open_mesh = open_cube(size=1.0)

    assert (compute_mesh_stats(closed).boundary_edge_count == 0) == bool(to_trimesh(closed).is_watertight)
    assert (compute_mesh_stats(open_mesh).boundary_edge_count == 0) == bool(to_trimesh(open_mesh).is_watertight)


def test_trimesh_round_trip_preserves_arrays() -> None:
    mesh = cube(size=1.5)
    round_trip = from_trimesh(to_trimesh(mesh, process=False))

    assert np.allclose(round_trip.vertices, mesh.vertices)
    assert np.array_equal(round_trip.faces, mesh.faces)


def test_meshlib_oracle_health_metrics_match_sdk_boundary_state(tmp_path) -> None:
    pytest.importorskip("meshlib")

    closed = cube(size=1.0)
    open_mesh = open_cube(size=1.0)
    closed_path = save_mesh(closed, tmp_path / "closed_cube.stl")
    open_path = save_mesh(open_mesh, tmp_path / "open_cube.stl")

    closed_oracle = health_metrics(closed_path)
    open_oracle = health_metrics(open_path)
    closed_health = compute_mesh_health(closed)
    open_health = compute_mesh_health(open_mesh)

    assert closed_oracle["is_closed"] == closed_health.is_closed
    assert open_oracle["is_closed"] == open_health.is_closed
    assert closed_oracle["holes_count"] == closed_health.holes_count
    assert open_oracle["holes_count"] == open_health.holes_count
    assert closed_oracle["self_intersections"] == closed_health.self_intersections
    assert open_oracle["self_intersections"] == open_health.self_intersections


def test_live_meshlib_thickness_matches_stored_reference_summaries(tmp_path) -> None:
    pytest.importorskip("meshlib")
    golden = load_golden("geometry_reference_v1.json")["fixtures"]

    for fixture_name, maker in MESH_LIB_GOLDEN_FIXTURES.items():
        mesh_path = save_mesh(maker(), tmp_path / f"{fixture_name}.stl")
        summary = meshlib_thickness_summary(mesh_path, threshold_mm=0.6)

        assert_metric_dict_close(summary, golden[fixture_name]["meshlib_thickness"], abs_tol=1e-5)


def test_meshlib_reference_thickness_npz_matches_current_artifact_contract(tmp_path) -> None:
    pytest.importorskip("meshlib")
    mesh = cube(size=2.0)
    mesh_path = save_mesh(mesh, tmp_path / "cube.stl")
    artifact_path = meshlib_save_thickness_npz(mesh_path, tmp_path / "meshlib_thickness.npz", threshold_mm=0.6)
    values, threshold = load_thickness_npz(artifact_path)
    golden = load_golden("geometry_reference_v1.json")["fixtures"]["cube_2mm"]["meshlib_thickness"]

    assert values.shape == (mesh.vertex_count,)
    assert threshold == pytest.approx(0.6)
    assert_metric_dict_close(meshlib_thickness_summary(mesh_path), golden, abs_tol=1e-5)


def test_meshlib_offset_reference_matches_sdk_voxel_offset_envelope(tmp_path) -> None:
    pytest.importorskip("meshlib")
    source = cube(size=2.0)
    source_path = save_mesh(source, tmp_path / "source.stl")

    meshlib_path = meshlib_offset_mesh(
        source_path,
        tmp_path / "meshlib_offset.stl",
        offset_mm=0.5,
        voxel_size_mm=0.25,
    )
    meshlib_result = load_mesh(meshlib_path)
    sdk_result = voxel_offset_mesh(source, offset_mm=0.5, voxel_size_mm=0.5, refine=True)
    meshlib_stats = compute_mesh_stats(meshlib_result)
    sdk_stats = compute_mesh_stats(sdk_result)

    assert compute_mesh_health(meshlib_result).is_closed
    assert compute_mesh_health(sdk_result).is_closed
    assert meshlib_stats.volume_mm3 > compute_mesh_stats(source).volume_mm3
    assert sdk_stats.volume_mm3 > compute_mesh_stats(source).volume_mm3
    assert np.allclose(sdk_stats.bbox_size, meshlib_stats.bbox_size, atol=0.1)
    assert np.isclose(sdk_stats.volume_mm3, meshlib_stats.volume_mm3, rtol=0.15, atol=0.75)


def test_meshlib_boolean_reference_matches_sdk_voxel_boolean_envelope(tmp_path) -> None:
    pytest.importorskip("meshlib")
    a = cube(size=2.0)
    b = a.copy(vertices=a.vertices + np.array([1.0, 0.0, 0.0]))
    a_path = save_mesh(a, tmp_path / "a.stl")
    b_path = save_mesh(b, tmp_path / "b.stl")
    source_volume = compute_mesh_stats(a).volume_mm3

    for operation in ("union", "intersection", "difference"):
        meshlib_path = meshlib_boolean_mesh(a_path, b_path, tmp_path / f"meshlib_{operation}.stl", operation=operation)
        meshlib_result = load_mesh(meshlib_path)
        sdk_result = voxel_boolean_mesh(a, b, operation=operation, voxel_size_mm=0.5, refine=True)
        meshlib_stats = compute_mesh_stats(meshlib_result)
        sdk_stats = compute_mesh_stats(sdk_result)

        assert compute_mesh_health(meshlib_result).is_closed
        assert compute_mesh_health(sdk_result).is_closed
        assert sdk_stats.volume_mm3 > 0.0
        assert np.isclose(sdk_stats.volume_mm3, meshlib_stats.volume_mm3, rtol=0.35, atol=0.75)

        if operation == "union":
            assert sdk_stats.volume_mm3 > source_volume
        else:
            assert sdk_stats.volume_mm3 < source_volume


def test_exact_boolean_sdk_facade_returns_meshlib_parity_ready_cube_overlap(tmp_path) -> None:
    pytest.importorskip("meshlib")
    if not _rust_common.available():
        pytest.skip("Rust extension is not installed")

    sdk = GeometrySDK()
    a = cube(size=2.0)
    b = a.copy(vertices=a.vertices + np.array([1.0, 0.0, 0.0]))
    a_path = save_mesh(a, tmp_path / "a.stl")
    b_path = save_mesh(b, tmp_path / "b.stl")

    for operation in ("union", "intersection", "difference"):
        meshlib_path = meshlib_boolean_mesh(
            a_path,
            b_path,
            tmp_path / f"meshlib_sdk_exact_{operation}.stl",
            operation=operation,
        )

        result = sdk.exact_boolean_mesh(
            a,
            b,
            operation=operation,
            leaf_size=8,
            epsilon=1e-9,
        )

        assert result.operation == operation
        assert result.mesh.unit == a.unit
        assert result.mesh.metadata["source"] == "rust_exact_boolean"
        assert result.mesh.metadata["operation"] == operation
        assert result.diagnostics["parity_ready"]
        assert result.diagnostics["stitch_compatible"]
        assert result.diagnostics["meshlib_topology_open_stitch_paths"] == 0
        assert compute_mesh_health(result.mesh).is_closed

        meshlib_stats = compute_mesh_stats(load_mesh(meshlib_path))
        sdk_stats = compute_mesh_stats(result.mesh)
        if operation == "difference":
            assert sdk_stats.vertex_count == meshlib_stats.vertex_count
            assert sdk_stats.face_count == meshlib_stats.face_count
        assert np.isclose(sdk_stats.volume_mm3, meshlib_stats.volume_mm3, atol=1e-6)
        assert np.allclose(sdk_stats.bbox_size, meshlib_stats.bbox_size, atol=1e-6)


def test_exact_boolean_sdk_facade_matches_meshlib_difference_ba_alias(tmp_path) -> None:
    pytest.importorskip("meshlib")
    if not _rust_common.available():
        pytest.skip("Rust extension is not installed")

    from meshlib import mrmeshpy as mr

    sdk = GeometrySDK()
    a = cube(size=2.0)
    b = a.copy(vertices=a.vertices + np.array([1.0, 0.0, 0.0]))
    a_path = save_mesh(a, tmp_path / "a.stl")
    b_path = save_mesh(b, tmp_path / "b.stl")
    meshlib_path = tmp_path / "meshlib_sdk_exact_difference_ba.stl"

    meshlib_result = mr.boolean(
        mr.loadMesh(str(a_path)),
        mr.loadMesh(str(b_path)),
        mr.BooleanOperation.DifferenceBA,
    )
    assert meshlib_result.valid(), meshlib_result.errorString
    mr.saveMesh(meshlib_result.mesh, str(meshlib_path))

    result = sdk.exact_boolean_mesh(
        a,
        b,
        operation="difference_ba",
        leaf_size=8,
        epsilon=1e-9,
    )

    assert result.operation == "difference_ba"
    assert result.mesh.metadata["source"] == "rust_exact_boolean"
    assert result.mesh.metadata["operation"] == "difference_ba"
    assert result.diagnostics["parity_ready"]
    assert result.diagnostics["stitch_compatible"]
    assert result.diagnostics["meshlib_topology_open_stitch_paths"] == 0
    assert compute_mesh_health(result.mesh).is_closed

    meshlib_stats = compute_mesh_stats(load_mesh(meshlib_path))
    sdk_stats = compute_mesh_stats(result.mesh)
    assert sdk_stats.vertex_count == meshlib_stats.vertex_count
    assert sdk_stats.face_count == meshlib_stats.face_count
    assert np.isclose(sdk_stats.volume_mm3, meshlib_stats.volume_mm3, atol=1e-6)
    assert np.allclose(sdk_stats.bbox_size, meshlib_stats.bbox_size, atol=1e-6)


def test_rust_exact_boolean_binding_matches_meshlib_cube_overlap(tmp_path) -> None:
    pytest.importorskip("meshlib")
    if not _rust_common.available():
        pytest.skip("Rust extension is not installed")

    a = cube(size=2.0)
    b = a.copy(vertices=a.vertices + np.array([1.0, 0.0, 0.0]))
    a_path = save_mesh(a, tmp_path / "a.stl")
    b_path = save_mesh(b, tmp_path / "b.stl")

    for operation in ("union", "intersection"):
        meshlib_path = meshlib_boolean_mesh(
            a_path,
            b_path,
            tmp_path / f"meshlib_exact_{operation}.stl",
            operation=operation,
        )
        payload = _rust_common._rs.exact_boolean_mesh(
            a.vertices,
            a.faces,
            b.vertices,
            b.faces,
            operation,
            leaf_size=8,
            epsilon=1e-9,
        )
        rust_mesh = _mesh_from_rust_payload(payload, source=f"rust_exact_{operation}")
        meshlib_stats = compute_mesh_stats(load_mesh(meshlib_path))
        rust_stats = compute_mesh_stats(rust_mesh)
        diagnostics = payload["diagnostics"]

        assert diagnostics["parity_ready"]
        assert diagnostics["stitch_compatible"]
        assert diagnostics["meshlib_topology_open_stitch_paths"] == 0
        if operation == "union":
            assert diagnostics["meshlib_topology_copied_edge_prepared_faces"] == 20
            assert diagnostics["meshlib_topology_copied_edge_prepared_vertices"] == 20
            assert diagnostics["meshlib_topology_virtual_copied_vertices"] == 8
            assert diagnostics["meshlib_topology_copied_edge_prepared_edges"] == 38
            assert diagnostics["meshlib_topology_copied_edge_mapped_edges"] == 16
            assert diagnostics["meshlib_topology_copied_edges"] == 22
            assert diagnostics["meshlib_topology_copied_edges_mapped_to_existing_output"] == 17
            assert diagnostics["meshlib_topology_copied_edges_mapped_to_output"] == 22
            assert diagnostics["meshlib_topology_copied_edges_missing_output_vertices"] == 0
            assert diagnostics["meshlib_topology_copied_edge_translation_ready"]
        else:
            assert diagnostics["meshlib_topology_copied_edge_prepared_faces"] == 16
            assert diagnostics["meshlib_topology_copied_edge_prepared_vertices"] == 16
            assert diagnostics["meshlib_topology_virtual_copied_vertices"] == 0
            assert diagnostics["meshlib_topology_copied_edge_prepared_edges"] == 32
            assert diagnostics["meshlib_topology_copied_edge_mapped_edges"] == 16
            assert diagnostics["meshlib_topology_copied_edges"] == 16
            assert diagnostics["meshlib_topology_copied_edges_mapped_to_existing_output"] == 16
            assert diagnostics["meshlib_topology_copied_edges_mapped_to_output"] == 16
            assert diagnostics["meshlib_topology_copied_edges_missing_output_vertices"] == 0
            assert diagnostics["meshlib_topology_copied_edge_translation_ready"]
        prepared_base_rewrite = diagnostics["meshlib_topology_prepared_base_record_rewrite"]
        expected_prepared_base_rewrite = (
            (20, 20, 0, 20, 16, 0, 0, 0, 40, 0)
            if operation == "union"
            else (16, 16, 0, 16, 16, 0, 0, 0, 32, 0)
        )
        assert (
            prepared_base_rewrite["prepared_faces"],
            prepared_base_rewrite["prepared_vertices"],
            prepared_base_rewrite["virtual_vertices"],
            prepared_base_rewrite["prepared_face_sources"],
            prepared_base_rewrite["applied_commands"],
            prepared_base_rewrite["failed_commands"],
            prepared_base_rewrite["near_stitch_updates_applied"],
            prepared_base_rewrite["near_stitch_updates_failed"],
            prepared_base_rewrite["exported_faces"],
            prepared_base_rewrite["export_failed_faces"],
        ) == expected_prepared_base_rewrite
        assert prepared_base_rewrite["ready_for_export"]
        record_rewrite_details = prepared_base_rewrite["record_rewrite_target_details"]
        _assert_record_rewrite_target_details(
            record_rewrite_details,
            expected_applied=prepared_base_rewrite["applied_commands"],
        )
        assert len(record_rewrite_details) == 16
        assert all(
            detail["target_was_near_stitch_target"]
            for detail in record_rewrite_details
        )
        expected_target_left_closures = 1 if operation == "union" else 5
        assert (
            prepared_base_rewrite[
                "record_rewrite_near_stitch_target_left_closures"
            ]
            == expected_target_left_closures
        )
        assert (
            prepared_base_rewrite[
                "record_rewrite_near_stitch_target_right_closures"
            ]
            == 0
        )
        expected_mapped_replays = 32
        assert prepared_base_rewrite["mapped_source_record_replays"] == expected_mapped_replays
        assert (
            prepared_base_rewrite[
                "mapped_source_record_replays_on_near_stitch_targets"
            ]
            == 0
        )
        assert (
            prepared_base_rewrite["mapped_source_record_replay_attempts"]
            == expected_mapped_replays
        )
        assert (
            prepared_base_rewrite[
                "mapped_source_record_replay_attempts_on_near_stitch_targets"
            ]
            == 0
        )
        assert prepared_base_rewrite["skipped_mapped_source_record_replays"] == 0
        _assert_mapped_source_record_replay_details(
            prepared_base_rewrite["mapped_source_record_replay_details"],
            expected_attempts=expected_mapped_replays,
            expected_applied=expected_mapped_replays,
        )
        copied_prev_next_attempts = prepared_base_rewrite[
            "copied_prev_next_edge_update_attempts"
        ]
        copied_prev_next_applied = prepared_base_rewrite[
            "copied_prev_next_edge_updates_applied"
        ]
        copied_prev_next_skipped = prepared_base_rewrite[
            "copied_prev_next_edge_updates_skipped"
        ]
        assert (
            copied_prev_next_applied + copied_prev_next_skipped
            == copied_prev_next_attempts
        )
        _assert_copied_prev_next_edge_update_details(
            prepared_base_rewrite["copied_prev_next_edge_update_details"],
            expected_attempts=copied_prev_next_attempts,
            expected_applied=copied_prev_next_applied,
        )
        _assert_copied_face_record_details(
            prepared_base_rewrite["copied_face_record_details"],
            expected_records=prepared_base_rewrite["translated_copied_face_records"],
        )
        expected_prepared_base_buckets = (
            (0, 0, 0, 0, 76, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            if operation == "union"
            else (0, 0, 0, 0, 64, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
        )
        assert (
            prepared_base_rewrite["record_failed_missing_targets"],
            prepared_base_rewrite["record_failed_closed_targets"],
            prepared_base_rewrite["record_failed_missing_sources"],
            prepared_base_rewrite["record_failed_other_commands"],
            prepared_base_rewrite["translated_copied_edge_records"],
            prepared_base_rewrite["translated_copied_face_records"],
            prepared_base_rewrite["failed_copied_edge_records"],
            prepared_base_rewrite["refreshed_face_records"],
            prepared_base_rewrite["near_stitch_failed_start"],
            prepared_base_rewrite["near_stitch_failed_end"],
            prepared_base_rewrite["near_stitch_missing_previous_edges"],
            prepared_base_rewrite["near_stitch_missing_next_edges"],
            prepared_base_rewrite["near_stitch_origin_mismatches"],
            prepared_base_rewrite["near_stitch_previous_left_faces"],
            prepared_base_rewrite["near_stitch_next_right_faces"],
            prepared_base_rewrite["near_stitch_failed_other"],
            prepared_base_rewrite["export_non_triangular_faces"],
            prepared_base_rewrite["export_left_ring_not_closed_faces"],
            prepared_base_rewrite["export_missing_origin_faces"],
            prepared_base_rewrite["export_face_record_left_mismatch_faces"],
            prepared_base_rewrite["export_face_left_ring_mismatch_faces"],
            prepared_base_rewrite["export_other_failed_faces"],
        ) == expected_prepared_base_buckets
        assert (
            len(prepared_base_rewrite["export_failed_face_indices"])
            == prepared_base_rewrite["export_failed_faces"]
        )
        export_failed_details = prepared_base_rewrite["export_failed_face_details"]
        assert len(export_failed_details) == prepared_base_rewrite["export_failed_faces"]
        assert [
            detail["face_index"] for detail in export_failed_details
        ] == prepared_base_rewrite["export_failed_face_indices"]
        assert all(detail["error"] for detail in export_failed_details)
        assert all("face_cut_face" in detail for detail in export_failed_details)
        assert all("face_source_face" in detail for detail in export_failed_details)
        assert all(
            detail["face_cut_face"] is None or isinstance(detail["face_cut_face"], int)
            for detail in export_failed_details
        )
        assert all(
            detail["face_source_face"] is None
            or isinstance(detail["face_source_face"], int)
            for detail in export_failed_details
        )
        assert all(
            len(detail["left_ring_edge_ids"])
            == len(detail["left_ring_record_next_edge_ids"])
            == len(detail["left_ring_record_prev_edge_ids"])
            == len(detail["left_ring_origins"])
            == len(detail["left_ring_destinations"])
            == len(detail["left_ring_left_faces"])
            == len(detail["left_ring_right_faces"])
            == len(detail["left_ring_next_edge_ids"])
            for detail in export_failed_details
        )
        assert all("same_left_face_edge_ids" in detail for detail in export_failed_details)
        assert all(
            len(detail["same_left_face_edge_ids"])
            == len(detail["same_left_face_record_next_edge_ids"])
            == len(detail["same_left_face_record_prev_edge_ids"])
            == len(detail["same_left_face_next_edge_ids"])
            == len(detail["same_left_face_origins"])
            == len(detail["same_left_face_destinations"])
            == len(detail["same_left_face_right_faces"])
            for detail in export_failed_details
        )
        assert all("left_ring_returned_to_start" in detail for detail in export_failed_details)
        assert all("left_ring_repeated_edge_id" in detail for detail in export_failed_details)
        failed_details = prepared_base_rewrite["near_stitch_failed_details"]
        assert len(failed_details) == prepared_base_rewrite["near_stitch_updates_failed"]
        assert {detail["endpoint"] for detail in failed_details} <= {"start", "end"}
        assert all(detail["error"] for detail in failed_details)
        guarded_details = [
            detail
            for detail in failed_details
            if detail["candidate_diagnostics"] is not None
            and detail["candidate_diagnostics"]["failures"]
        ]
        assert not guarded_details
        assert all(
            detail["candidate_diagnostics"]["previous_candidates"] >= 1
            and detail["candidate_diagnostics"]["next_candidates"] >= 1
            for detail in guarded_details
        )
        missing_candidate_details = [
            detail
            for detail in failed_details
            if detail["candidate_diagnostics"] is not None
            and not detail["candidate_diagnostics"]["failures"]
        ]
        assert all(
            detail["candidate_diagnostics"]["previous_candidates"] == 0
            or detail["candidate_diagnostics"]["next_candidates"] == 0
            for detail in missing_candidate_details
        )
        assert all(
            detail["candidate_diagnostics"]["attempt"] == "vertex-pair-fallback"
            and detail["candidate_diagnostics"]["fallback_from"] is not None
            and detail["candidate_diagnostics"]["fallback_from"]["attempt"]
            == "identity-target-source"
            for detail in missing_candidate_details
        )
        assert all(
            (
                detail["candidate_diagnostics"]["previous_source_lookup"]
                or detail["candidate_diagnostics"]["next_source_lookup"]
            )
            is not None
            for detail in missing_candidate_details
        )
        _assert_near_stitch_missing_edges_are_not_prepared(missing_candidate_details)
        assert len(missing_candidate_details) == (
            prepared_base_rewrite["near_stitch_missing_previous_edges"]
            + prepared_base_rewrite["near_stitch_missing_next_edges"]
        )
        if guarded_details:
            _assert_near_stitch_candidate_ring_details(guarded_details)
        assert diagnostics["meshlib_topology_open_stitch_near_edge_updates"] == 0
        assert diagnostics["meshlib_topology_open_stitch_near_edge_blocked_updates"] == 0
        assert diagnostics["meshlib_topology_open_stitch_near_edge_ready"]
        assert (
            diagnostics["meshlib_topology_near_stitch_update_commands"],
            diagnostics["meshlib_topology_near_stitch_updates_applied"],
            diagnostics["meshlib_topology_near_stitch_updates_failed"],
            diagnostics["meshlib_topology_near_stitch_updates_failed_start"],
            diagnostics["meshlib_topology_near_stitch_updates_failed_end"],
            diagnostics["meshlib_topology_near_stitch_updates_missing_previous_edges"],
            diagnostics["meshlib_topology_near_stitch_updates_missing_next_edges"],
            diagnostics["meshlib_topology_near_stitch_updates_origin_mismatches"],
            diagnostics["meshlib_topology_near_stitch_updates_previous_left_faces"],
            diagnostics["meshlib_topology_near_stitch_updates_next_right_faces"],
            diagnostics["meshlib_topology_near_stitch_updates_failed_other"],
        ) == (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
        top_failed_details = diagnostics["meshlib_topology_near_stitch_failed_details"]
        assert top_failed_details == []
        assert diagnostics["is_closed"]
        assert diagnostics["boundary_edge_count"] == 0
        assert diagnostics["nonmanifold_edge_count"] == 0
        assert np.isclose(rust_stats.volume_mm3, meshlib_stats.volume_mm3, atol=1e-6)
        assert np.isclose(rust_stats.surface_area_mm2, meshlib_stats.surface_area_mm2, atol=1e-6)
        assert np.allclose(rust_stats.bbox_size, meshlib_stats.bbox_size, atol=1e-6)


def test_rust_exact_boolean_binding_tracks_difference_prepared_base_closure(tmp_path) -> None:
    pytest.importorskip("meshlib")
    if not _rust_common.available():
        pytest.skip("Rust extension is not installed")

    a = cube(size=2.0)
    b = a.copy(vertices=a.vertices + np.array([1.0, 0.0, 0.0]))
    a_path = save_mesh(a, tmp_path / "a.stl")
    b_path = save_mesh(b, tmp_path / "b.stl")
    meshlib_path = meshlib_boolean_mesh(
        a_path,
        b_path,
        tmp_path / "meshlib_exact_difference.stl",
        operation="difference",
    )
    payload = _rust_common._rs.exact_boolean_mesh(
        a.vertices,
        a.faces,
        b.vertices,
        b.faces,
        "difference",
        leaf_size=8,
        epsilon=1e-9,
    )
    rust_mesh = _mesh_from_rust_payload(payload, source="rust_exact_difference")
    meshlib_mesh = load_mesh(meshlib_path)
    meshlib_stats = compute_mesh_stats(meshlib_mesh)
    meshlib_health = compute_mesh_health(meshlib_mesh)
    rust_stats = compute_mesh_stats(rust_mesh)
    rust_health = compute_mesh_health(rust_mesh)
    diagnostics = payload["diagnostics"]
    meshlib_in_memory = _meshlib_in_memory_boolean_mapper_summary(
        a,
        b,
        operation="difference",
    )
    meshlib_cutmesh_reference = _meshlib_pre_cut_cutmesh_mapper_summary(
        a,
        b,
        operation="difference",
    )

    assert meshlib_health.is_closed
    assert (
        meshlib_in_memory["face_size"],
        meshlib_in_memory["valid_faces"],
        meshlib_in_memory["vert_size"],
        meshlib_in_memory["valid_verts"],
        meshlib_in_memory["mapped_a_faces"],
        meshlib_in_memory["mapped_b_faces"],
    ) == (44, 44, 24, 24, 28, 16)
    assert meshlib_in_memory["mapped_a_cut_faces"] == [
        2,
        3,
        8,
        9,
        10,
        11,
        14,
        15,
        16,
        20,
        24,
        25,
        26,
        27,
        28,
        29,
        30,
        34,
        37,
        38,
        39,
        40,
        41,
        42,
        46,
        47,
        48,
        49,
    ]
    assert meshlib_in_memory["mapped_b_cut_faces"] == [
        14,
        15,
        16,
        17,
        23,
        24,
        27,
        28,
        29,
        36,
        37,
        38,
        44,
        45,
        46,
        49,
    ]
    assert (
        meshlib_in_memory["cut2origin_a_size"],
        meshlib_in_memory["cut2origin_b_size"],
        meshlib_in_memory["cut2origin_a_valid"],
        meshlib_in_memory["cut2origin_b_valid"],
        meshlib_in_memory["cut2origin_a_unique_origins"],
        meshlib_in_memory["cut2origin_b_unique_origins"],
        meshlib_in_memory["cut2origin_a_duplicate_cut_faces"],
        meshlib_in_memory["cut2origin_b_duplicate_cut_faces"],
    ) == (50, 50, 50, 50, 12, 12, 38, 38)
    assert meshlib_in_memory["cut2origin_a_histogram"] == [
        [0, 8],
        [1, 6],
        [2, 1],
        [3, 1],
        [4, 6],
        [5, 10],
        [6, 8],
        [7, 6],
        [8, 1],
        [9, 1],
        [10, 1],
        [11, 1],
    ]
    assert meshlib_in_memory["cut2origin_b_histogram"] == [
        [0, 1],
        [1, 1],
        [2, 8],
        [3, 8],
        [4, 1],
        [5, 1],
        [6, 1],
        [7, 1],
        [8, 8],
        [9, 4],
        [10, 10],
        [11, 6],
    ]
    expected_prepared_cut2origin_prefix = list(range(12))
    assert (
        meshlib_in_memory["cut2origin_a_values"][:12],
        meshlib_in_memory["cut2origin_b_values"][:12],
    ) == (
        expected_prepared_cut2origin_prefix,
        expected_prepared_cut2origin_prefix,
    )
    assert (
        meshlib_cutmesh_reference["cut2origin_a_values"],
        meshlib_cutmesh_reference["cut2origin_b_values"],
    ) == (
        meshlib_in_memory["cut2origin_a_values"],
        meshlib_in_memory["cut2origin_b_values"],
    )
    assert (
        meshlib_cutmesh_reference["result_cut_paths"],
        meshlib_cutmesh_reference["bad_contour_faces"],
        meshlib_cutmesh_reference["face_sizes"],
        meshlib_cutmesh_reference["pre_cut_contour_lengths"],
        meshlib_cutmesh_reference["pre_cut_contour_closed"],
    ) == (
        [1, 1],
        [0, 0],
        [50, 50],
        [[17], [17]],
        [[True], [True]],
    )
    assert meshlib_cutmesh_reference["valid_cut_faces"] == [
        [
            2,
            3,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
            16,
            17,
            18,
            19,
            20,
            21,
            22,
            23,
            24,
            25,
            26,
            27,
            28,
            29,
            30,
            31,
            32,
            33,
            34,
            35,
            36,
            37,
            38,
            39,
            40,
            41,
            42,
            43,
            44,
            45,
            46,
            47,
            48,
            49,
        ],
        [
            0,
            1,
            4,
            5,
            6,
            7,
            12,
            13,
            14,
            15,
            16,
            17,
            18,
            19,
            20,
            21,
            22,
            23,
            24,
            25,
            26,
            27,
            28,
            29,
            30,
            31,
            32,
            33,
            34,
            35,
            36,
            37,
            38,
            39,
            40,
            41,
            42,
            43,
            44,
            45,
            46,
            47,
            48,
            49,
        ],
    ]
    assert diagnostics["paired_coplanar_candidate_first_meshlib_valid_cut_faces"] == (
        meshlib_cutmesh_reference["valid_cut_faces"][0]
    )
    assert diagnostics["paired_coplanar_candidate_second_meshlib_valid_cut_faces"] == (
        meshlib_cutmesh_reference["valid_cut_faces"][1]
    )
    assert all(
        face in meshlib_cutmesh_reference["valid_cut_faces"][0]
        for face in meshlib_in_memory["mapped_a_cut_faces"]
    )
    assert all(
        face in meshlib_cutmesh_reference["valid_cut_faces"][1]
        for face in meshlib_in_memory["mapped_b_cut_faces"]
    )
    assert meshlib_cutmesh_reference["pre_cut_contour_start_primitive_kinds"] == [
        [[1, 2, 1, 1, 2, 2, 2, 1, 1, 1, 2, 2, 1, 1, 2, 2]],
        [[2, 1, 2, 2, 1, 1, 1, 2, 2, 2, 1, 1, 2, 2, 1, 1]],
    ]
    assert meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"] == [
        [[-1, 7, -1, -1, 5, 5, 5, -1, -1, -1, 0, 0, -1, -1, 6, 6]],
        [[3, -1, 2, 2, -1, -1, -1, 10, 10, 10, -1, -1, 8, 8, -1, -1]],
    ]
    expected_meshlib_result_cut_edges = [
        [36, 38, 40, 42, 44, 46, 48, 50, 52, 54, 56, 58, 60, 62, 64, 66]
    ]
    assert (
        meshlib_cutmesh_reference["cut2origin_a_result_cut_edges"],
        meshlib_cutmesh_reference["cut2origin_b_result_cut_edges"],
    ) == (
        expected_meshlib_result_cut_edges,
        expected_meshlib_result_cut_edges,
    )
    assert meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"] == [
        [7, 7, 4, 5, 5, 5, 5, 4, 1, 0, 0, 0, 1, 6, 6, 6]
    ]
    assert meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"] == [
        [3, 2, 2, 2, 3, 11, 10, 10, 10, 10, 11, 8, 8, 8, 9, 3]
    ]
    assert meshlib_cutmesh_reference["cut2origin_a_result_cut_old_face_runs"] == [
        [[7, 2], [4, 1], [5, 4], [4, 1], [1, 1], [0, 3], [1, 1], [6, 3]]
    ]
    assert meshlib_cutmesh_reference["cut2origin_b_result_cut_old_face_runs"] == [
        [
            [3, 1],
            [2, 3],
            [3, 1],
            [11, 1],
            [10, 4],
            [11, 1],
            [8, 3],
            [9, 1],
            [3, 1],
        ]
    ]
    assert meshlib_in_memory["cut2origin_a_appended_values"] == [
        7,
        7,
        7,
        7,
        7,
        4,
        4,
        4,
        4,
        5,
        5,
        5,
        5,
        5,
        5,
        5,
        5,
        5,
        4,
        1,
        1,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
        6,
        6,
        6,
        6,
        6,
        6,
        6,
    ]
    assert meshlib_in_memory["cut2origin_b_appended_values"] == [
        3,
        3,
        3,
        3,
        3,
        3,
        2,
        2,
        2,
        2,
        2,
        2,
        2,
        3,
        11,
        11,
        11,
        11,
        10,
        10,
        10,
        10,
        10,
        10,
        10,
        10,
        10,
        11,
        8,
        8,
        8,
        8,
        8,
        8,
        8,
        9,
        9,
        9,
    ]
    assert meshlib_in_memory["cut2origin_a_appended_runs"] == [
        [7, 5],
        [4, 4],
        [5, 9],
        [4, 1],
        [1, 4],
        [0, 7],
        [1, 1],
        [6, 7],
    ]
    assert meshlib_in_memory["cut2origin_b_appended_runs"] == [
        [3, 6],
        [2, 7],
        [3, 1],
        [11, 4],
        [10, 9],
        [11, 1],
        [8, 7],
        [9, 3],
    ]
    assert (
        meshlib_cutmesh_reference["cut2origin_a_appended_runs"],
        meshlib_cutmesh_reference["cut2origin_b_appended_runs"],
    ) == (
        meshlib_in_memory["cut2origin_a_appended_runs"],
        meshlib_in_memory["cut2origin_b_appended_runs"],
    )
    assert diagnostics["output_mesh_source"] == "assembly"
    assert diagnostics["parity_ready"]
    assert diagnostics["stitch_compatible"]
    assert diagnostics["is_closed"]
    assert diagnostics["boundary_edge_count"] == 0
    assert diagnostics["nonmanifold_edge_count"] == 0
    assert rust_health.boundary_edge_count == diagnostics["boundary_edge_count"]
    assert rust_health.nonmanifold_edge_count == diagnostics["nonmanifold_edge_count"]
    prepared_base_rewrite = diagnostics["meshlib_topology_prepared_base_record_rewrite"]
    assert (
        prepared_base_rewrite["prepared_faces"],
        prepared_base_rewrite["prepared_vertices"],
        prepared_base_rewrite["virtual_vertices"],
        prepared_base_rewrite["prepared_face_sources"],
        prepared_base_rewrite["applied_commands"],
        prepared_base_rewrite["failed_commands"],
        prepared_base_rewrite["near_stitch_updates_applied"],
        prepared_base_rewrite["near_stitch_updates_failed"],
        prepared_base_rewrite["exported_faces"],
        prepared_base_rewrite["export_failed_faces"],
    ) == (16, 14, 0, 16, 10, 0, 0, 0, 26, 0)
    assert prepared_base_rewrite["ready_for_export"]
    record_rewrite_details = prepared_base_rewrite["record_rewrite_target_details"]
    _assert_record_rewrite_target_details(
        record_rewrite_details,
        expected_applied=prepared_base_rewrite["applied_commands"],
    )
    assert len(record_rewrite_details) == 10
    assert prepared_base_rewrite["record_rewrite_near_stitch_target_left_closures"] == 3
    assert prepared_base_rewrite["record_rewrite_near_stitch_target_right_closures"] == 0
    assert prepared_base_rewrite["mapped_source_record_replays"] == 20
    assert (
        prepared_base_rewrite["mapped_source_record_replays_on_near_stitch_targets"]
        == 0
    )
    assert prepared_base_rewrite["mapped_source_record_replay_attempts"] == 20
    assert (
        prepared_base_rewrite[
            "mapped_source_record_replay_attempts_on_near_stitch_targets"
        ]
        == 0
    )
    assert prepared_base_rewrite["skipped_mapped_source_record_replays"] == 0
    replay_details = prepared_base_rewrite["mapped_source_record_replay_details"]
    _assert_mapped_source_record_replay_details(
        replay_details,
        expected_attempts=20,
        expected_applied=20,
    )
    assert len(replay_details) == 20
    assert (
        prepared_base_rewrite["copied_prev_next_edge_update_attempts"],
        prepared_base_rewrite["copied_prev_next_edge_updates_applied"],
        prepared_base_rewrite["copied_prev_next_edge_updates_skipped"],
    ) == (0, 0, 0)
    _assert_copied_prev_next_edge_update_details(
        prepared_base_rewrite["copied_prev_next_edge_update_details"],
        expected_attempts=0,
        expected_applied=0,
    )
    copied_face_record_details = prepared_base_rewrite["copied_face_record_details"]
    _assert_copied_face_record_details(
        copied_face_record_details,
        expected_records=prepared_base_rewrite["translated_copied_face_records"],
    )
    assert len(copied_face_record_details) == 10
    assert (
        prepared_base_rewrite["record_failed_missing_targets"],
        prepared_base_rewrite["record_failed_closed_targets"],
        prepared_base_rewrite["record_failed_missing_sources"],
        prepared_base_rewrite["record_failed_other_commands"],
        prepared_base_rewrite["translated_copied_edge_records"],
        prepared_base_rewrite["translated_copied_face_records"],
        prepared_base_rewrite["failed_copied_edge_records"],
        prepared_base_rewrite["refreshed_face_records"],
        prepared_base_rewrite["near_stitch_failed_start"],
        prepared_base_rewrite["near_stitch_failed_end"],
        prepared_base_rewrite["near_stitch_missing_previous_edges"],
        prepared_base_rewrite["near_stitch_missing_next_edges"],
        prepared_base_rewrite["near_stitch_origin_mismatches"],
        prepared_base_rewrite["near_stitch_previous_left_faces"],
        prepared_base_rewrite["near_stitch_next_right_faces"],
        prepared_base_rewrite["near_stitch_failed_other"],
        prepared_base_rewrite["export_non_triangular_faces"],
        prepared_base_rewrite["export_left_ring_not_closed_faces"],
        prepared_base_rewrite["export_missing_origin_faces"],
        prepared_base_rewrite["export_face_record_left_mismatch_faces"],
        prepared_base_rewrite["export_face_left_ring_mismatch_faces"],
        prepared_base_rewrite["export_other_failed_faces"],
    ) == (0, 0, 0, 0, 40, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    assert prepared_base_rewrite["exported_mesh_stats"] is not None
    assert prepared_base_rewrite["exported_mesh_health"] is not None
    assert prepared_base_rewrite["packed_mesh_stats"] is not None
    assert prepared_base_rewrite["packed_mesh_health"] is not None
    assert (
        prepared_base_rewrite["exported_mesh_stats"]["vertex_count"],
        prepared_base_rewrite["exported_mesh_stats"]["face_count"],
        prepared_base_rewrite["exported_mesh_stats"]["connected_components"],
        prepared_base_rewrite["exported_mesh_stats"]["boundary_edge_count"],
        prepared_base_rewrite["exported_mesh_health"]["boundary_edge_count"],
        prepared_base_rewrite["exported_mesh_health"]["nonmanifold_edge_count"],
        prepared_base_rewrite["exported_mesh_health"]["is_closed"],
        prepared_base_rewrite["packed_mesh_stats"]["vertex_count"],
        prepared_base_rewrite["packed_mesh_stats"]["face_count"],
        prepared_base_rewrite["packed_mesh_health"]["boundary_edge_count"],
        prepared_base_rewrite["packed_mesh_health"]["nonmanifold_edge_count"],
        prepared_base_rewrite["packed_mesh_health"]["is_closed"],
    ) == (25, 26, 2, 20, 20, 0, False, 25, 26, 20, 0, False)
    assert (
        prepared_base_rewrite["near_stitch_skipped_previous_left_source_edges"],
        prepared_base_rewrite["near_stitch_skipped_next_right_source_edges"],
    ) == (0, 0)
    assert (
        prepared_base_rewrite["near_stitch_previous_left_copied_source_edges"],
        prepared_base_rewrite["near_stitch_next_right_copied_source_edges"],
    ) == (0, 0)
    assert (
        len(prepared_base_rewrite["export_failed_face_indices"])
        == prepared_base_rewrite["export_failed_faces"]
    )
    export_failed_details = prepared_base_rewrite["export_failed_face_details"]
    assert len(export_failed_details) == prepared_base_rewrite["export_failed_faces"]
    assert [
        detail["face_index"] for detail in export_failed_details
    ] == prepared_base_rewrite["export_failed_face_indices"]
    assert all(detail["error"] for detail in export_failed_details)
    assert all("face_cut_face" in detail for detail in export_failed_details)
    assert all("face_source_face" in detail for detail in export_failed_details)
    assert all(
        detail["face_cut_face"] is None or isinstance(detail["face_cut_face"], int)
        for detail in export_failed_details
    )
    assert all(
        detail["face_source_face"] is None or isinstance(detail["face_source_face"], int)
        for detail in export_failed_details
    )
    assert all(
        len(detail["left_ring_edge_ids"])
        == len(detail["left_ring_record_next_edge_ids"])
        == len(detail["left_ring_record_prev_edge_ids"])
        == len(detail["left_ring_origins"])
        == len(detail["left_ring_destinations"])
        == len(detail["left_ring_left_faces"])
        == len(detail["left_ring_right_faces"])
        == len(detail["left_ring_next_edge_ids"])
        for detail in export_failed_details
    )
    assert all("same_left_face_edge_ids" in detail for detail in export_failed_details)
    assert all(
        len(detail["same_left_face_edge_ids"])
        == len(detail["same_left_face_record_next_edge_ids"])
        == len(detail["same_left_face_record_prev_edge_ids"])
        == len(detail["same_left_face_next_edge_ids"])
        == len(detail["same_left_face_origins"])
        == len(detail["same_left_face_destinations"])
        == len(detail["same_left_face_right_faces"])
        for detail in export_failed_details
    )
    assert [detail["left_ring_repeated_edge_id"] for detail in export_failed_details] == []
    assert not any(detail["left_ring_returned_to_start"] for detail in export_failed_details)
    failed_details = prepared_base_rewrite["near_stitch_failed_details"]
    assert failed_details == []
    assert diagnostics["paired_coplanar_candidate_stitch_compatible"]
    assert diagnostics["paired_coplanar_candidate_first_prepare_part_dividable"]
    assert diagnostics["paired_coplanar_candidate_second_prepare_part_dividable"]
    assert diagnostics["paired_coplanar_candidate_first_cut_path_side_components"] == [1, 1]
    assert diagnostics["paired_coplanar_candidate_second_cut_path_side_components"] == [1, 1]
    assert not diagnostics["paired_coplanar_candidate_result_cut_paths_complete"]
    assert diagnostics["paired_coplanar_candidate_prepare_result_cut_paths_complete"]
    assert diagnostics["paired_coplanar_candidate_output_faces"] == 20
    assert np.isclose(
        diagnostics["paired_coplanar_candidate_output_volume"],
        meshlib_stats.volume_mm3,
        atol=1e-6,
    )
    assert np.isclose(
        diagnostics["paired_coplanar_candidate_output_area"],
        16.0,
        atol=1e-6,
    )
    assert not np.isclose(
        diagnostics["paired_coplanar_candidate_output_area"],
        meshlib_stats.surface_area_mm2,
        atol=1e-6,
    )
    assert np.isclose(
        diagnostics["paired_coplanar_candidate_active_volume_delta"],
        0.0,
        atol=1e-6,
    )
    assert diagnostics["paired_coplanar_candidate_preserves_active_volume"]
    assert diagnostics["paired_coplanar_candidate_self_intersections_available"]
    assert diagnostics["paired_coplanar_candidate_self_intersections"] == 0
    assert diagnostics["paired_coplanar_candidate_boundary_edges"] == 0
    assert diagnostics["paired_coplanar_candidate_nonmanifold_edges"] == 0
    assert diagnostics["paired_coplanar_candidate_duplicate_output_faces"] == 0
    paired_prepared_base_rewrite = diagnostics[
        "paired_coplanar_candidate_prepared_base_record_rewrite"
    ]
    assert paired_prepared_base_rewrite is not None
    assert (
        paired_prepared_base_rewrite["prepared_faces"],
        paired_prepared_base_rewrite["prepared_vertices"],
        paired_prepared_base_rewrite["virtual_vertices"],
        paired_prepared_base_rewrite["prepared_face_sources"],
        paired_prepared_base_rewrite["applied_commands"],
        paired_prepared_base_rewrite["failed_commands"],
        paired_prepared_base_rewrite["translated_copied_edge_records"],
        paired_prepared_base_rewrite["translated_copied_face_records"],
        paired_prepared_base_rewrite["mapped_source_record_replays"],
        paired_prepared_base_rewrite["copied_prev_next_edge_update_attempts"],
        paired_prepared_base_rewrite["copied_prev_next_edge_updates_applied"],
        paired_prepared_base_rewrite["copied_prev_next_edge_updates_skipped"],
        paired_prepared_base_rewrite["near_stitch_updates_applied"],
        paired_prepared_base_rewrite["near_stitch_updates_failed"],
        paired_prepared_base_rewrite["exported_faces"],
        paired_prepared_base_rewrite["export_failed_faces"],
        paired_prepared_base_rewrite["ready_for_export"],
    ) == (20, 20, 0, 20, 8, 0, 64, 16, 16, 0, 0, 0, 0, 0, 36, 0, True)
    assert paired_prepared_base_rewrite["exported_face_operands"] == [
        *(["first"] * 20),
        *(["second"] * 16),
    ]
    assert paired_prepared_base_rewrite["exported_face_cut_faces"] == [
        1,
        2,
        6,
        10,
        12,
        13,
        17,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        28,
        29,
        33,
        34,
        35,
        1,
        2,
        3,
        6,
        9,
        11,
        12,
        13,
        16,
        18,
        19,
        20,
        24,
        25,
        26,
        29,
    ]
    assert paired_prepared_base_rewrite["exported_face_source_faces"] == [
        0,
        0,
        1,
        2,
        3,
        3,
        4,
        5,
        5,
        6,
        6,
        6,
        7,
        7,
        7,
        8,
        8,
        9,
        10,
        11,
        0,
        0,
        0,
        1,
        2,
        3,
        3,
        3,
        4,
        5,
        5,
        5,
        8,
        8,
        8,
        9,
    ]
    assert paired_prepared_base_rewrite["exported_face_cut_faces"][:20] == diagnostics[
        "paired_coplanar_candidate_prepare_first_face_indices"
    ]
    assert paired_prepared_base_rewrite["exported_face_cut_faces"][20:] == diagnostics[
        "paired_coplanar_candidate_prepare_second_face_indices"
    ]
    assert (
        paired_prepared_base_rewrite["record_failed_missing_targets"],
        paired_prepared_base_rewrite["record_failed_closed_targets"],
        paired_prepared_base_rewrite["record_failed_missing_sources"],
        paired_prepared_base_rewrite["record_failed_other_commands"],
        paired_prepared_base_rewrite[
            "record_rewrite_near_stitch_target_left_closures"
        ],
        paired_prepared_base_rewrite[
            "record_rewrite_near_stitch_target_right_closures"
        ],
        paired_prepared_base_rewrite[
            "mapped_source_record_replays_on_near_stitch_targets"
        ],
        paired_prepared_base_rewrite["mapped_source_record_replay_attempts"],
        paired_prepared_base_rewrite[
            "mapped_source_record_replay_attempts_on_near_stitch_targets"
        ],
        paired_prepared_base_rewrite["skipped_mapped_source_record_replays"],
        paired_prepared_base_rewrite["failed_copied_edge_records"],
        paired_prepared_base_rewrite["refreshed_face_records"],
        paired_prepared_base_rewrite["near_stitch_failed_start"],
        paired_prepared_base_rewrite["near_stitch_failed_end"],
        paired_prepared_base_rewrite[
            "near_stitch_skipped_previous_left_source_edges"
        ],
        paired_prepared_base_rewrite["near_stitch_skipped_next_right_source_edges"],
        paired_prepared_base_rewrite["near_stitch_missing_previous_edges"],
        paired_prepared_base_rewrite["near_stitch_missing_next_edges"],
        paired_prepared_base_rewrite["near_stitch_origin_mismatches"],
        paired_prepared_base_rewrite["near_stitch_previous_left_faces"],
        paired_prepared_base_rewrite["near_stitch_previous_left_copied_source_edges"],
        paired_prepared_base_rewrite["near_stitch_next_right_faces"],
        paired_prepared_base_rewrite["near_stitch_next_right_copied_source_edges"],
        paired_prepared_base_rewrite["near_stitch_failed_other"],
        paired_prepared_base_rewrite["export_non_triangular_faces"],
        paired_prepared_base_rewrite["export_left_ring_not_closed_faces"],
        paired_prepared_base_rewrite["export_missing_origin_faces"],
        paired_prepared_base_rewrite["export_face_record_left_mismatch_faces"],
        paired_prepared_base_rewrite["export_face_left_ring_mismatch_faces"],
        paired_prepared_base_rewrite["export_other_failed_faces"],
    ) == (
        0,
        0,
        0,
        0,
        4,
        0,
        0,
        16,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    )
    record_rewrite_details = paired_prepared_base_rewrite[
        "record_rewrite_target_details"
    ]
    _assert_record_rewrite_target_details(record_rewrite_details, expected_applied=8)
    assert all(detail["target_was_near_stitch_target"] for detail in record_rewrite_details)
    assert [detail["target_left_before"] for detail in record_rewrite_details] == [
        None
    ] * 8
    assert [detail["target_left_after"] for detail in record_rewrite_details] == [
        None,
        29,
        None,
        25,
        None,
        32,
        None,
        20,
    ]
    _assert_mapped_source_record_replay_details(
        paired_prepared_base_rewrite["mapped_source_record_replay_details"],
        expected_attempts=16,
        expected_applied=16,
    )
    _assert_copied_prev_next_edge_update_details(
        paired_prepared_base_rewrite["copied_prev_next_edge_update_details"],
        expected_attempts=0,
        expected_applied=0,
    )
    _assert_copied_face_record_details(
        paired_prepared_base_rewrite["copied_face_record_details"],
        expected_records=16,
    )
    assert all(
        detail["selected_by_valid_left_ring"]
        for detail in paired_prepared_base_rewrite["copied_face_record_details"]
    )
    assert paired_prepared_base_rewrite["near_stitch_failed_details"] == []
    assert paired_prepared_base_rewrite["export_failed_face_indices"] == []
    assert paired_prepared_base_rewrite["export_failed_face_details"] == []
    paired_replacement_prepared_base_rewrite = diagnostics[
        "paired_coplanar_candidate_replacement_prepared_base_record_rewrite"
    ]
    assert paired_replacement_prepared_base_rewrite is not None
    assert (
        paired_replacement_prepared_base_rewrite["prepared_faces"],
        paired_replacement_prepared_base_rewrite["prepared_vertices"],
        paired_replacement_prepared_base_rewrite["virtual_vertices"],
        paired_replacement_prepared_base_rewrite["prepared_face_sources"],
        paired_replacement_prepared_base_rewrite["applied_commands"],
        paired_replacement_prepared_base_rewrite["failed_commands"],
        paired_replacement_prepared_base_rewrite["translated_copied_edge_records"],
        paired_replacement_prepared_base_rewrite["translated_copied_face_records"],
        paired_replacement_prepared_base_rewrite["mapped_source_record_replays"],
        paired_replacement_prepared_base_rewrite[
            "copied_prev_next_edge_update_attempts"
        ],
        paired_replacement_prepared_base_rewrite[
            "copied_prev_next_edge_updates_applied"
        ],
        paired_replacement_prepared_base_rewrite[
            "copied_prev_next_edge_updates_skipped"
        ],
        paired_replacement_prepared_base_rewrite["near_stitch_updates_applied"],
        paired_replacement_prepared_base_rewrite["near_stitch_updates_failed"],
        paired_replacement_prepared_base_rewrite["exported_faces"],
        paired_replacement_prepared_base_rewrite["export_failed_faces"],
        paired_replacement_prepared_base_rewrite["ready_for_export"],
    ) == (20, 20, 0, 20, 16, 0, 64, 16, 48, 0, 0, 0, 0, 0, 36, 0, True)
    assert (
        paired_replacement_prepared_base_rewrite["record_failed_missing_targets"],
        paired_replacement_prepared_base_rewrite["record_failed_closed_targets"],
        paired_replacement_prepared_base_rewrite["record_failed_missing_sources"],
        paired_replacement_prepared_base_rewrite["record_failed_other_commands"],
        paired_replacement_prepared_base_rewrite[
            "record_rewrite_near_stitch_target_left_closures"
        ],
        paired_replacement_prepared_base_rewrite[
            "record_rewrite_near_stitch_target_right_closures"
        ],
        paired_replacement_prepared_base_rewrite[
            "mapped_source_record_replays_on_near_stitch_targets"
        ],
        paired_replacement_prepared_base_rewrite[
            "mapped_source_record_replay_attempts"
        ],
        paired_replacement_prepared_base_rewrite[
            "mapped_source_record_replay_attempts_on_near_stitch_targets"
        ],
        paired_replacement_prepared_base_rewrite[
            "skipped_mapped_source_record_replays"
        ],
        paired_replacement_prepared_base_rewrite["failed_copied_edge_records"],
        paired_replacement_prepared_base_rewrite["refreshed_face_records"],
        paired_replacement_prepared_base_rewrite["near_stitch_failed_start"],
        paired_replacement_prepared_base_rewrite["near_stitch_failed_end"],
        paired_replacement_prepared_base_rewrite[
            "near_stitch_skipped_previous_left_source_edges"
        ],
        paired_replacement_prepared_base_rewrite[
            "near_stitch_skipped_next_right_source_edges"
        ],
        paired_replacement_prepared_base_rewrite["near_stitch_missing_previous_edges"],
        paired_replacement_prepared_base_rewrite["near_stitch_missing_next_edges"],
        paired_replacement_prepared_base_rewrite["near_stitch_origin_mismatches"],
        paired_replacement_prepared_base_rewrite["near_stitch_previous_left_faces"],
        paired_replacement_prepared_base_rewrite[
            "near_stitch_previous_left_copied_source_edges"
        ],
        paired_replacement_prepared_base_rewrite["near_stitch_next_right_faces"],
        paired_replacement_prepared_base_rewrite[
            "near_stitch_next_right_copied_source_edges"
        ],
        paired_replacement_prepared_base_rewrite["near_stitch_failed_other"],
        paired_replacement_prepared_base_rewrite["export_non_triangular_faces"],
        paired_replacement_prepared_base_rewrite[
            "export_left_ring_not_closed_faces"
        ],
        paired_replacement_prepared_base_rewrite["export_missing_origin_faces"],
        paired_replacement_prepared_base_rewrite[
            "export_face_record_left_mismatch_faces"
        ],
        paired_replacement_prepared_base_rewrite[
            "export_face_left_ring_mismatch_faces"
        ],
        paired_replacement_prepared_base_rewrite["export_other_failed_faces"],
    ) == (0, 0, 0, 0, 0, 0, 16, 48, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    assert [
        detail["stitch_pair_index"]
        for detail in paired_replacement_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == list(range(16))
    assert [
        detail["target_edge_id"]
        for detail in paired_replacement_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [31, 37, 19, 27, 73, 71, 9, 13, 59, 55, 53, 63, 45, 41, 39, 51]
    assert [
        detail["target_was_near_stitch_target"]
        for detail in paired_replacement_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [True] * 16
    assert paired_replacement_prepared_base_rewrite[
        "record_rewrite_failed_command_details"
    ] == []
    _assert_mapped_source_record_replay_details(
        paired_replacement_prepared_base_rewrite["mapped_source_record_replay_details"],
        expected_attempts=48,
        expected_applied=48,
    )
    assert paired_replacement_prepared_base_rewrite["near_stitch_failed_details"] == []
    assert paired_replacement_prepared_base_rewrite["export_failed_face_indices"] == []
    assert paired_replacement_prepared_base_rewrite["export_failed_face_details"] == []
    assert paired_replacement_prepared_base_rewrite["exported_face_cut_faces"] == [
        1,
        2,
        6,
        10,
        12,
        13,
        17,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        28,
        29,
        33,
        34,
        35,
        1,
        2,
        3,
        6,
        9,
        11,
        12,
        13,
        16,
        18,
        19,
        20,
        24,
        25,
        26,
        29,
    ]
    assert paired_replacement_prepared_base_rewrite["exported_face_source_faces"] == [
        0,
        0,
        1,
        2,
        3,
        3,
        4,
        5,
        5,
        6,
        6,
        6,
        7,
        7,
        7,
        8,
        8,
        9,
        10,
        11,
        0,
        0,
        0,
        1,
        2,
        3,
        3,
        3,
        4,
        5,
        5,
        5,
        8,
        8,
        8,
        9,
    ]
    assert (
        paired_replacement_prepared_base_rewrite["exported_mesh_stats"][
            "vertex_count"
        ],
        paired_replacement_prepared_base_rewrite["exported_mesh_stats"][
            "face_count"
        ],
        paired_replacement_prepared_base_rewrite["exported_mesh_stats"][
            "connected_components"
        ],
        paired_replacement_prepared_base_rewrite["exported_mesh_stats"][
            "boundary_edge_count"
        ],
        paired_replacement_prepared_base_rewrite["exported_mesh_health"][
            "boundary_edge_count"
        ],
        paired_replacement_prepared_base_rewrite["exported_mesh_health"][
            "nonmanifold_edge_count"
        ],
        paired_replacement_prepared_base_rewrite["exported_mesh_health"][
            "is_closed"
        ],
    ) == (20, 36, 1, 6, 6, 3, False)
    paired_replacement_barriered_prepared_base_rewrite = diagnostics[
        "paired_coplanar_candidate_replacement_barriered_prepared_base_record_rewrite"
    ]
    assert paired_replacement_barriered_prepared_base_rewrite is not None
    assert (
        paired_replacement_barriered_prepared_base_rewrite["prepared_faces"],
        paired_replacement_barriered_prepared_base_rewrite["prepared_vertices"],
        paired_replacement_barriered_prepared_base_rewrite["virtual_vertices"],
        paired_replacement_barriered_prepared_base_rewrite["prepared_face_sources"],
        paired_replacement_barriered_prepared_base_rewrite["applied_commands"],
        paired_replacement_barriered_prepared_base_rewrite["failed_commands"],
        paired_replacement_barriered_prepared_base_rewrite[
            "record_failed_missing_targets"
        ],
        paired_replacement_barriered_prepared_base_rewrite[
            "record_failed_missing_sources"
        ],
        paired_replacement_barriered_prepared_base_rewrite[
            "translated_copied_edge_records"
        ],
        paired_replacement_barriered_prepared_base_rewrite[
            "translated_copied_face_records"
        ],
        paired_replacement_barriered_prepared_base_rewrite[
            "mapped_source_record_replays"
        ],
        paired_replacement_barriered_prepared_base_rewrite[
            "copied_prev_next_edge_update_attempts"
        ],
        paired_replacement_barriered_prepared_base_rewrite[
            "copied_prev_next_edge_updates_applied"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_faces"],
        paired_replacement_barriered_prepared_base_rewrite["export_failed_faces"],
        paired_replacement_barriered_prepared_base_rewrite["ready_for_export"],
    ) == (20, 20, 0, 20, 16, 0, 0, 0, 64, 16, 48, 0, 0, 36, 0, True)
    assert (
        len(
            paired_replacement_barriered_prepared_base_rewrite[
                "record_rewrite_target_details"
            ]
        ),
        len(
            paired_replacement_barriered_prepared_base_rewrite[
                "record_rewrite_failed_command_details"
            ]
        ),
        len(
            paired_replacement_barriered_prepared_base_rewrite[
                "copied_face_record_details"
            ]
        ),
    ) == (16, 0, 16)
    assert [
        detail["stitch_pair_index"]
        for detail in paired_replacement_barriered_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == list(range(16))
    assert [
        detail["target_was_near_stitch_target"]
        for detail in paired_replacement_barriered_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [True] * 16
    assert paired_replacement_barriered_prepared_base_rewrite[
        "export_failed_face_indices"
    ] == []
    assert (
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "vertex_count"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "face_count"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "connected_components"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "boundary_edge_count"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_health"][
            "boundary_edge_count"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_health"][
            "nonmanifold_edge_count"
        ],
        paired_replacement_barriered_prepared_base_rewrite["exported_mesh_health"][
            "is_closed"
        ],
    ) == (20, 36, 1, 6, 6, 3, False)
    owner_remapped_prepared_base_rewrite = diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepared_base_record_rewrite"
    ]
    assert owner_remapped_prepared_base_rewrite is not None
    assert (
        owner_remapped_prepared_base_rewrite["prepared_faces"],
        owner_remapped_prepared_base_rewrite["prepared_vertices"],
        owner_remapped_prepared_base_rewrite["virtual_vertices"],
        owner_remapped_prepared_base_rewrite["prepared_face_sources"],
        owner_remapped_prepared_base_rewrite["applied_commands"],
        owner_remapped_prepared_base_rewrite["failed_commands"],
        owner_remapped_prepared_base_rewrite["translated_copied_edge_records"],
        owner_remapped_prepared_base_rewrite["translated_copied_face_records"],
        owner_remapped_prepared_base_rewrite["mapped_source_record_replays"],
        owner_remapped_prepared_base_rewrite[
            "copied_prev_next_edge_update_attempts"
        ],
        owner_remapped_prepared_base_rewrite[
            "copied_prev_next_edge_updates_applied"
        ],
        owner_remapped_prepared_base_rewrite[
            "copied_prev_next_edge_updates_skipped"
        ],
        owner_remapped_prepared_base_rewrite["near_stitch_updates_applied"],
        owner_remapped_prepared_base_rewrite["near_stitch_updates_failed"],
        owner_remapped_prepared_base_rewrite["exported_faces"],
        owner_remapped_prepared_base_rewrite["export_failed_faces"],
        owner_remapped_prepared_base_rewrite["ready_for_export"],
    ) == (20, 20, 0, 20, 14, 0, 64, 16, 42, 0, 0, 0, 0, 0, 36, 0, True)
    assert (
        owner_remapped_prepared_base_rewrite["record_failed_missing_targets"],
        owner_remapped_prepared_base_rewrite["record_failed_closed_targets"],
        owner_remapped_prepared_base_rewrite["record_failed_missing_sources"],
        owner_remapped_prepared_base_rewrite["record_failed_other_commands"],
        owner_remapped_prepared_base_rewrite[
            "record_rewrite_near_stitch_target_left_closures"
        ],
        owner_remapped_prepared_base_rewrite[
            "record_rewrite_near_stitch_target_right_closures"
        ],
        owner_remapped_prepared_base_rewrite[
            "mapped_source_record_replays_on_near_stitch_targets"
        ],
        owner_remapped_prepared_base_rewrite[
            "mapped_source_record_replay_attempts"
        ],
        owner_remapped_prepared_base_rewrite[
            "mapped_source_record_replay_attempts_on_near_stitch_targets"
        ],
        owner_remapped_prepared_base_rewrite[
            "skipped_mapped_source_record_replays"
        ],
        owner_remapped_prepared_base_rewrite["failed_copied_edge_records"],
        owner_remapped_prepared_base_rewrite["refreshed_face_records"],
        owner_remapped_prepared_base_rewrite["near_stitch_failed_start"],
        owner_remapped_prepared_base_rewrite["near_stitch_failed_end"],
        owner_remapped_prepared_base_rewrite[
            "near_stitch_skipped_previous_left_source_edges"
        ],
        owner_remapped_prepared_base_rewrite[
            "near_stitch_skipped_next_right_source_edges"
        ],
        owner_remapped_prepared_base_rewrite["near_stitch_missing_previous_edges"],
        owner_remapped_prepared_base_rewrite["near_stitch_missing_next_edges"],
        owner_remapped_prepared_base_rewrite["near_stitch_origin_mismatches"],
        owner_remapped_prepared_base_rewrite["near_stitch_previous_left_faces"],
        owner_remapped_prepared_base_rewrite[
            "near_stitch_previous_left_copied_source_edges"
        ],
        owner_remapped_prepared_base_rewrite["near_stitch_next_right_faces"],
        owner_remapped_prepared_base_rewrite[
            "near_stitch_next_right_copied_source_edges"
        ],
        owner_remapped_prepared_base_rewrite["near_stitch_failed_other"],
        owner_remapped_prepared_base_rewrite["export_non_triangular_faces"],
        owner_remapped_prepared_base_rewrite[
            "export_left_ring_not_closed_faces"
        ],
        owner_remapped_prepared_base_rewrite["export_missing_origin_faces"],
        owner_remapped_prepared_base_rewrite[
            "export_face_record_left_mismatch_faces"
        ],
        owner_remapped_prepared_base_rewrite[
            "export_face_left_ring_mismatch_faces"
        ],
        owner_remapped_prepared_base_rewrite["export_other_failed_faces"],
    ) == (0, 0, 0, 0, 0, 0, 12, 42, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    assert [
        detail["stitch_pair_index"]
        for detail in owner_remapped_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
    assert [
        detail["target_edge_id"]
        for detail in owner_remapped_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [31, 19, 27, 73, 71, 9, 13, 59, 55, 53, 63, 45, 41, 39]
    assert [
        detail["target_was_near_stitch_target"]
        for detail in owner_remapped_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [
        False,
        True,
        True,
        True,
        True,
        True,
        True,
        False,
        True,
        True,
        True,
        True,
        True,
        True,
    ]
    owner_remapped_failed_commands = owner_remapped_prepared_base_rewrite[
        "record_rewrite_failed_command_details"
    ]
    assert owner_remapped_failed_commands == []
    _assert_mapped_source_record_replay_details(
        owner_remapped_prepared_base_rewrite["mapped_source_record_replay_details"],
        expected_attempts=42,
        expected_applied=42,
    )
    assert owner_remapped_prepared_base_rewrite["near_stitch_failed_details"] == []
    assert owner_remapped_prepared_base_rewrite["export_failed_face_indices"] == []
    assert owner_remapped_prepared_base_rewrite["export_failed_face_details"] == []
    assert owner_remapped_prepared_base_rewrite[
        "exported_face_operands"
    ] == ["first"] * 20 + ["second"] * 16
    assert owner_remapped_prepared_base_rewrite[
        "exported_face_cut_faces"
    ] == paired_replacement_prepared_base_rewrite["exported_face_cut_faces"]
    assert owner_remapped_prepared_base_rewrite[
        "exported_face_source_faces"
    ] == [
        1,
        2,
        6,
        10,
        7,
        7,
        4,
        4,
        4,
        5,
        5,
        5,
        5,
        5,
        5,
        5,
        5,
        1,
        1,
        0,
        1,
        2,
        3,
        6,
        9,
        11,
        3,
        3,
        3,
        2,
        2,
        2,
        2,
        3,
        11,
        11,
    ]
    assert owner_remapped_prepared_base_rewrite[
        "exported_face_source_faces"
    ] != paired_replacement_prepared_base_rewrite["exported_face_source_faces"]
    assert [
        (detail["output_face"], detail["cut_face"], detail["source_face"])
        for detail in owner_remapped_prepared_base_rewrite[
            "copied_face_record_details"
        ]
    ] == [
        (20, 1, 1),
        (21, 2, 2),
        (22, 3, 3),
        (23, 6, 6),
        (24, 9, 9),
        (25, 11, 11),
        (26, 12, 3),
        (27, 13, 3),
        (28, 16, 3),
        (29, 18, 2),
        (30, 19, 2),
        (31, 20, 2),
        (32, 24, 2),
        (33, 25, 3),
        (34, 26, 11),
        (35, 29, 11),
    ]
    assert all(
        detail["selected_by_valid_left_ring"]
        for detail in owner_remapped_prepared_base_rewrite[
            "copied_face_record_details"
        ]
    )
    assert (
        owner_remapped_prepared_base_rewrite["exported_mesh_stats"][
            "vertex_count"
        ],
        owner_remapped_prepared_base_rewrite["exported_mesh_stats"]["face_count"],
        owner_remapped_prepared_base_rewrite["exported_mesh_stats"][
            "connected_components"
        ],
        owner_remapped_prepared_base_rewrite["exported_mesh_stats"][
            "boundary_edge_count"
        ],
        owner_remapped_prepared_base_rewrite["exported_mesh_health"][
            "boundary_edge_count"
        ],
        owner_remapped_prepared_base_rewrite["exported_mesh_health"][
            "nonmanifold_edge_count"
        ],
        owner_remapped_prepared_base_rewrite["exported_mesh_health"]["is_closed"],
    ) == (20, 36, 1, 6, 6, 3, False)
    owner_remapped_barriered_prepared_base_rewrite = diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_barriered_prepared_base_record_rewrite"
    ]
    assert owner_remapped_barriered_prepared_base_rewrite is not None
    assert (
        owner_remapped_barriered_prepared_base_rewrite["prepared_faces"],
        owner_remapped_barriered_prepared_base_rewrite["prepared_vertices"],
        owner_remapped_barriered_prepared_base_rewrite["virtual_vertices"],
        owner_remapped_barriered_prepared_base_rewrite["prepared_face_sources"],
        owner_remapped_barriered_prepared_base_rewrite["applied_commands"],
        owner_remapped_barriered_prepared_base_rewrite["failed_commands"],
        owner_remapped_barriered_prepared_base_rewrite[
            "record_failed_missing_targets"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "record_failed_missing_sources"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "translated_copied_edge_records"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "translated_copied_face_records"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "mapped_source_record_replays"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "copied_prev_next_edge_update_attempts"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "copied_prev_next_edge_updates_applied"
        ],
        owner_remapped_barriered_prepared_base_rewrite[
            "copied_prev_next_edge_updates_skipped"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_faces"],
        owner_remapped_barriered_prepared_base_rewrite["export_failed_faces"],
        owner_remapped_barriered_prepared_base_rewrite["ready_for_export"],
    ) == (20, 20, 0, 20, 16, 0, 0, 0, 64, 16, 48, 0, 0, 0, 36, 0, True)
    assert (
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "vertex_count"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "face_count"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "connected_components"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_stats"][
            "boundary_edge_count"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_health"][
            "boundary_edge_count"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_health"][
            "nonmanifold_edge_count"
        ],
        owner_remapped_barriered_prepared_base_rewrite["exported_mesh_health"][
            "is_closed"
        ],
    ) == (20, 36, 1, 6, 6, 3, False)
    assert (
        owner_remapped_barriered_prepared_base_rewrite["packed_mesh_stats"][
            "vertex_count"
        ],
        owner_remapped_barriered_prepared_base_rewrite["packed_mesh_stats"]["face_count"],
        owner_remapped_barriered_prepared_base_rewrite["packed_mesh_health"][
            "boundary_edge_count"
        ],
    ) == (20, 36, 6)
    owner_remapped_barriered_failed_commands = (
        owner_remapped_barriered_prepared_base_rewrite[
            "record_rewrite_failed_command_details"
        ]
    )
    assert owner_remapped_barriered_failed_commands == []
    assert [
        detail["stitch_pair_index"]
        for detail in owner_remapped_barriered_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == list(range(16))
    assert [
        detail["target_edge_id"]
        for detail in owner_remapped_barriered_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [31, 37, 19, 27, 73, 71, 9, 13, 59, 55, 53, 63, 45, 41, 39, 51]
    assert [
        detail["target_was_near_stitch_target"]
        for detail in owner_remapped_barriered_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [True] * 16
    _assert_mapped_source_record_replay_details(
        owner_remapped_barriered_prepared_base_rewrite[
            "mapped_source_record_replay_details"
        ],
        expected_attempts=48,
        expected_applied=48,
    )
    assert owner_remapped_barriered_prepared_base_rewrite[
        "exported_face_source_faces"
    ][:20] == [1, 2, 6, 10, 7, 7, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 1, 1, 0]
    assert owner_remapped_barriered_prepared_base_rewrite[
        "exported_face_source_faces"
    ][20:] == owner_remapped_prepared_base_rewrite["exported_face_source_faces"][20:]
    slot_projected_owner_remapped_barriered_prepared_base_rewrite = diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepared_base_record_rewrite"
    ]
    assert slot_projected_owner_remapped_barriered_prepared_base_rewrite is not None
    expected_slot_projected_first_prepare = [
        8,
        9,
        11,
        14,
        15,
        16,
        27,
        30,
        31,
        32,
        37,
    ]
    expected_slot_projected_first_added = [37]
    expected_slot_projected_first_base = [
        face
        for face in expected_slot_projected_first_prepare
        if face not in expected_slot_projected_first_added
    ]
    expected_slot_projected_second_prepare = [
        face
        for face in diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_second_face_indices"
        ]
        if face in diagnostics["paired_coplanar_candidate_second_meshlib_valid_cut_faces"]
    ]
    expected_slot_projected_second_selected = [
        20,
        30,
        33,
        36,
        37,
        38,
        40,
        42,
        43,
    ]
    expected_slot_projected_second_added = [36, 37, 44, 45, 46, 47, 48, 49]
    expected_slot_projected_second_base = [
        face
        for face in expected_slot_projected_second_prepare
        if face not in expected_slot_projected_second_added
    ]
    assert (
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "prepared_faces"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "prepared_vertices"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "prepared_face_sources"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "applied_commands"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "failed_commands"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "record_failed_missing_targets"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "record_failed_missing_sources"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "translated_copied_edge_records"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "translated_copied_face_records"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "exported_faces"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "export_failed_faces"
        ],
        slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "ready_for_export"
        ],
    ) == (10, 14, 10, 9, 1, 0, 1, 54, 12, 22, 0, False)
    assert slot_projected_owner_remapped_barriered_prepared_base_rewrite[
        "exported_face_operands"
    ] == ["first"] * 10 + ["second"] * 12
    assert slot_projected_owner_remapped_barriered_prepared_base_rewrite[
        "exported_face_cut_faces"
    ] == expected_slot_projected_first_base + expected_slot_projected_second_base
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_first_face_indices"
    ] == expected_slot_projected_first_prepare
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_second_face_indices"
    ] == expected_slot_projected_second_prepare
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_first_face_indices"
    ] == expected_slot_projected_first_prepare
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_second_face_indices"
    ] == expected_slot_projected_second_selected
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_first_prepare_part_dividable"
    ] is True
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_second_prepare_part_dividable"
    ] is True
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_first_prepare_part_dividable"
    ] is False
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_second_prepare_part_dividable"
    ] is False
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_selected_first_face_indices"
    ] == [36]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_selected_second_face_indices"
    ] == [37, 38, 39, 40, 41, 42, 43]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_no_contact_barrier_selected_second_face_indices"
    ] != expected_slot_projected_second_selected
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_selected_first_face_indices"
    ] == expected_slot_projected_first_prepare
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_fixed_barriered_selected_second_face_indices"
    ] == expected_slot_projected_second_selected
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_first_added_face_indices"
    ] == expected_slot_projected_first_added
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_second_added_face_indices"
    ] == expected_slot_projected_second_added
    assert [
        face
        for face in diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepare_first_face_indices"
        ]
        if face
        not in slot_projected_owner_remapped_barriered_prepared_base_rewrite[
            "exported_face_cut_faces"
        ]
    ] == expected_slot_projected_first_added
    slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite = (
        diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_prepared_base_record_rewrite"
        ]
    )
    assert (
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "prepared_faces"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "prepared_vertices"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "prepared_face_sources"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "applied_commands"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "failed_commands"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "record_failed_missing_targets"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "record_failed_missing_sources"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "translated_copied_edge_records"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "translated_copied_face_records"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "exported_faces"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "export_failed_faces"
        ],
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "ready_for_export"
        ],
    ) == (11, 15, 11, 10, 0, 0, 0, 82, 20, 31, 0, False)
    assert slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
        "exported_face_operands"
    ] == ["first"] * 11 + ["second"] * 20
    assert slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
        "exported_face_cut_faces"
    ] == expected_slot_projected_first_prepare + expected_slot_projected_second_prepare
    assert (
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "record_rewrite_failed_command_details"
        ]
        == []
    )
    assert (
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "mapped_source_record_replays"
        ]
        == 25
    )
    _assert_mapped_source_record_replay_details(
        slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "mapped_source_record_replay_details"
        ],
        expected_attempts=25,
        expected_applied=25,
    )
    assert [
        detail["stitch_pair_index"]
        for detail in slot_projected_owner_remapped_barriered_added_fill_prepared_base_rewrite[
            "record_rewrite_target_details"
        ]
    ] == [0, 2, 3, 4, 5, 6, 9, 11, 12, 13]
    slot_projected_prepared_base_first_cut_faces = [
        face
        for operand, face in zip(
            slot_projected_owner_remapped_barriered_prepared_base_rewrite[
                "exported_face_operands"
            ],
            slot_projected_owner_remapped_barriered_prepared_base_rewrite[
                "exported_face_cut_faces"
            ],
        )
        if operand == "first"
    ]
    slot_projected_prepared_base_second_cut_faces = [
        face
        for operand, face in zip(
            slot_projected_owner_remapped_barriered_prepared_base_rewrite[
                "exported_face_operands"
            ],
            slot_projected_owner_remapped_barriered_prepared_base_rewrite[
                "exported_face_cut_faces"
            ],
        )
        if operand == "second"
    ]
    assert [
        face
        for face in meshlib_in_memory["mapped_a_cut_faces"]
        if face not in slot_projected_prepared_base_first_cut_faces
    ] == [
        2,
        3,
        10,
        20,
        24,
        25,
        26,
        28,
        29,
        34,
        37,
        38,
        39,
        40,
        41,
        42,
        46,
        47,
        48,
        49,
    ]
    assert [
        face
        for face in slot_projected_prepared_base_first_cut_faces
        if face not in meshlib_in_memory["mapped_a_cut_faces"]
    ] == [31, 32]
    assert [
        face
        for face in meshlib_in_memory["mapped_b_cut_faces"]
        if face not in slot_projected_prepared_base_second_cut_faces
    ] == [14, 15, 17, 23, 27, 28, 36, 37, 38, 44, 45, 46, 49]
    assert [
        face
        for face in slot_projected_prepared_base_second_cut_faces
        if face not in meshlib_in_memory["mapped_b_cut_faces"]
    ] == [1, 6, 12, 13, 18, 19, 20, 25, 26]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_coverage"
    ] == [
        [0, 0, 7, 2, 0, 5, 12, 17, 3],
        [0, 1, 4, 1, 1, 4, 17, 21, 0],
        [0, 2, 5, 4, 2, 9, 21, 30, 1],
        [0, 3, 4, 1, 0, 1, 30, 31, 1],
        [0, 4, 1, 1, 0, 4, 31, 35, 2],
        [0, 5, 0, 3, 1, 7, 35, 42, 1],
        [0, 6, 1, 1, 1, 1, 42, 43, 0],
        [0, 7, 6, 3, 1, 7, 43, 50, 0],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_coverage"
    ] == [
        [0, 0, 3, 2, 1, 6, 12, 18, 0],
        [0, 1, 2, 3, 1, 7, 18, 25, 1],
        [0, 2, 3, 1, 1, 1, 25, 26, 0],
        [0, 3, 11, 1, 1, 4, 26, 30, 0],
        [0, 4, 10, 4, 0, 9, 30, 39, 5],
        [0, 5, 11, 1, 1, 1, 39, 40, 0],
        [0, 6, 8, 3, 1, 7, 40, 47, 3],
        [0, 7, 9, 1, 0, 3, 47, 50, 0],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_coverage"
    ] == [
        [0, 0, 7, 2, 0, 5, 12, 17, 3],
        [0, 1, 4, 1, 1, 4, 17, 21, 0],
        [0, 2, 5, 4, 2, 9, 21, 30, 1],
        [0, 3, 4, 1, 0, 1, 30, 31, 1],
        [0, 4, 1, 1, 0, 4, 31, 35, 2],
        [0, 5, 0, 3, 1, 7, 35, 42, 0],
        [0, 6, 1, 1, 1, 1, 42, 43, 0],
        [0, 7, 6, 3, 1, 7, 43, 50, 0],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_coverage"
    ] == [
        [0, 0, 3, 2, 1, 6, 12, 18, 3],
        [0, 1, 2, 3, 1, 7, 18, 25, 4],
        [0, 2, 3, 1, 1, 1, 25, 26, 1],
        [0, 3, 11, 1, 1, 4, 26, 30, 2],
        [0, 4, 10, 4, 0, 9, 30, 39, 0],
        [0, 5, 11, 1, 1, 1, 39, 40, 0],
        [0, 6, 8, 3, 1, 7, 40, 47, 0],
        [0, 7, 9, 1, 0, 3, 47, 50, 0],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_coverage"
    ] == [
        [0, 0, 7, 2, 0, 5, 12, 17, 3],
        [0, 1, 4, 1, 1, 4, 17, 21, 0],
        [0, 2, 5, 4, 2, 9, 21, 30, 1],
        [0, 3, 4, 1, 0, 1, 30, 31, 1],
        [0, 4, 1, 1, 0, 4, 31, 35, 2],
        [0, 5, 0, 3, 1, 7, 35, 42, 1],
        [0, 6, 1, 1, 1, 1, 42, 43, 0],
        [0, 7, 6, 3, 1, 7, 43, 50, 0],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_coverage"
    ] == [
        [0, 0, 3, 2, 1, 6, 12, 18, 3],
        [0, 1, 2, 3, 1, 7, 18, 25, 4],
        [0, 2, 3, 1, 1, 1, 25, 26, 1],
        [0, 3, 11, 1, 1, 4, 26, 30, 2],
        [0, 4, 10, 4, 0, 9, 30, 39, 2],
        [0, 5, 11, 1, 1, 1, 39, 40, 0],
        [0, 6, 8, 3, 1, 7, 40, 47, 3],
        [0, 7, 9, 1, 0, 3, 47, 50, 3],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_slots"
    ] == [[14, 15, 16], [], [27], [30], [31, 32], [37], [], []]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_slots"
    ] == [
        [],
        [20],
        [],
        [],
        [30, 33, 36, 37, 38],
        [],
        [40, 42, 43],
        [],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_slots"
    ] == [[14, 15, 16], [], [27], [30], [31, 32], [], [], []]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_lifecycle_export_slots"
    ] == [
        [12, 13, 16],
        [18, 19, 20, 24],
        [25],
        [26, 29],
        [],
        [],
        [],
        [],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots"
    ] == diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_selected_lifecycle_slots"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots"
    ] == [
        [12, 13, 16],
        [18, 19, 20, 24],
        [25],
        [26, 29],
        [36, 37],
        [],
        [44, 45, 46],
        [47, 48, 49],
    ]
    meshlib_mapped_a_lifecycle_slots = _slots_by_lifecycle_runs(
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs"
        ],
        meshlib_in_memory["mapped_a_cut_faces"],
    )
    meshlib_mapped_b_lifecycle_slots = _slots_by_lifecycle_runs(
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs"
        ],
        meshlib_in_memory["mapped_b_cut_faces"],
    )
    assert meshlib_mapped_a_lifecycle_slots == [
        [14, 15, 16],
        [20],
        [24, 25, 26, 27, 28, 29],
        [30],
        [34],
        [37, 38, 39, 40, 41],
        [42],
        [46, 47, 48, 49],
    ]
    assert meshlib_mapped_b_lifecycle_slots == [
        [14, 15, 16, 17],
        [23, 24],
        [],
        [27, 28, 29],
        [36, 37, 38],
        [],
        [44, 45, 46],
        [49],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots"
    ] != meshlib_mapped_a_lifecycle_slots
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots"
    ] != meshlib_mapped_b_lifecycle_slots
    (
        missing_a_lifecycle_slots,
        extra_a_lifecycle_slots,
    ) = _slot_group_deltas(
        meshlib_mapped_a_lifecycle_slots,
        diagnostics[
            "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots"
        ],
    )
    (
        missing_b_lifecycle_slots,
        extra_b_lifecycle_slots,
    ) = _slot_group_deltas(
        meshlib_mapped_b_lifecycle_slots,
        diagnostics[
            "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_lifecycle_export_slots"
        ],
    )
    assert missing_a_lifecycle_slots == [
        [],
        [20],
        [24, 25, 26, 28, 29],
        [],
        [34],
        [38, 39, 40, 41],
        [42],
        [46, 47, 48, 49],
    ]
    assert extra_a_lifecycle_slots == [[], [], [], [], [31, 32], [], [], []]
    assert missing_b_lifecycle_slots == [
        [14, 15, 17],
        [23],
        [],
        [27, 28],
        [38],
        [],
        [],
        [],
    ]
    assert extra_b_lifecycle_slots == [
        [12, 13],
        [18, 19, 20],
        [25],
        [26],
        [],
        [],
        [],
        [47, 48],
    ]
    assert meshlib_in_memory["mapped_b_cut_faces"]
    assert diagnostics[
        "paired_coplanar_candidate_prepare_first_face_indices"
    ] == [
        1,
        2,
        6,
        10,
        12,
        13,
        17,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        28,
        29,
        33,
        34,
        35,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_prepare_second_face_indices"
    ] == [
        1,
        2,
        3,
        6,
        9,
        11,
        12,
        13,
        16,
        18,
        19,
        20,
        24,
        25,
        26,
        29,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_selected_first_face_indices"
    ] == [
        1,
        2,
        6,
        10,
        12,
        13,
        17,
        19,
        20,
        28,
        29,
        33,
        34,
        35,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_selected_second_face_indices"
    ] == [30, 31, 32, 33, 34, 35]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_first_prepare_part_dividable"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_second_prepare_part_dividable"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_first_cut_path_side_components"
    ] == [1, 1]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_second_cut_path_side_components"
    ] == [1, 1]
    assert (
        diagnostics[
            "paired_coplanar_candidate_replacement_first_cut_path_overlap_components"
        ],
        diagnostics[
            "paired_coplanar_candidate_replacement_second_cut_path_overlap_components"
        ],
    ) == (0, 0)
    assert diagnostics[
        "paired_coplanar_candidate_replacement_first_cut_path_left_component_indices"
    ] == [0, 4]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_first_cut_path_right_component_indices"
    ] == [1, 2, 3]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_first_cut_path_overlap_component_indices"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_second_cut_path_left_component_indices"
    ] == [0, 2, 3]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_second_cut_path_right_component_indices"
    ] == [1, 4]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_second_cut_path_overlap_component_indices"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_first_cut_path_overlap_component_faces"
    ] == []
    assert (
        diagnostics[
            "paired_coplanar_candidate_replacement_second_cut_path_overlap_component_faces"
        ]
        == []
    )
    assert diagnostics[
        "paired_coplanar_candidate_replacement_synthetic_contact_edges"
    ] == [18, 18]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_first_prepare_part_dividable"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_second_prepare_part_dividable"
    ]
    assert (
        diagnostics[
            "paired_coplanar_candidate_replacement_barriered_first_cut_path_overlap_components"
        ],
        diagnostics[
            "paired_coplanar_candidate_replacement_barriered_second_cut_path_overlap_components"
        ],
    ) == (0, 0)
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_first_cut_path_overlap_component_indices"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_second_cut_path_overlap_component_indices"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_first_cut_path_overlap_component_faces"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_second_cut_path_overlap_component_faces"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_prepare_first_face_indices"
    ] == [
        1,
        2,
        6,
        10,
        12,
        13,
        17,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        28,
        29,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_barriered_prepare_second_face_indices"
    ] == [
        1,
        2,
        3,
        6,
        9,
        11,
        12,
        13,
        16,
        18,
        19,
        20,
        24,
        25,
        26,
        29,
        42,
        43,
        44,
        45,
        46,
        47,
    ]
    assert not diagnostics[
        "paired_coplanar_candidate_replacement_fixed_barriered_first_prepare_part_dividable"
    ]
    assert not diagnostics[
        "paired_coplanar_candidate_replacement_fixed_barriered_second_prepare_part_dividable"
    ]
    assert (
        diagnostics[
            "paired_coplanar_candidate_replacement_fixed_barriered_first_cut_path_overlap_components"
        ],
        diagnostics[
            "paired_coplanar_candidate_replacement_fixed_barriered_second_cut_path_overlap_components"
        ],
    ) == (1, 1)
    assert diagnostics[
        "paired_coplanar_candidate_replacement_fixed_barriered_prepare_first_face_indices"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_fixed_barriered_prepare_second_face_indices"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_replacement_prepare_first_face_indices"
    ] == [
        1,
        2,
        6,
        10,
        12,
        13,
        17,
        19,
        20,
        21,
        22,
        23,
        24,
        25,
        26,
        28,
        29,
        33,
        34,
        35,
        36,
        37,
        38,
        39,
        40,
        41,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_prepare_second_face_indices"
    ] == [
        1,
        2,
        3,
        6,
        9,
        11,
        12,
        13,
        16,
        18,
        19,
        20,
        24,
        25,
        26,
        29,
        42,
        43,
        44,
        45,
        46,
        47,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_selected_first_face_indices"
    ] == [
        0,
        3,
        4,
        5,
        7,
        8,
        9,
        11,
        14,
        15,
        16,
        18,
        27,
        30,
        31,
        32,
        36,
        37,
        38,
        39,
        40,
        41,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_selected_second_face_indices"
    ] == []
    assert diagnostics["paired_coplanar_candidate_replacement_result_cut_paths_complete"]
    assert diagnostics[
        "paired_coplanar_candidate_replacement_prepare_result_cut_paths_complete"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_first_prepare_part_dividable"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_second_prepare_part_dividable"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_result_cut_paths_complete"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_result_cut_paths_complete"
    ]
    assert (
        diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_first_face_indices"
        ],
        diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepare_second_face_indices"
        ],
        diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_selected_first_face_indices"
        ],
        diagnostics[
            "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_selected_second_face_indices"
        ],
    ) == (
        [
            1,
            2,
            6,
            10,
            12,
            13,
            17,
            19,
            20,
            21,
            22,
            23,
            24,
            25,
            26,
            28,
            29,
            33,
            34,
            35,
            36,
            37,
            38,
            39,
            40,
            41,
            42,
            43,
        ],
        [
            1,
            2,
            3,
            6,
            9,
            11,
            12,
            13,
            16,
            18,
            19,
            20,
            24,
            25,
            26,
            29,
            36,
            37,
            44,
            45,
            46,
            47,
            48,
            49,
        ],
        [
            0,
            3,
            4,
            5,
            7,
            8,
            9,
            11,
            14,
            15,
            16,
            18,
            27,
            30,
            31,
            32,
            36,
            37,
            38,
            39,
            40,
            41,
            42,
            43,
        ],
        [
            36,
            37,
        ],
    )
    assert [
        face
        for face in meshlib_in_memory["mapped_a_cut_faces"]
        if face not in diagnostics[
            "paired_coplanar_candidate_replacement_selected_first_face_indices"
        ]
    ] == [2, 10, 20, 24, 25, 26, 28, 29, 34, 42, 46, 47, 48, 49]
    assert [
        face
        for face in diagnostics[
            "paired_coplanar_candidate_replacement_selected_first_face_indices"
        ]
        if face not in meshlib_in_memory["mapped_a_cut_faces"]
    ] == [0, 4, 5, 7, 18, 31, 32, 36]
    assert [
        face
        for face in meshlib_in_memory["mapped_b_cut_faces"]
        if face not in diagnostics[
            "paired_coplanar_candidate_replacement_prepare_second_face_indices"
        ]
    ] == [14, 15, 17, 23, 27, 28, 36, 37, 38, 49]
    assert [
        face
        for face in diagnostics[
            "paired_coplanar_candidate_replacement_prepare_second_face_indices"
        ]
        if face not in meshlib_in_memory["mapped_b_cut_faces"]
    ] == [1, 2, 3, 6, 9, 11, 12, 13, 18, 19, 20, 25, 26, 42, 43, 47]
    assert [
        face
        for face in meshlib_in_memory["mapped_a_cut_faces"]
        if face not in diagnostics[
            "paired_coplanar_candidate_prepare_first_face_indices"
        ]
    ] == [
        3,
        8,
        9,
        11,
        14,
        15,
        16,
        27,
        30,
        37,
        38,
        39,
        40,
        41,
        42,
        46,
        47,
        48,
        49,
    ]
    assert [
        face
        for face in diagnostics["paired_coplanar_candidate_prepare_first_face_indices"]
        if face not in meshlib_in_memory["mapped_a_cut_faces"]
    ] == [1, 6, 12, 13, 17, 19, 21, 22, 23, 33, 35]
    assert [
        face
        for face in meshlib_in_memory["mapped_b_cut_faces"]
        if face not in diagnostics[
            "paired_coplanar_candidate_prepare_second_face_indices"
        ]
    ] == [14, 15, 17, 23, 27, 28, 36, 37, 38, 44, 45, 46, 49]
    assert [
        face
        for face in diagnostics["paired_coplanar_candidate_prepare_second_face_indices"]
        if face not in meshlib_in_memory["mapped_b_cut_faces"]
    ] == [1, 2, 3, 6, 9, 11, 12, 13, 18, 19, 20, 25, 26]
    assert (
        diagnostics["paired_coplanar_candidate_meshlib_base_faces"],
        diagnostics["paired_coplanar_candidate_meshlib_incoming_faces"],
        diagnostics["paired_coplanar_candidate_meshlib_unstitched_faces"],
    ) == (
        paired_prepared_base_rewrite["prepared_faces"],
        paired_prepared_base_rewrite["translated_copied_face_records"],
        paired_prepared_base_rewrite["exported_faces"],
    )
    assert paired_prepared_base_rewrite["exported_mesh_stats"] is not None
    assert paired_prepared_base_rewrite["exported_mesh_health"] is not None
    assert paired_prepared_base_rewrite["packed_mesh_stats"] is not None
    assert paired_prepared_base_rewrite["packed_mesh_health"] is not None
    assert (
        diagnostics["paired_coplanar_candidate_meshlib_base_vertices"],
        diagnostics["paired_coplanar_candidate_meshlib_incoming_vertices"],
        diagnostics["paired_coplanar_candidate_meshlib_unstitched_vertices"],
    ) == (
        paired_prepared_base_rewrite["prepared_vertices"],
        meshlib_in_memory["mapped_b_faces"],
        diagnostics["paired_coplanar_candidate_meshlib_base_vertices"]
        + diagnostics["paired_coplanar_candidate_meshlib_incoming_vertices"],
    )
    assert diagnostics["paired_coplanar_candidate_meshlib_base_cut_paths_complete"]
    assert diagnostics["paired_coplanar_candidate_meshlib_incoming_cut_paths_complete"]
    assert not diagnostics["paired_coplanar_candidate_meshlib_path_count_mismatch"]
    assert diagnostics["paired_coplanar_candidate_meshlib_path_length_mismatches"] == 0
    assert diagnostics["paired_coplanar_candidate_cut_faces"] == [36, 36]
    assert diagnostics["paired_coplanar_candidate_cut_source_records"] == [36, 36]
    assert diagnostics["paired_coplanar_candidate_cut_unique_source_faces"] == [12, 12]
    assert diagnostics["paired_coplanar_candidate_cut_duplicate_source_records"] == [24, 24]
    assert diagnostics["paired_coplanar_candidate_cut_fill_plans"] == [0, 0]
    assert diagnostics["paired_coplanar_candidate_cut_added_faces"] == [0, 0]
    assert diagnostics["paired_coplanar_candidate_replacement_cut_faces"] == [48, 48]
    assert diagnostics["paired_coplanar_candidate_replacement_cut_source_records"] == [48, 48]
    assert diagnostics["paired_coplanar_candidate_replacement_cut_unique_source_faces"] == [12, 12]
    assert diagnostics["paired_coplanar_candidate_replacement_cut_duplicate_source_records"] == [
        36,
        36,
    ]
    assert diagnostics["paired_coplanar_candidate_replacement_cut_fill_plans"] == [2, 2]
    assert diagnostics["paired_coplanar_candidate_replacement_cut_added_faces"] == [12, 12]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_cut_path_lengths"
    ] == diagnostics["paired_coplanar_candidate_first_cut_path_lengths"]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_cut_path_lengths"
    ] == diagnostics["paired_coplanar_candidate_second_cut_path_lengths"]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_lengths"
    ] == diagnostics["paired_coplanar_candidate_first_closed_cut_path_lengths"]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_lengths"
    ] == diagnostics["paired_coplanar_candidate_second_closed_cut_path_lengths"]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_source_faces"
    ] == diagnostics["paired_coplanar_candidate_first_closed_cut_path_source_faces"]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_source_faces"
    ] == diagnostics["paired_coplanar_candidate_second_closed_cut_path_source_faces"]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_source_face_runs"
    ] == diagnostics["paired_coplanar_candidate_first_closed_cut_path_source_face_runs"]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_source_face_runs"
    ] == diagnostics["paired_coplanar_candidate_second_closed_cut_path_source_face_runs"]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_edge_adjacent_source_faces"
    ] == [
        [[4], [4, 5], [2, 4], [3, 4], [4, 9], [4, 8], [0, 4], [1, 4]],
        [[4, 7], [4, 7], [2, 7], [2, 7], [6, 7, 9], [6, 7, 9], [1, 6, 7], [1, 6, 7]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_edge_adjacent_source_faces"
    ] == [
        [
            [5, 10],
            [5, 10],
            [3, 10, 11],
            [3, 10, 11],
            [8, 10, 11],
            [8, 10, 11],
            [0, 10],
            [0, 10],
        ],
        [[4], [4, 5], [2, 4], [3, 4], [4, 9], [4, 8], [0, 4], [1, 4]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_edge_left_primary_source_faces"
    ] == [[4, 4, 2, 3, 4, 4, 0, 1], [4, 4, 2, 2, 7, 7, 1, 1]]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_edge_left_primary_source_faces"
    ] == [[10, 10, 10, 10, 10, 10, 10, 10], [4, 4, 2, 3, 4, 4, 0, 1]]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_edge_right_primary_source_faces"
    ] == [[4, 5, 2, 3, 9, 8, 0, 1], [7, 7, 7, 7, 6, 6, 6, 6]]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_edge_right_primary_source_faces"
    ] == [[5, 5, 3, 3, 8, 8, 0, 0], [4, 5, 2, 3, 9, 8, 0, 1]]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_edge_left_primary_source_face_runs"
    ] == [
        [[4, 2], [2, 1], [3, 1], [4, 2], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [7, 2], [1, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_edge_left_primary_source_face_runs"
    ] == [
        [[10, 8]],
        [[4, 2], [2, 1], [3, 1], [4, 2], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_closed_cut_path_edge_right_primary_source_face_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[7, 4], [6, 4]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_closed_cut_path_edge_right_primary_source_face_runs"
    ] == [
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_meshlib_removed_face_owner_candidates"
    ] == [
        [4, 4, 2, 3, 4, 4, 0, 1],
        [7, 7, 7, 7, 7, 7, 6, 6],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_meshlib_removed_face_owner_candidates"
    ] == [
        [10, 10, 10, 10, 10, 10, 10, 10],
        [4, 4, 2, 3, 4, 4, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_meshlib_removed_face_owner_candidate_runs"
    ] == [
        [[4, 2], [2, 1], [3, 1], [4, 2], [0, 1], [1, 1]],
        [[7, 6], [6, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_meshlib_removed_face_owner_candidate_runs"
    ] == [
        [[10, 8]],
        [[4, 2], [2, 1], [3, 1], [4, 2], [0, 1], [1, 1]],
    ]
    assert (
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_unique_source_faces"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_duplicate_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_split_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_cut_event_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_split_event_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_prefill_repair_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_open_edge_restoration_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_orphan_repair_source_records"
        ],
        diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_fill_source_records"
        ],
    ) == (
        [50, 50],
        [12, 12],
        [38, 38],
        [24, 24],
        [36, 36],
        [34, 34],
        [2, 2],
        [2, 2],
        [0, 0],
        [12, 12],
    )
    assert (
        diagnostics["coplanar_cut_trial_contours"],
        diagnostics["coplanar_cut_trial_contour_edges"],
        diagnostics["coplanar_cut_trial_first_cut_edges"],
        diagnostics["coplanar_cut_trial_second_cut_edges"],
    ) == (8, 32, 24, 24)
    assert (
        diagnostics["paired_coplanar_cut_trial_contours"],
        diagnostics["paired_coplanar_cut_trial_contour_edges"],
        diagnostics["paired_coplanar_cut_trial_first_cut_edges"],
        diagnostics["paired_coplanar_cut_trial_second_cut_edges"],
        diagnostics["paired_coplanar_stitch_cut_path_length_mismatches"],
        diagnostics["paired_coplanar_stitch_unmatched_first_edges"],
        diagnostics["paired_coplanar_stitch_unmatched_second_edges"],
        diagnostics["paired_coplanar_duplicate_first_path_edges"],
        diagnostics["paired_coplanar_duplicate_second_path_edges"],
        diagnostics["paired_coplanar_duplicate_first_path_edge_occurrences"],
        diagnostics["paired_coplanar_duplicate_second_path_edge_occurrences"],
    ) == (2, 16, 16, 16, 0, 0, 0, 0, 0, 0, 0)
    assert diagnostics["paired_coplanar_duplicate_first_path_edge_path_indices"] == []
    assert diagnostics["paired_coplanar_duplicate_second_path_edge_path_indices"] == []
    assert diagnostics["paired_coplanar_combined_first_cut_path_lengths"] == [
        10,
        8,
        8,
    ]
    assert diagnostics["paired_coplanar_combined_second_cut_path_lengths"] == [
        10,
        8,
        8,
    ]
    assert diagnostics["paired_coplanar_combined_first_cut_path_lengths"] != [16]
    assert diagnostics["paired_coplanar_combined_second_cut_path_lengths"] != [16]
    assert diagnostics["paired_coplanar_combined_first_cut_path_source_faces"] == [
        [6, 6, 2, 2, 2, 5, 4, 1, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
        [4, 4, 2, 2, 6, 6, 1, 1],
    ]
    assert diagnostics["paired_coplanar_combined_second_cut_path_source_faces"] == [
        [8, 9, 3, 2, 2, 5, 5, 0, 0, 0],
        [5, 5, 3, 3, 8, 8, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics["paired_coplanar_combined_first_cut_path_source_face_runs"] == [
        [[6, 2], [2, 3], [5, 1], [4, 1], [1, 1], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [6, 2], [1, 2]],
    ]
    assert diagnostics["paired_coplanar_combined_second_cut_path_source_face_runs"] == [
        [[8, 1], [9, 1], [3, 1], [2, 2], [5, 2], [0, 3]],
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_collapsed_cut_path_lengths"
    ] == [6, 0, 0]
    assert diagnostics[
        "paired_coplanar_combined_second_collapsed_cut_path_lengths"
    ] == [6, 0, 0]
    assert diagnostics[
        "paired_coplanar_combined_first_collapsed_cut_path_source_faces"
    ] == [[0, 2, 2, 2, 2, 0], [], []]
    assert diagnostics[
        "paired_coplanar_combined_second_collapsed_cut_path_source_faces"
    ] == [[0, 3, 2, 2, 2, 0], [], []]
    assert diagnostics[
        "paired_coplanar_combined_first_collapsed_cut_path_source_face_runs"
    ] == [[[0, 1], [2, 4], [0, 1]], [], []]
    assert diagnostics[
        "paired_coplanar_combined_second_collapsed_cut_path_source_face_runs"
    ] == [[[0, 1], [3, 1], [2, 3], [0, 1]], [], []]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_cut_path_lengths"
    ] == [16, 8, 8]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_cut_path_lengths"
    ] == [16, 8, 8]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_cut_path_source_faces"
    ] == [
        [0, 6, 6, 2, 2, 2, 2, 2, 2, 2, 5, 4, 1, 0, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
        [4, 4, 2, 2, 6, 6, 1, 1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_cut_path_source_faces"
    ] == [
        [0, 8, 9, 3, 3, 2, 2, 2, 2, 2, 5, 5, 0, 0, 0, 0],
        [5, 5, 3, 3, 8, 8, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_cut_path_source_face_runs"
    ] == [
        [[0, 1], [6, 2], [2, 7], [5, 1], [4, 1], [1, 1], [0, 3]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [6, 2], [1, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_cut_path_source_face_runs"
    ] == [
        [[0, 1], [8, 1], [9, 1], [3, 2], [2, 5], [5, 2], [0, 4]],
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_cut_path_collapsed"
    ] == [
        [
            True,
            False,
            False,
            True,
            False,
            False,
            True,
            False,
            True,
            True,
            False,
            False,
            False,
            False,
            True,
            False,
        ],
        [False, False, False, False, False, False, False, False],
        [False, False, False, False, False, False, False, False],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_cut_path_collapsed"
    ] == diagnostics["paired_coplanar_combined_first_source_preserving_cut_path_collapsed"]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_cut_path_start_primitive_kinds"
    ] == [
        [1, 1, 2, 2, 1, 2, 1, 1, 2, 2, 2, 1, 1, 1, 2, 2],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_cut_path_start_primitive_kinds"
    ] == [
        [2, 2, 1, 1, 2, 1, 2, 2, 1, 1, 1, 2, 2, 2, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
        [1, 1, 1, 1, 1, 1, 1, 1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_cut_path_start_primitive_faces"
    ] == [
        [-1, -1, 6, 6, -1, 7, -1, -1, 5, 5, 5, -1, -1, -1, 0, 0],
        [-1, -1, -1, -1, -1, -1, -1, -1],
        [-1, -1, -1, -1, -1, -1, -1, -1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_cut_path_start_primitive_faces"
    ] == [
        [8, 8, -1, -1, 3, -1, 2, 2, -1, -1, -1, 10, 10, 10, -1, -1],
        [-1, -1, -1, -1, -1, -1, -1, -1],
        [-1, -1, -1, -1, -1, -1, -1, -1],
    ]
    assert (
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_cut_path_start_primitive_faces"
        ][0]
        != meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"][0][0]
    )
    assert (
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_cut_path_start_primitive_faces"
        ][0]
        != meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"][1][0]
    )
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_order_rotations"
    ] == [4, 4, 4]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_order_rotations"
    ] == [4, 4, 4]
    assert [
        len(path)
        for path in diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_like_cut_edge_paths"
        ]
    ] == diagnostics["paired_coplanar_combined_first_source_preserving_cut_path_lengths"]
    assert [
        len(path)
        for path in diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_like_cut_edge_paths"
        ]
    ] == diagnostics["paired_coplanar_combined_second_source_preserving_cut_path_lengths"]
    first_meshlib_like_rotation = diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_order_rotations"
    ][0]
    second_meshlib_like_rotation = diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_order_rotations"
    ][0]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut_path_start_primitive_faces"
    ] == [
        [-1, 7, -1, -1, 5, 5, 5, -1, -1, -1, 0, 0, -1, -1, 6, 6],
        [-1, -1, -1, -1, -1, -1, -1, -1],
        [-1, -1, -1, -1, -1, -1, -1, -1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_cut_path_start_primitive_faces"
    ] == [
        [3, -1, 2, 2, -1, -1, -1, 10, 10, 10, -1, -1, 8, 8, -1, -1],
        [-1, -1, -1, -1, -1, -1, -1, -1],
        [-1, -1, -1, -1, -1, -1, -1, -1],
    ]
    assert (
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_like_cut_path_start_primitive_faces"
        ][0]
        == meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"][0][0]
    )
    assert (
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_like_cut_path_start_primitive_faces"
        ][0]
        == meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"][1][0]
    )
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut_path_collapsed"
    ] == [
        [
            False,
            False,
            True,
            False,
            True,
            True,
            False,
            False,
            False,
            False,
            True,
            False,
            True,
            False,
            False,
            True,
        ],
        [False, False, False, False, False, False, False, False],
        [False, False, False, False, False, False, False, False],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_cut_path_collapsed"
    ] == diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut_path_collapsed"
    ]
    assert _rotate_list(
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_cut_path_start_primitive_faces"
        ][0],
        first_meshlib_like_rotation,
    ) == meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"][0][0]
    assert _rotate_list(
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_cut_path_start_primitive_faces"
        ][0],
        second_meshlib_like_rotation,
    ) == meshlib_cutmesh_reference["pre_cut_contour_start_primitive_faces"][1][0]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_removed_face_owner_candidates"
    ] == [
        [1, 6, 6, 6, 7, 7, 4, 5, 5, 5, 5, 4, 1, 0, 0, 0],
        [1, 4, 2, 2, 3, 8, 0, 0],
        [1, 4, 2, 2, 2, 9, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_removed_face_owner_candidates"
    ] == [
        [8, 8, 9, 3, 3, 2, 2, 2, 3, 11, 10, 10, 10, 10, 11, 8],
        [0, 5, 2, 3, 3, 8, 0, 0],
        [1, 4, 2, 2, 3, 8, 0, 0],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates"
    ] == [
        [7, 7, 4, 5, 5, 5, 5, 4, 1, 0, 0, 0, 1, 6, 6, 6],
        [3, 8, 0, 0, 1, 4, 2, 2],
        [2, 9, 0, 1, 1, 4, 2, 2],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates"
    ] == [
        [3, 2, 2, 2, 3, 11, 10, 10, 10, 10, 11, 8, 8, 8, 9, 3],
        [3, 8, 0, 0, 0, 5, 2, 3],
        [3, 8, 0, 0, 1, 4, 2, 2],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates"
    ] == [[4, 5, 5, 0, 1, 6], [], []]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates"
    ] == [[2, 3, 11, 11, 8, 3], [], []]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs"
    ] == [[[4, 1], [5, 2], [0, 1], [1, 1], [6, 1]], [], []]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs"
    ] == [[[2, 1], [3, 1], [11, 2], [8, 1], [3, 1]], [], []]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_lifecycle_runs"
    ] == [
        [
            [7, 2, 0, 5],
            [4, 1, 1, 4],
            [5, 4, 2, 9],
            [4, 1, 0, 1],
            [1, 1, 0, 4],
            [0, 3, 1, 7],
            [1, 1, 1, 1],
            [6, 3, 1, 7],
        ],
        [
            [3, 1, 0, 3],
            [8, 1, 0, 3],
            [0, 2, 0, 5],
            [1, 1, 0, 3],
            [4, 1, 0, 3],
            [2, 2, 0, 5],
        ],
        [
            [2, 3, 0, 7],
            [9, 1, 0, 3],
            [0, 1, 0, 3],
            [1, 2, 0, 5],
            [4, 1, 0, 3],
        ],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_lifecycle_runs"
    ] == [
        [
            [3, 2, 1, 6],
            [2, 3, 1, 7],
            [3, 1, 1, 1],
            [11, 1, 1, 4],
            [10, 4, 0, 9],
            [11, 1, 1, 1],
            [8, 3, 1, 7],
            [9, 1, 0, 3],
        ],
        [
            [3, 2, 0, 5],
            [8, 1, 0, 3],
            [0, 3, 0, 7],
            [5, 1, 0, 3],
            [2, 1, 0, 3],
        ],
        [
            [3, 1, 0, 3],
            [8, 1, 0, 3],
            [0, 2, 0, 5],
            [1, 1, 0, 3],
            [4, 1, 0, 3],
            [2, 2, 0, 5],
        ],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs"
    ][0] == [
        [0, 0, 7, 2, 0, 5, 12, 17],
        [0, 1, 4, 1, 1, 4, 17, 21],
        [0, 2, 5, 4, 2, 9, 21, 30],
        [0, 3, 4, 1, 0, 1, 30, 31],
        [0, 4, 1, 1, 0, 4, 31, 35],
        [0, 5, 0, 3, 1, 7, 35, 42],
        [0, 6, 1, 1, 1, 1, 42, 43],
        [0, 7, 6, 3, 1, 7, 43, 50],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs"
    ][0] == [
        [0, 0, 3, 2, 1, 6, 12, 18],
        [0, 1, 2, 3, 1, 7, 18, 25],
        [0, 2, 3, 1, 1, 1, 25, 26],
        [0, 3, 11, 1, 1, 4, 26, 30],
        [0, 4, 10, 4, 0, 9, 30, 39],
        [0, 5, 11, 1, 1, 1, 39, 40],
        [0, 6, 8, 3, 1, 7, 40, 47],
        [0, 7, 9, 1, 0, 3, 47, 50],
    ]
    assert (
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs"
        ][0][-1][-1]
        == len(meshlib_cutmesh_reference["cut2origin_a_values"])
    )
    assert (
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs"
        ][0][-1][-1]
        == len(meshlib_cutmesh_reference["cut2origin_b_values"])
    )
    assert _rotate_list(
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_removed_face_owner_candidates"
        ][0],
        first_meshlib_like_rotation,
    ) == meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"][0]
    assert _rotate_list(
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_removed_face_owner_candidates"
        ][0],
        second_meshlib_like_rotation,
    ) == meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"][0]
    assert (
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates"
        ][0]
        == meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"][0]
    )
    assert (
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates"
        ][0]
        == meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"][0]
    )
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_removed_face_owner_candidate_runs"
    ] == [
        [
            [1, 1],
            [6, 3],
            [7, 2],
            [4, 1],
            [5, 4],
            [4, 1],
            [1, 1],
            [0, 3],
        ],
        [[1, 1], [4, 1], [2, 2], [3, 1], [8, 1], [0, 2]],
        [[1, 1], [4, 1], [2, 3], [9, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_removed_face_owner_candidate_runs"
    ] == [
        [[8, 2], [9, 1], [3, 2], [2, 3], [3, 1], [11, 1], [10, 4], [11, 1], [8, 1]],
        [[0, 1], [5, 1], [2, 1], [3, 2], [8, 1], [0, 2]],
        [[1, 1], [4, 1], [2, 2], [3, 1], [8, 1], [0, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_removed_face_owner_candidate_runs"
    ] == [
        [[7, 2], [4, 1], [5, 4], [4, 1], [1, 1], [0, 3], [1, 1], [6, 3]],
        [[3, 1], [8, 1], [0, 2], [1, 1], [4, 1], [2, 2]],
        [[2, 1], [9, 1], [0, 1], [1, 2], [4, 1], [2, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_removed_face_owner_candidate_runs"
    ] == [
        [[3, 1], [2, 3], [3, 1], [11, 1], [10, 4], [11, 1], [8, 3], [9, 1], [3, 1]],
        [[3, 1], [8, 1], [0, 3], [5, 1], [2, 1], [3, 1]],
        [[3, 1], [8, 1], [0, 2], [1, 1], [4, 1], [2, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_source_faces"
    ][0] == meshlib_cutmesh_reference["cut2origin_a_appended_values"]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_source_faces"
    ][0] == meshlib_cutmesh_reference["cut2origin_b_appended_values"]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_source_face_counts"
    ][0] == _source_face_counts(meshlib_cutmesh_reference["cut2origin_a_appended_values"])
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_source_face_counts"
    ][0] == _source_face_counts(meshlib_cutmesh_reference["cut2origin_b_appended_values"])
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_source_face_runs"
    ][0] == meshlib_cutmesh_reference["cut2origin_a_appended_runs"]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_source_face_runs"
    ][0] == meshlib_cutmesh_reference["cut2origin_b_appended_runs"]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_faces"
    ][0] == meshlib_cutmesh_reference["cut2origin_a_values"]
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_faces"
    ][0] == meshlib_cutmesh_reference["cut2origin_b_values"]
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts"
    ][0] == _source_face_counts(meshlib_cutmesh_reference["cut2origin_a_values"])
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts"
    ][0] == _source_face_counts(meshlib_cutmesh_reference["cut2origin_b_values"])
    assert diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_face_runs"
    ][0] == _source_face_runs(meshlib_cutmesh_reference["cut2origin_a_values"])
    assert diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_face_runs"
    ][0] == _source_face_runs(meshlib_cutmesh_reference["cut2origin_b_values"])
    assert (
        diagnostics[
            "paired_coplanar_combined_source_preserving_meshlib_removed_face_owner_missing_records"
        ]
        == [0, 0]
    )
    assert (
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_removed_face_owner_candidates"
        ][0]
        != meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"][0]
    )
    assert (
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_removed_face_owner_candidates"
        ][0]
        != meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"][0]
    )
    assert (
        diagnostics["paired_coplanar_combined_duplicate_first_path_edge_occurrences"],
        diagnostics["paired_coplanar_combined_duplicate_second_path_edge_occurrences"],
    ) == (8, 8)
    assert diagnostics[
        "paired_coplanar_combined_duplicate_first_path_edge_path_indices"
    ] == [[0, 2], [0, 2], [0, 2], [0, 2], [0, 1], [0, 1], [0, 1], [0, 1]]
    assert diagnostics[
        "paired_coplanar_combined_duplicate_second_path_edge_path_indices"
    ] == [[0, 1], [0, 1], [0, 1], [0, 1], [0, 2], [0, 2], [0, 2], [0, 2]]
    assert diagnostics["paired_coplanar_candidate_first_cut_path_lengths"] == [8, 8]
    assert sum(diagnostics["paired_coplanar_candidate_first_cut_path_lengths"]) == (
        diagnostics["paired_coplanar_cut_trial_first_cut_edges"]
    )
    assert sum(diagnostics["paired_coplanar_candidate_second_cut_path_lengths"]) == (
        diagnostics["paired_coplanar_cut_trial_second_cut_edges"]
    )
    assert diagnostics["paired_coplanar_candidate_second_cut_path_lengths"] == [8, 8]
    assert diagnostics["paired_coplanar_candidate_first_closed_cut_path_lengths"] == [8, 8]
    assert diagnostics["paired_coplanar_candidate_second_closed_cut_path_lengths"] == [8, 8]
    assert diagnostics["paired_coplanar_candidate_first_closed_cut_path_source_faces"] == [
        [4, 5, 2, 3, 9, 8, 0, 1],
        [4, 4, 2, 2, 6, 6, 1, 1],
    ]
    assert diagnostics["paired_coplanar_candidate_second_closed_cut_path_source_faces"] == [
        [5, 5, 3, 3, 8, 8, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_source_face_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [6, 2], [1, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_source_face_runs"
    ] == [
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert (
        diagnostics["paired_coplanar_candidate_first_closed_cut_path_source_face_runs"]
        != meshlib_cutmesh_reference["cut2origin_a_result_cut_old_face_runs"]
    )
    assert (
        diagnostics["paired_coplanar_candidate_second_closed_cut_path_source_face_runs"]
        != meshlib_cutmesh_reference["cut2origin_b_result_cut_old_face_runs"]
    )
    assert diagnostics["paired_coplanar_candidate_meshlib_path_pairs"] == 2
    assert not diagnostics["paired_coplanar_candidate_meshlib_path_count_mismatch"]
    assert diagnostics["paired_coplanar_candidate_meshlib_path_length_mismatches"] == 0
    assert diagnostics["paired_coplanar_candidate_meshlib_path_closed_mismatches"] == 0
    assert diagnostics["paired_coplanar_candidate_meshlib_path_coordinate_mismatches"] == 0
    assert diagnostics["paired_coplanar_candidate_meshlib_path_same_direction_edges"] == 16
    assert diagnostics["paired_coplanar_candidate_meshlib_path_reversed_edges"] == 0
    assert diagnostics["paired_coplanar_candidate_stitch_result_cut_path_lengths"] == [8, 8]
    assert diagnostics["paired_coplanar_candidate_stitch_result_cut_path_lengths"] != [16]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_path_source_faces"
    ] == [
        [4, 5, 2, 3, 9, 8, 0, 1],
        [4, 4, 2, 2, 6, 6, 1, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_path_source_faces"
    ] == [
        [5, 5, 3, 3, 8, 8, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_path_source_face_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [6, 2], [1, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_path_source_face_runs"
    ] == [
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_meshlib_removed_face_owner_candidates"
    ] == [
        [4, 5, 2, 3, 9, 8, 0, 1],
        [7, 7, 7, 7, 9, 9, 6, 6],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_meshlib_removed_face_owner_candidates"
    ] == [
        [10, 10, 11, 11, 11, 11, 10, 10],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_meshlib_removed_face_owner_candidate_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[7, 4], [9, 2], [6, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_meshlib_removed_face_owner_candidate_runs"
    ] == [
        [[10, 2], [11, 4], [10, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_source_faces"
    ] == [
        4,
        5,
        2,
        3,
        9,
        8,
        0,
        1,
        4,
        4,
        2,
        2,
        6,
        6,
        1,
        1,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_source_faces"
    ] == [
        5,
        5,
        3,
        3,
        8,
        8,
        0,
        0,
        4,
        5,
        2,
        3,
        9,
        8,
        0,
        1,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_source_face_runs"
    ] == [
        [4, 1],
        [5, 1],
        [2, 1],
        [3, 1],
        [9, 1],
        [8, 1],
        [0, 1],
        [1, 1],
        [4, 2],
        [2, 2],
        [6, 2],
        [1, 2],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_source_face_runs"
    ] == [
        [5, 2],
        [3, 2],
        [8, 2],
        [0, 2],
        [4, 1],
        [5, 1],
        [2, 1],
        [3, 1],
        [9, 1],
        [8, 1],
        [0, 1],
        [1, 1],
    ]
    assert (
        diagnostics["paired_coplanar_candidate_stitch_result_cut_missing_source_records"]
        == [0, 0]
    )
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_meshlib_removed_face_owner_source_faces"
    ] == [
        4,
        5,
        2,
        3,
        9,
        8,
        0,
        1,
        7,
        7,
        7,
        7,
        9,
        9,
        6,
        6,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_meshlib_removed_face_owner_source_faces"
    ] == [
        10,
        10,
        11,
        11,
        11,
        11,
        10,
        10,
        4,
        5,
        2,
        3,
        9,
        8,
        0,
        1,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_meshlib_removed_face_owner_source_face_runs"
    ] == [
        [4, 1],
        [5, 1],
        [2, 1],
        [3, 1],
        [9, 1],
        [8, 1],
        [0, 1],
        [1, 1],
        [7, 4],
        [9, 2],
        [6, 2],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_meshlib_removed_face_owner_source_face_runs"
    ] == [
        [10, 2],
        [11, 4],
        [10, 2],
        [4, 1],
        [5, 1],
        [2, 1],
        [3, 1],
        [9, 1],
        [8, 1],
        [0, 1],
        [1, 1],
    ]
    assert (
        diagnostics[
            "paired_coplanar_candidate_stitch_result_cut_meshlib_removed_face_owner_missing_records"
        ]
        == [0, 0]
    )
    assert diagnostics[
        "paired_coplanar_candidate_stitch_result_cut_edge_grouped_path_lengths"
    ] == [8, 8]
    assert diagnostics[
        "paired_coplanar_candidate_stitch_result_cut_edge_grouped_path_lengths"
    ] != [16]
    assert (
        diagnostics[
            "paired_coplanar_candidate_stitch_result_cut_edge_grouped_closed_paths"
        ]
        == 2
    )
    assert diagnostics[
        "paired_coplanar_candidate_stitch_result_cut_edge_grouped_path_lengths"
    ] == diagnostics["paired_coplanar_candidate_stitch_result_cut_path_lengths"]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_edge_grouped_path_source_faces"
    ] == [
        [4, 5, 2, 3, 9, 8, 0, 1],
        [4, 4, 2, 2, 6, 6, 1, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_edge_grouped_path_source_faces"
    ] == [
        [5, 5, 3, 3, 8, 8, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_edge_grouped_path_source_face_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [6, 2], [1, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_edge_grouped_path_source_face_runs"
    ] == [
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_edge_grouped_source_faces"
    ] == [
        4,
        5,
        2,
        3,
        9,
        8,
        0,
        1,
        4,
        4,
        2,
        2,
        6,
        6,
        1,
        1,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_edge_grouped_source_faces"
    ] == [
        5,
        5,
        3,
        3,
        8,
        8,
        0,
        0,
        4,
        5,
        2,
        3,
        9,
        8,
        0,
        1,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_stitch_result_cut_edge_grouped_source_face_runs"
    ] == [
        [4, 1],
        [5, 1],
        [2, 1],
        [3, 1],
        [9, 1],
        [8, 1],
        [0, 1],
        [1, 1],
        [4, 2],
        [2, 2],
        [6, 2],
        [1, 2],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_stitch_result_cut_edge_grouped_source_face_runs"
    ] == [
        [5, 2],
        [3, 2],
        [8, 2],
        [0, 2],
        [4, 1],
        [5, 1],
        [2, 1],
        [3, 1],
        [9, 1],
        [8, 1],
        [0, 1],
        [1, 1],
    ]
    assert (
        diagnostics[
            "paired_coplanar_candidate_stitch_result_cut_edge_grouped_missing_source_records"
        ]
        == [0, 0]
    )
    assert (
        len(diagnostics["paired_coplanar_candidate_first_stitch_result_cut_source_faces"]),
        len(diagnostics["paired_coplanar_candidate_second_stitch_result_cut_source_faces"]),
    ) == (
        len(meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"][0]),
        len(meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"][0]),
    )
    assert (
        diagnostics["paired_coplanar_candidate_first_stitch_result_cut_source_faces"]
        != meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"][0]
    )
    assert (
        diagnostics["paired_coplanar_candidate_second_stitch_result_cut_source_faces"]
        != meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"][0]
    )
    assert (
        diagnostics[
            "paired_coplanar_candidate_first_stitch_result_cut_meshlib_removed_face_owner_source_faces"
        ]
        != meshlib_cutmesh_reference["cut2origin_a_result_cut_old_faces"][0]
    )
    assert (
        diagnostics[
            "paired_coplanar_candidate_second_stitch_result_cut_meshlib_removed_face_owner_source_faces"
        ]
        != meshlib_cutmesh_reference["cut2origin_b_result_cut_old_faces"][0]
    )
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_adjacent_source_faces"
    ] == [
        [[4], [5], [2], [3], [9], [8], [0], [1]],
        [[4, 7], [4, 7], [2, 7], [2, 7], [6, 9], [6, 9], [1, 6], [1, 6]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_adjacent_source_faces"
    ] == [
        [[5, 10], [5, 10], [3, 11], [3, 11], [8, 11], [8, 11], [0, 10], [0, 10]],
        [[4], [5], [2], [3], [9], [8], [0], [1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_left_source_faces"
    ] == [
        [[4], [5], [2], [3], [9], [8], [0], [1]],
        [[4], [4], [2], [2], [9], [9], [1], [1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_right_source_faces"
    ] == [
        [[4], [5], [2], [3], [9], [8], [0], [1]],
        [[7], [7], [7], [7], [6], [6], [6], [6]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_left_source_faces"
    ] == [
        [[10], [10], [11], [11], [11], [11], [10], [10]],
        [[4], [5], [2], [3], [9], [8], [0], [1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_right_source_faces"
    ] == [
        [[5], [5], [3], [3], [8], [8], [0], [0]],
        [[4], [5], [2], [3], [9], [8], [0], [1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_left_primary_source_faces"
    ] == [
        [4, 5, 2, 3, 9, 8, 0, 1],
        [4, 4, 2, 2, 9, 9, 1, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_right_primary_source_faces"
    ] == [
        [4, 5, 2, 3, 9, 8, 0, 1],
        [7, 7, 7, 7, 6, 6, 6, 6],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_left_primary_source_faces"
    ] == [
        [10, 10, 11, 11, 11, 11, 10, 10],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_right_primary_source_faces"
    ] == [
        [5, 5, 3, 3, 8, 8, 0, 0],
        [4, 5, 2, 3, 9, 8, 0, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_left_primary_source_face_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[4, 2], [2, 2], [9, 2], [1, 2]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_closed_cut_path_edge_right_primary_source_face_runs"
    ] == [
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
        [[7, 4], [6, 4]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_left_primary_source_face_runs"
    ] == [
        [[10, 2], [11, 4], [10, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_closed_cut_path_edge_right_primary_source_face_runs"
    ] == [
        [[5, 2], [3, 2], [8, 2], [0, 2]],
        [[4, 1], [5, 1], [2, 1], [3, 1], [9, 1], [8, 1], [0, 1], [1, 1]],
    ]
    closed_loop_replacement_faces = sum(
        length - 2
        for length in diagnostics["paired_coplanar_candidate_first_closed_cut_path_lengths"]
    )
    assert closed_loop_replacement_faces == 12
    assert (
        meshlib_in_memory["cut2origin_a_valid"]
        - diagnostics["paired_coplanar_candidate_cut_source_records"][0]
    ) == 14
    assert (
        meshlib_in_memory["cut2origin_a_valid"]
        - diagnostics["paired_coplanar_candidate_replacement_cut_source_records"][0]
    ) == 2
    assert (
        meshlib_in_memory["cut2origin_b_valid"]
        - diagnostics["paired_coplanar_candidate_cut_source_records"][1]
    ) == 14
    assert (
        meshlib_in_memory["cut2origin_b_valid"]
        - diagnostics["paired_coplanar_candidate_replacement_cut_source_records"][1]
    ) == 2
    assert diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_faces"
    ] == [50, 50]
    assert diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_source_records"
    ] == [50, 50]
    assert diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_fill_plans"
    ] == [4, 4]
    assert diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_added_faces"
    ] == [14, 14]
    assert (
        meshlib_in_memory["cut2origin_a_valid"]
        - diagnostics[
            "paired_coplanar_candidate_shadow_repaired_replacement_cut_source_records"
        ][0]
    ) == 0
    assert (
        meshlib_in_memory["cut2origin_b_valid"]
        - diagnostics[
            "paired_coplanar_candidate_shadow_repaired_replacement_cut_source_records"
        ][1]
    ) == 0
    assert (
        meshlib_in_memory["cut2origin_a_valid"]
        - diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_source_records"
        ][0]
    ) == 0
    assert (
        meshlib_in_memory["cut2origin_b_valid"]
        - diagnostics[
            "paired_coplanar_candidate_meshlib_cut2origin_shadow_source_records"
        ][1]
    ) == 0
    assert diagnostics["paired_coplanar_candidate_first_cut_source_face_counts"] == [
        [0, 3],
        [1, 4],
        [2, 4],
        [3, 3],
        [4, 4],
        [5, 3],
        [6, 3],
        [7, 3],
        [8, 3],
        [9, 4],
        [10, 1],
        [11, 1],
    ]
    assert diagnostics["paired_coplanar_candidate_second_cut_source_face_counts"] == [
        [0, 4],
        [1, 3],
        [2, 3],
        [3, 4],
        [4, 3],
        [5, 4],
        [6, 1],
        [7, 1],
        [8, 4],
        [9, 3],
        [10, 3],
        [11, 3],
    ]
    assert diagnostics["paired_coplanar_candidate_first_replacement_cut_source_face_counts"] == [
        [0, 3],
        [1, 4],
        [2, 4],
        [3, 3],
        [4, 10],
        [5, 3],
        [6, 3],
        [7, 9],
        [8, 3],
        [9, 4],
        [10, 1],
        [11, 1],
    ]
    assert diagnostics["paired_coplanar_candidate_second_replacement_cut_source_face_counts"] == [
        [0, 4],
        [1, 3],
        [2, 3],
        [3, 4],
        [4, 9],
        [5, 4],
        [6, 1],
        [7, 1],
        [8, 4],
        [9, 3],
        [10, 9],
        [11, 3],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_shadow_repaired_replacement_cut_source_face_counts"
    ] == diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_face_counts"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_shadow_repaired_replacement_cut_source_face_counts"
    ] == diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_face_counts"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_face_counts"
    ] == [
        [0, 4],
        [1, 4],
        [2, 5],
        [3, 3],
        [4, 10],
        [5, 3],
        [6, 3],
        [7, 9],
        [8, 3],
        [9, 4],
        [10, 1],
        [11, 1],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_face_counts"
    ] == [
        [0, 5],
        [1, 3],
        [2, 4],
        [3, 4],
        [4, 9],
        [5, 4],
        [6, 1],
        [7, 1],
        [8, 4],
        [9, 3],
        [10, 9],
        [11, 3],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_vs_source_preserving_source_face_count_deltas"
    ] == _source_face_count_deltas(
        diagnostics[
            "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_face_counts"
        ],
        diagnostics[
            "paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts"
        ][0],
    )
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_vs_source_preserving_source_face_count_deltas"
    ] == _source_face_count_deltas(
        diagnostics[
            "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_face_counts"
        ],
        diagnostics[
            "paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts"
        ][0],
    )
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_vs_source_preserving_source_face_count_deltas"
    ] == [
        [0, -4],
        [1, -2],
        [2, 4],
        [3, 2],
        [4, 4],
        [5, -7],
        [6, -5],
        [7, 3],
        [8, 2],
        [9, 3],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_vs_source_preserving_source_face_count_deltas"
    ] == [
        [0, 4],
        [1, 2],
        [2, -4],
        [3, -4],
        [4, 8],
        [5, 3],
        [8, -4],
        [9, -1],
        [10, -1],
        [11, -3],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_face_runs"
    ] == _source_face_runs(
        diagnostics["paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_faces"]
    )
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_face_runs"
    ] == _source_face_runs(
        diagnostics["paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_faces"]
    )
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_faces"
    ] == [
        *expected_prepared_cut2origin_prefix,
        0,
        0,
        1,
        1,
        1,
        2,
        2,
        2,
        3,
        3,
        4,
        4,
        4,
        5,
        5,
        6,
        6,
        7,
        7,
        8,
        8,
        9,
        9,
        9,
        2,
        0,
        4,
        4,
        4,
        4,
        4,
        4,
        7,
        7,
        7,
        7,
        7,
        7,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_faces"
    ] == [
        *expected_prepared_cut2origin_prefix,
        0,
        0,
        0,
        1,
        1,
        2,
        2,
        3,
        3,
        3,
        4,
        4,
        5,
        5,
        5,
        8,
        8,
        8,
        9,
        9,
        10,
        10,
        11,
        11,
        2,
        0,
        10,
        10,
        10,
        10,
        10,
        10,
        4,
        4,
        4,
        4,
        4,
        4,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_appended_source_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_faces"
    ][12:]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_appended_source_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_faces"
    ][12:]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_appended_source_face_runs"
    ] == [
        [0, 2],
        [1, 3],
        [2, 3],
        [3, 2],
        [4, 3],
        [5, 2],
        [6, 2],
        [7, 2],
        [8, 2],
        [9, 3],
        [2, 1],
        [0, 1],
        [4, 6],
        [7, 6],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_appended_source_face_runs"
    ] == [
        [0, 3],
        [1, 2],
        [2, 2],
        [3, 3],
        [4, 2],
        [5, 3],
        [8, 3],
        [9, 2],
        [10, 2],
        [11, 2],
        [2, 1],
        [0, 1],
        [10, 6],
        [4, 6],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_appended_source_face_runs"
    ] == _source_face_runs(
        diagnostics[
            "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_appended_source_faces"
        ]
    )
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_appended_source_face_runs"
    ] == _source_face_runs(
        diagnostics[
            "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_appended_source_faces"
        ]
    )
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_appended_source_face_runs"
    ] != meshlib_cutmesh_reference["cut2origin_a_appended_runs"]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_appended_source_face_runs"
    ] != meshlib_cutmesh_reference["cut2origin_b_appended_runs"]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_split_source_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_appended_source_faces"
    ][:24]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_split_source_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_appended_source_faces"
    ][:24]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_cut_event_source_faces"
    ] == [
        0,
        0,
        0,
        1,
        1,
        1,
        1,
        2,
        2,
        2,
        2,
        3,
        3,
        3,
        4,
        4,
        4,
        4,
        5,
        5,
        5,
        6,
        6,
        6,
        7,
        7,
        7,
        8,
        8,
        8,
        9,
        9,
        9,
        9,
        10,
        11,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_cut_event_source_faces"
    ] == [
        0,
        0,
        0,
        0,
        1,
        1,
        1,
        2,
        2,
        2,
        3,
        3,
        3,
        3,
        4,
        4,
        4,
        5,
        5,
        5,
        5,
        6,
        7,
        8,
        8,
        8,
        8,
        9,
        9,
        9,
        10,
        10,
        10,
        11,
        11,
        11,
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_split_event_source_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_cut_event_source_faces"
    ][:-2]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_split_event_source_faces"
    ] == [
        *diagnostics[
            "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_cut_event_source_faces"
        ][:21],
        *diagnostics[
            "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_cut_event_source_faces"
        ][23:],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_prefill_repair_source_faces"
    ] == [2, 0]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_prefill_repair_source_faces"
    ] == [2, 0]
    assert diagnostics[
        "paired_coplanar_candidate_first_shadow_repair_path_lengths"
    ] == [1, 1]
    assert diagnostics[
        "paired_coplanar_candidate_second_shadow_repair_path_lengths"
    ] == [1, 1]
    assert diagnostics["paired_coplanar_candidate_first_shadow_repair_path_edges"] == [
        [[5, 10]],
        [[14, 2]],
    ]
    assert diagnostics["paired_coplanar_candidate_second_shadow_repair_path_edges"] == [
        [[12, 4]],
        [[3, 8]],
    ]
    assert diagnostics["paired_coplanar_candidate_first_shadow_repair_path_source_faces"] == [
        [2],
        [0],
    ]
    assert diagnostics["paired_coplanar_candidate_second_shadow_repair_path_source_faces"] == [
        [2],
        [0],
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_prefill_repair_record_details"
    ] == [[2, 2], [3, 0]]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_prefill_repair_record_details"
    ] == [[2, 2], [3, 0]]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_open_edge_restoration_source_faces"
    ] == [2, 0]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_open_edge_restoration_source_faces"
    ] == [2, 0]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_open_edge_restoration_record_details"
    ] == [[2, 2], [3, 0]]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_open_edge_restoration_record_details"
    ] == [[2, 2], [3, 0]]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_orphan_repair_source_faces"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_orphan_repair_source_faces"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_orphan_repair_record_details"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_orphan_repair_record_details"
    ] == []
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_fill_source_faces"
    ] == [4, 4, 4, 4, 4, 4, 7, 7, 7, 7, 7, 7]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_fill_source_faces"
    ] == [10, 10, 10, 10, 10, 10, 4, 4, 4, 4, 4, 4]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_faces"
    ] != meshlib_in_memory["cut2origin_a_values"]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_faces"
    ] != meshlib_in_memory["cut2origin_b_values"]
    assert diagnostics[
        "paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_ready"
    ] == [True, True]
    assert diagnostics[
        "paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_source_records"
    ] == [50, 50]
    assert diagnostics[
        "paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_matching_source_records"
    ] == [15, 16]
    assert diagnostics[
        "paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_mismatched_source_records"
    ] == [35, 34]
    assert diagnostics[
        "paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_missing_materialized_source_records"
    ] == [0, 0]
    assert diagnostics[
        "paired_coplanar_candidate_meshlib_cut2origin_shadow_owner_remap_extra_materialized_source_records"
    ] == [0, 0]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_cut_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_faces"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_cut_source_records"
    ] == [50, 50]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_cut_unique_source_faces"
    ] == [12, 12]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_cut_duplicate_source_records"
    ] == [38, 38]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_cut_fill_plans"
    ] == diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_fill_plans"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_cut_added_faces"
    ] == diagnostics[
        "paired_coplanar_candidate_shadow_repaired_replacement_cut_added_faces"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_faces"
    ] == diagnostics[
        "paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_faces"
    ][0]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_faces"
    ] == diagnostics[
        "paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_faces"
    ][0]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_faces"
    ] == meshlib_in_memory["cut2origin_a_values"]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_faces"
    ] == meshlib_in_memory["cut2origin_b_values"]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_appended_source_faces"
    ] == meshlib_cutmesh_reference["cut2origin_a_appended_values"]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_appended_source_faces"
    ] == meshlib_cutmesh_reference["cut2origin_b_appended_values"]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_face_counts"
    ] == _source_face_counts(meshlib_in_memory["cut2origin_a_values"])
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_face_counts"
    ] == _source_face_counts(meshlib_in_memory["cut2origin_b_values"])
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_cut_source_face_counts"
    ] == diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_face_counts"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_cut_source_face_counts"
    ] == diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_face_counts"
    ]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_source_face_runs"
    ] == _source_face_runs(meshlib_in_memory["cut2origin_a_values"])
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_source_face_runs"
    ] == _source_face_runs(meshlib_in_memory["cut2origin_b_values"])
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_appended_source_face_runs"
    ] == meshlib_cutmesh_reference["cut2origin_a_appended_runs"]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_appended_source_face_runs"
    ] == meshlib_cutmesh_reference["cut2origin_b_appended_runs"]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_fill_plan_source_faces"
    ] == [0, 0, 0, 6]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_fill_plan_source_faces"
    ] == [10, 10, 10, 8]
    assert diagnostics[
        "paired_coplanar_candidate_first_owner_remapped_shadow_repaired_replacement_fill_plan_added_faces"
    ] == [1, 1, 6, 6]
    assert diagnostics[
        "paired_coplanar_candidate_second_owner_remapped_shadow_repaired_replacement_fill_plan_added_faces"
    ] == [1, 1, 6, 6]
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_mismatch_details"
    ] == _source_face_mismatch_details(
        diagnostics[
            "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_source_faces"
        ],
        meshlib_in_memory["cut2origin_a_values"],
    )
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_mismatch_details"
    ] == _source_face_mismatch_details(
        diagnostics[
            "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_source_faces"
        ],
        meshlib_in_memory["cut2origin_b_values"],
    )
    assert diagnostics[
        "paired_coplanar_candidate_first_meshlib_cut2origin_shadow_owner_remap_mismatch_details"
    ][:4] == [[12, 0, 7], [13, 0, 7], [14, 1, 7], [15, 1, 7]]
    assert diagnostics[
        "paired_coplanar_candidate_second_meshlib_cut2origin_shadow_owner_remap_mismatch_details"
    ][:4] == [[12, 0, 3], [13, 0, 3], [14, 0, 3], [15, 1, 3]]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_fill_plan_source_faces"
    ] == [4, 7]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_fill_plan_source_faces"
    ] == [10, 4]
    assert diagnostics[
        "paired_coplanar_candidate_first_shadow_repaired_replacement_fill_plan_source_faces"
    ] == [2, 0, 4, 7]
    assert diagnostics[
        "paired_coplanar_candidate_second_shadow_repaired_replacement_fill_plan_source_faces"
    ] == [2, 0, 10, 4]
    assert diagnostics[
        "paired_coplanar_candidate_first_replacement_fill_plan_added_faces"
    ] == [6, 6]
    assert diagnostics[
        "paired_coplanar_candidate_second_replacement_fill_plan_added_faces"
    ] == [6, 6]
    assert diagnostics[
        "paired_coplanar_candidate_first_shadow_repaired_replacement_fill_plan_added_faces"
    ] == [1, 1, 6, 6]
    assert diagnostics[
        "paired_coplanar_candidate_second_shadow_repaired_replacement_fill_plan_added_faces"
    ] == [1, 1, 6, 6]
    assert (
        paired_prepared_base_rewrite["exported_mesh_stats"]["vertex_count"],
        paired_prepared_base_rewrite["exported_mesh_stats"]["face_count"],
        paired_prepared_base_rewrite["exported_mesh_stats"]["connected_components"],
        paired_prepared_base_rewrite["exported_mesh_stats"]["boundary_edge_count"],
        paired_prepared_base_rewrite["exported_mesh_health"]["boundary_edge_count"],
        paired_prepared_base_rewrite["exported_mesh_health"]["nonmanifold_edge_count"],
        paired_prepared_base_rewrite["exported_mesh_health"]["is_closed"],
        paired_prepared_base_rewrite["packed_mesh_stats"]["vertex_count"],
        paired_prepared_base_rewrite["packed_mesh_stats"]["face_count"],
        paired_prepared_base_rewrite["packed_mesh_health"]["boundary_edge_count"],
        paired_prepared_base_rewrite["packed_mesh_health"]["nonmanifold_edge_count"],
        paired_prepared_base_rewrite["packed_mesh_health"]["is_closed"],
    ) == (36, 36, 3, 32, 32, 0, False, 36, 36, 32, 0, False)
    assert (
        meshlib_in_memory["mapped_a_faces"]
        - diagnostics["paired_coplanar_candidate_meshlib_base_faces"]
    ) == 8
    assert (
        diagnostics["paired_coplanar_candidate_meshlib_incoming_faces"]
        == meshlib_in_memory["mapped_b_faces"]
    )
    assert (
        diagnostics["paired_coplanar_candidate_meshlib_unstitched_faces"]
        == diagnostics["paired_coplanar_candidate_meshlib_base_faces"]
        + diagnostics["paired_coplanar_candidate_meshlib_incoming_faces"]
    )
    assert np.isclose(
        paired_prepared_base_rewrite["exported_mesh_stats"]["volume_mm3"],
        8.0 / 3.0,
        atol=1e-6,
    )
    assert not np.isclose(
        paired_prepared_base_rewrite["exported_mesh_stats"]["volume_mm3"],
        meshlib_stats.volume_mm3,
        atol=1e-6,
    )
    assert np.isclose(
        paired_prepared_base_rewrite["exported_mesh_stats"]["surface_area_mm2"],
        meshlib_stats.surface_area_mm2,
        atol=1e-6,
    )
    assert np.isclose(rust_stats.volume_mm3, meshlib_stats.volume_mm3, atol=1e-6)
    assert np.isclose(rust_stats.surface_area_mm2, meshlib_stats.surface_area_mm2, atol=1e-6)
    assert (
        rust_stats.vertex_count,
        rust_stats.face_count,
        rust_stats.connected_components,
        rust_stats.boundary_edge_count,
        rust_health.boundary_edge_count,
        rust_health.nonmanifold_edge_count,
        rust_health.is_closed,
    ) == (15, 26, 1, 0, 0, 0, True)
    assert np.allclose(rust_stats.bbox_size, meshlib_stats.bbox_size, atol=1e-6)


def test_meshlib_signed_compare_reference_matches_sdk_summary_for_shifted_cube(tmp_path) -> None:
    pytest.importorskip("meshlib")
    source = cube(size=2.0)
    target = source.copy(vertices=source.vertices + np.array([0.5, 0.0, 0.0]))
    source_path = save_mesh(source, tmp_path / "source.stl")
    target_path = save_mesh(target, tmp_path / "target.stl")

    meshlib_values = meshlib_signed_distance_values(source_path, target_path)
    meshlib_summary = meshlib_signed_distance_summary(meshlib_values)
    sdk_summary = signed_compare_summary(source, target)

    assert meshlib_values.shape == (source.vertex_count,)
    assert np.isclose(meshlib_summary["min_signed_distance_mm"], sdk_summary["min_signed_distance_mm"], atol=1e-6)
    assert np.isclose(meshlib_summary["max_signed_distance_mm"], sdk_summary["max_signed_distance_mm"], atol=1e-6)
    assert np.isclose(meshlib_summary["mean_signed_distance_mm"], sdk_summary["mean_signed_distance_mm"], atol=1e-6)
    assert np.allclose(np.sort(meshlib_values), np.sort(np.abs(meshlib_values)), atol=1e-6)


def test_meshlib_reference_compare_npz_matches_current_artifact_contract(tmp_path) -> None:
    pytest.importorskip("meshlib")
    source = cube(size=2.0)
    target = source.copy(vertices=source.vertices + np.array([0.5, 0.0, 0.0]))
    source_path = save_mesh(source, tmp_path / "source.stl")
    target_path = save_mesh(target, tmp_path / "target.stl")
    artifact_path = meshlib_save_compare_npz(
        source_path,
        target_path,
        tmp_path / "compare.npz",
        other_version_id="target-version",
    )
    values, other_version_id = load_compare_npz(artifact_path)

    assert values.shape == (source.vertex_count,)
    assert other_version_id == "target-version"
    assert np.isclose(float(np.mean(values, dtype=np.float64)), 0.25, atol=1e-6)


def test_operation_metric_goldens_cover_meshlib_reference_cases() -> None:
    golden = load_golden("operation_reference_v1.json")

    assert golden["schema_version"] == 1
    assert set(golden["fixtures"]) == set(OPERATION_FIXTURES)
    assert set(golden["meshlib_operations"]) == {
        "cube_2mm_offset_0_5mm_voxel_0_25mm",
        "cube_2mm_overlap_difference",
        "cube_2mm_overlap_intersection",
        "cube_2mm_overlap_union",
        *JEWELRY_CUTTER_BOOLEAN_GOLDENS,
        "ring_16x8_offset_0_25mm_voxel_0_5mm",
    }
    for fixture_name in OPERATION_FIXTURES:
        _assert_nested_metric_close(
            _operation_metric_payload(_operation_fixture(fixture_name)),
            {key: golden["fixtures"][fixture_name][key] for key in ("mesh", "stats", "health")},
        )


def test_live_meshlib_operation_metrics_match_stored_goldens(tmp_path) -> None:
    pytest.importorskip("meshlib")
    golden = load_golden("operation_reference_v1.json")["meshlib_operations"]
    source = cube(size=2.0)
    target = source.copy(vertices=source.vertices + np.array([1.0, 0.0, 0.0]))
    source_path = save_mesh(source, tmp_path / "source.stl")
    target_path = save_mesh(target, tmp_path / "target.stl")
    ring_source = ring(radial_segments=16, tube_segments=8)
    ring_path = save_mesh(ring_source, tmp_path / "ring.stl")

    offset_path = meshlib_offset_mesh(
        source_path,
        tmp_path / "meshlib_offset.stl",
        offset_mm=0.5,
        voxel_size_mm=0.25,
    )
    _assert_nested_metric_close(
        _operation_metric_payload(load_mesh(offset_path)),
        {key: golden["cube_2mm_offset_0_5mm_voxel_0_25mm"][key] for key in ("mesh", "stats", "health")},
    )

    ring_offset_path = meshlib_offset_mesh(
        ring_path,
        tmp_path / "meshlib_ring_offset.stl",
        offset_mm=0.25,
        voxel_size_mm=0.5,
    )
    _assert_nested_metric_close(
        _operation_metric_payload(load_mesh(ring_offset_path)),
        {key: golden["ring_16x8_offset_0_25mm_voxel_0_5mm"][key] for key in ("mesh", "stats", "health")},
    )

    for operation in ("union", "intersection", "difference"):
        meshlib_path = meshlib_boolean_mesh(
            source_path,
            target_path,
            tmp_path / f"meshlib_{operation}.stl",
            operation=operation,
        )
        _assert_nested_metric_close(
            _operation_metric_payload(load_mesh(meshlib_path)),
            {key: golden[f"cube_2mm_overlap_{operation}"][key] for key in ("mesh", "stats", "health")},
        )

    for operation_key in JEWELRY_CUTTER_BOOLEAN_GOLDENS:
        expected = golden[operation_key]
        ring_source = _operation_source_mesh(expected)
        cutter = _operation_fixture(expected["target_fixture"])
        source_path = save_mesh(ring_source, tmp_path / f"{operation_key}_source.stl")
        target_path = save_mesh(cutter, tmp_path / f"{operation_key}_target.stl")
        meshlib_path = meshlib_boolean_mesh(
            source_path,
            target_path,
            tmp_path / f"meshlib_{operation_key}.stl",
            operation=expected["operation"],
        )
        _assert_nested_metric_close(
            _operation_metric_payload(load_mesh(meshlib_path)),
            {key: expected[key] for key in ("mesh", "stats", "health")},
        )


def test_sdk_voxel_operations_remain_inside_stored_meshlib_operation_envelopes() -> None:
    golden = load_golden("operation_reference_v1.json")["meshlib_operations"]
    source = cube(size=2.0)
    target = source.copy(vertices=source.vertices + np.array([1.0, 0.0, 0.0]))
    source_volume = compute_mesh_stats(source).volume_mm3

    offset = voxel_offset_mesh(source, offset_mm=0.5, voxel_size_mm=0.5, refine=True)
    offset_stats = compute_mesh_stats(offset)
    meshlib_offset = golden["cube_2mm_offset_0_5mm_voxel_0_25mm"]["stats"]

    assert compute_mesh_health(offset).is_closed
    assert offset_stats.volume_mm3 > source_volume
    assert np.allclose(offset_stats.bbox_size, meshlib_offset["bbox_size"], atol=0.1)
    assert np.isclose(offset_stats.volume_mm3, meshlib_offset["volume_mm3"], rtol=0.15, atol=0.75)

    ring_source = ring(radial_segments=16, tube_segments=8)
    ring_source_volume = compute_mesh_stats(ring_source).volume_mm3
    ring_offset = voxel_offset_mesh(ring_source, offset_mm=0.25, voxel_size_mm=0.75, refine=True)
    ring_offset_stats = compute_mesh_stats(ring_offset)
    meshlib_ring_offset = golden["ring_16x8_offset_0_25mm_voxel_0_5mm"]["stats"]

    assert compute_mesh_health(ring_offset).is_closed
    assert ring_offset_stats.volume_mm3 > ring_source_volume
    assert np.allclose(ring_offset_stats.bbox_size, meshlib_ring_offset["bbox_size"], atol=0.3)
    assert np.isclose(ring_offset_stats.volume_mm3, meshlib_ring_offset["volume_mm3"], rtol=0.08, atol=5.0)

    for operation in ("union", "intersection", "difference"):
        sdk_result = voxel_boolean_mesh(source, target, operation=operation, voxel_size_mm=0.5, refine=True)
        sdk_stats = compute_mesh_stats(sdk_result)
        expected_stats = golden[f"cube_2mm_overlap_{operation}"]["stats"]

        assert compute_mesh_health(sdk_result).is_closed
        assert sdk_stats.volume_mm3 > 0.0
        assert np.isclose(sdk_stats.volume_mm3, expected_stats["volume_mm3"], rtol=0.35, atol=0.75)
        if operation == "union":
            assert sdk_stats.volume_mm3 > source_volume
        else:
            assert sdk_stats.volume_mm3 < source_volume

    for operation_key in JEWELRY_CUTTER_BOOLEAN_GOLDENS:
        expected = golden[operation_key]
        tolerances = expected["sdk_tolerances"]
        params = expected["params"]
        if _forced_python_accelerator() and params["voxel_size_mm"] < 0.1:
            continue
        boolean_kwargs: dict[str, Any] = {}
        if "origin_phase" in params:
            boolean_kwargs["origin_phase"] = tuple(params["origin_phase"])
        sdk_result = voxel_boolean_mesh(
            _operation_source_mesh(expected),
            _operation_fixture(expected["target_fixture"]),
            operation=expected["operation"],
            voxel_size_mm=params["voxel_size_mm"],
            refine=True,
            **boolean_kwargs,
        )
        sdk_health = compute_mesh_health(sdk_result)
        sdk_stats = compute_mesh_stats(sdk_result)
        expected_stats = expected["stats"]

        assert sdk_health.is_closed
        assert sdk_health.nonmanifold_edge_count == 0
        assert sdk_health.self_intersections == 0
        assert sdk_stats.volume_mm3 > 0.0
        assert np.allclose(sdk_stats.bbox_size, expected_stats["bbox_size"], atol=tolerances["bbox_size_atol"])
        assert np.isclose(sdk_stats.surface_area_mm2, expected_stats["surface_area_mm2"], rtol=tolerances["surface_area_rtol"])
        assert np.isclose(sdk_stats.volume_mm3, expected_stats["volume_mm3"], rtol=tolerances["volume_rtol"])
