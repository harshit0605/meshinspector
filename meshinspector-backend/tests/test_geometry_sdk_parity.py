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
        assert diagnostics["meshlib_topology_open_stitch_paths"] == 5
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
            (20, 24, 0, 20, 16, 0, 0, 0, 40, 0)
            if operation == "union"
            else (16, 24, 8, 16, 16, 0, 0, 0, 32, 0)
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
        expected_target_left_closures = 8 if operation == "union" else 16
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
        expected_mapped_replays = 8 if operation == "union" else 0
        expected_skipped_replays = 24 if operation == "union" else 32
        assert prepared_base_rewrite["mapped_source_record_replays"] == expected_mapped_replays
        assert (
            prepared_base_rewrite[
                "mapped_source_record_replays_on_near_stitch_targets"
            ]
            == expected_mapped_replays
        )
        assert prepared_base_rewrite["mapped_source_record_replay_attempts"] == 32
        assert (
            prepared_base_rewrite[
                "mapped_source_record_replay_attempts_on_near_stitch_targets"
            ]
            == 32
        )
        assert (
            prepared_base_rewrite["skipped_mapped_source_record_replays"]
            == expected_skipped_replays
        )
        _assert_mapped_source_record_replay_details(
            prepared_base_rewrite["mapped_source_record_replay_details"],
            expected_attempts=32,
            expected_applied=expected_mapped_replays,
        )
        expected_prepared_base_buckets = (
            (0, 0, 0, 0, 44, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            if operation == "union"
            else (0, 0, 0, 0, 32, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
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
        assert all(
            len(detail["left_ring_edge_ids"])
            == len(detail["left_ring_record_next_edge_ids"])
            == len(detail["left_ring_record_prev_edge_ids"])
            == len(detail["left_ring_origins"])
            == len(detail["left_ring_left_faces"])
            == len(detail["left_ring_right_faces"])
            == len(detail["left_ring_next_edge_ids"])
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
        assert diagnostics["meshlib_topology_open_stitch_near_edge_updates"] == 10
        assert diagnostics["meshlib_topology_open_stitch_near_edge_blocked_updates"] == 0
        assert diagnostics["meshlib_topology_open_stitch_near_edge_ready"]
        assert diagnostics["meshlib_topology_near_stitch_update_commands"] == 10
        expected_applied = 8 if operation == "union" else 1
        expected_failed = 10 - expected_applied
        assert diagnostics["meshlib_topology_near_stitch_updates_applied"] == expected_applied
        assert diagnostics["meshlib_topology_near_stitch_updates_failed"] == expected_failed
        if operation == "union":
            assert diagnostics["meshlib_topology_near_stitch_updates_failed_start"] == 1
            assert diagnostics["meshlib_topology_near_stitch_updates_failed_end"] == 1
        else:
            assert diagnostics["meshlib_topology_near_stitch_updates_failed_start"] == 5
            assert diagnostics["meshlib_topology_near_stitch_updates_failed_end"] == 4
        assert diagnostics["meshlib_topology_near_stitch_updates_missing_previous_edges"] == 0
        assert diagnostics["meshlib_topology_near_stitch_updates_missing_next_edges"] == 0
        if operation == "union":
            assert diagnostics["meshlib_topology_near_stitch_updates_origin_mismatches"] == 0
            assert diagnostics["meshlib_topology_near_stitch_updates_previous_left_faces"] == 1
            assert diagnostics["meshlib_topology_near_stitch_updates_next_right_faces"] == 1
        else:
            assert diagnostics["meshlib_topology_near_stitch_updates_origin_mismatches"] == 0
            assert diagnostics["meshlib_topology_near_stitch_updates_previous_left_faces"] == 9
            assert diagnostics["meshlib_topology_near_stitch_updates_next_right_faces"] == 0
        assert diagnostics["meshlib_topology_near_stitch_updates_failed_other"] == 0
        top_failed_details = diagnostics["meshlib_topology_near_stitch_failed_details"]
        assert len(top_failed_details) == expected_failed
        assert {detail["endpoint"] for detail in top_failed_details} <= {"start", "end"}
        assert all(detail["error"] for detail in top_failed_details)
        assert all(
            detail["candidate_diagnostics"] is not None
            and detail["candidate_diagnostics"]["previous_candidates"] >= 1
            and detail["candidate_diagnostics"]["next_candidates"] >= 1
            and detail["candidate_diagnostics"]["failures"]
            for detail in top_failed_details
        )
        _assert_near_stitch_candidate_ring_details(top_failed_details)
        assert diagnostics["is_closed"]
        assert diagnostics["boundary_edge_count"] == 0
        assert diagnostics["nonmanifold_edge_count"] == 0
        assert np.isclose(rust_stats.volume_mm3, meshlib_stats.volume_mm3, atol=1e-6)
        assert np.isclose(rust_stats.surface_area_mm2, meshlib_stats.surface_area_mm2, atol=1e-6)
        assert np.allclose(rust_stats.bbox_size, meshlib_stats.bbox_size, atol=1e-6)


def test_rust_exact_boolean_binding_tracks_meshlib_difference_gap(tmp_path) -> None:
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

    assert meshlib_health.is_closed
    assert not diagnostics["parity_ready"]
    assert not diagnostics["stitch_compatible"]
    assert not diagnostics["is_closed"]
    assert diagnostics["boundary_edge_count"] == 8
    assert diagnostics["nonmanifold_edge_count"] == 8
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
    ) == (22, 20, 0, 22, 20, 0, 0, 0, 32, 4)
    assert not prepared_base_rewrite["ready_for_export"]
    record_rewrite_details = prepared_base_rewrite["record_rewrite_target_details"]
    _assert_record_rewrite_target_details(
        record_rewrite_details,
        expected_applied=prepared_base_rewrite["applied_commands"],
    )
    assert len(record_rewrite_details) == 20
    assert sum(
        1 for detail in record_rewrite_details if detail["target_was_near_stitch_target"]
    ) == 20
    assert prepared_base_rewrite["record_rewrite_near_stitch_target_left_closures"] == 15
    assert prepared_base_rewrite["record_rewrite_near_stitch_target_right_closures"] == 0
    assert any(
        detail["target_edge_id"] == 63
        and detail["target_was_near_stitch_target"]
        and detail["target_left_before"] is None
        and detail["target_left_after"] == 24
        and detail["target_next_edge_id_before"] == 8
        and detail["target_next_edge_id_after"] == 77
        and detail["record_next_edge_id"] == 77
        and detail["record_sym_prev_edge_id"] == 101
        for detail in record_rewrite_details
    )
    assert any(
        detail["target_edge_id"] == 45
        and detail["target_was_near_stitch_target"]
        and detail["target_left_before"] is None
        and detail["target_left_after"] == 28
        and detail["record_next_edge_id"] == 107
        and detail["record_sym_prev_edge_id"] == 105
        for detail in record_rewrite_details
    )
    assert prepared_base_rewrite["mapped_source_record_replays"] == 9
    assert (
        prepared_base_rewrite["mapped_source_record_replays_on_near_stitch_targets"]
        == 9
    )
    assert prepared_base_rewrite["mapped_source_record_replay_attempts"] == 30
    assert (
        prepared_base_rewrite[
            "mapped_source_record_replay_attempts_on_near_stitch_targets"
        ]
        == 30
    )
    assert prepared_base_rewrite["skipped_mapped_source_record_replays"] == 21
    replay_details = prepared_base_rewrite["mapped_source_record_replay_details"]
    _assert_mapped_source_record_replay_details(
        replay_details,
        expected_attempts=30,
        expected_applied=9,
    )
    assert any(
        detail["target_edge_id"] == 63
        and detail["target_was_near_stitch_target"]
        and detail["target_origin_before"] == 3
        and detail["target_left_before"] == 24
        and detail["target_right_before"] == 15
        and not detail["applied"]
        and detail["skipped_reason"] == "target already has left face"
        for detail in replay_details
    )
    assert any(
        detail["target_edge_id"] == 44
        and detail["target_was_near_stitch_target"]
        and detail["target_origin_before"] == 4
        and detail["target_left_before"] == 10
        and detail["target_right_before"] == 28
        and not detail["applied"]
        and detail["skipped_reason"] == "target already has left face"
        for detail in replay_details
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
    ) == (0, 0, 0, 0, 22, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0)
    assert (
        prepared_base_rewrite["near_stitch_skipped_previous_left_source_edges"],
        prepared_base_rewrite["near_stitch_skipped_next_right_source_edges"],
    ) == (1, 0)
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
    assert all(
        len(detail["left_ring_edge_ids"])
        == len(detail["left_ring_record_next_edge_ids"])
        == len(detail["left_ring_record_prev_edge_ids"])
        == len(detail["left_ring_origins"])
        == len(detail["left_ring_left_faces"])
        == len(detail["left_ring_right_faces"])
        == len(detail["left_ring_next_edge_ids"])
        for detail in export_failed_details
    )
    assert [detail["left_ring_repeated_edge_id"] for detail in export_failed_details] == [
        12,
        50,
        8,
        44,
    ]
    assert not any(detail["left_ring_returned_to_start"] for detail in export_failed_details)
    failed_details = prepared_base_rewrite["near_stitch_failed_details"]
    assert failed_details == []
    assert diagnostics["paired_coplanar_candidate_stitch_compatible"]
    assert diagnostics["paired_coplanar_candidate_first_prepare_part_dividable"]
    assert diagnostics["paired_coplanar_candidate_second_prepare_part_dividable"]
    assert diagnostics["paired_coplanar_candidate_first_cut_path_side_components"] == [1, 1]
    assert diagnostics["paired_coplanar_candidate_second_cut_path_side_components"] == [1, 1]
    assert not diagnostics["paired_coplanar_candidate_result_cut_paths_complete"]
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
    assert diagnostics["paired_coplanar_candidate_self_intersections"] == 2
    assert diagnostics["paired_coplanar_candidate_boundary_edges"] == 0
    assert diagnostics["paired_coplanar_candidate_nonmanifold_edges"] == 0
    assert diagnostics["paired_coplanar_candidate_duplicate_output_faces"] == 0
    assert np.isclose(rust_stats.volume_mm3, meshlib_stats.volume_mm3, atol=1e-6)
    assert np.isclose(rust_stats.surface_area_mm2, meshlib_stats.surface_area_mm2, atol=1e-6)
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
