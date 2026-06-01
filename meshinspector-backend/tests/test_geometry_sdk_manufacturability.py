from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.accelerators import _rust_common
from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.manufacturability import build_recommendations, compute_manufacturability_report
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.thickness import summarize_thickness
from geometry_sdk.core.mesh import bounds, normalize_axis, safe_normalize, vertex_normals
from geometry_sdk.jewelry.hollow import (
    adaptive_hollow_to_weight,
    adaptive_protected_hollow_to_weight,
    apply_drain_holes_voxel,
    drain_hole_cutter_mesh,
    drain_hole_cutters_mesh,
    inward_directions_for_hollow,
    plan_drain_holes,
    protected_hollow_mesh,
    protected_hollow_scale_field,
    service_hollow_mesh,
    service_hollow_voxel_size,
    weighted_inner_offset_preview,
)
from geometry_sdk.materials import grams_to_mm3, material_weight_table, mm3_to_grams
from geometry_sdk.testing.fixtures import cube, open_cube, ring, ring_with_head
from geometry_sdk.types import DrainHolePlan, MeshHealth, RegionEntry
from geometry_sdk.voxel.mesh_ops import voxel_shell_mesh


def _python_nearest_distances(vertices: np.ndarray, target_indices: np.ndarray, chunk_size: int = 4096) -> np.ndarray:
    targets = vertices[target_indices]
    distances = np.empty(len(vertices), dtype=np.float64)
    for start in range(0, len(vertices), chunk_size):
        points = vertices[start : start + chunk_size]
        diff = points[:, None, :] - targets[None, :, :]
        distances[start : start + len(points)] = np.sqrt(np.min(np.einsum("ijk,ijk->ij", diff, diff), axis=1))
    return distances


def _python_region_map(regions):
    return {region.region_id: np.asarray(region.vertex_indices, dtype=np.int32) for region in regions}


def _python_protected_hollow_scale_field(mesh, regions, protect_region_ids, base_thickness_mm):
    scales = np.ones(mesh.vertex_count, dtype=np.float32)
    if mesh.vertex_count == 0 or not regions or not protect_region_ids:
        return scales

    region_map = _python_region_map(regions)
    protected_sets = [
        region_map[region_id]
        for region_id in protect_region_ids
        if region_id in region_map and region_map[region_id].size
    ]
    if not protected_sets:
        return scales

    protected = np.unique(np.concatenate(protected_sets))
    min_hollow_mm = max(base_thickness_mm * 0.18, 0.08)
    min_scale = float(np.clip(min_hollow_mm / max(base_thickness_mm, 1e-6), 0.08, 0.45))
    distances = _python_nearest_distances(mesh.vertices, protected)
    falloff_mm = max(base_thickness_mm * 3.5, 1.5)
    protection = np.exp(-0.5 * np.square(distances / falloff_mm))
    protection[distances > falloff_mm * 2.75] = 0.0
    scales = np.clip(1.0 - 0.92 * protection, min_scale, 1.0).astype(np.float32)
    scales[protected] = min_scale
    return scales


def _python_inward_directions_for_hollow(mesh):
    normals = safe_normalize(vertex_normals(mesh))
    if mesh.vertex_count == 0:
        return normals
    center = mesh.vertices.mean(axis=0)
    toward_center = safe_normalize(center - mesh.vertices)
    outward = np.where((np.einsum("ij,ij->i", normals, toward_center) >= 0.0)[:, None], -normals, normals)
    return -outward


def _python_plan_drain_holes(mesh, regions, ring_axis, *, wall_thickness_mm, hole_diameter_mm=0.8):
    region_map = _python_region_map(regions)
    inner_indices = region_map.get("inner_band")
    if inner_indices is None or inner_indices.size == 0:
        raise ValueError("Drain-hole planning requires inner_band region data")

    vertices = mesh.vertices
    center = vertices.mean(axis=0)
    axis = normalize_axis(ring_axis)
    inner_vertices = vertices[inner_indices]
    centered = inner_vertices - center
    radial_vectors = centered - np.outer(centered @ axis, axis)
    radial_norms = np.linalg.norm(radial_vectors, axis=1)
    valid = radial_norms > 1e-6
    if not np.any(valid):
        raise ValueError("Unable to determine radial directions for drain holes")

    valid_dirs = radial_vectors[valid] / radial_norms[valid][:, None]
    radial_basis = valid_dirs.mean(axis=0)
    if np.linalg.norm(radial_basis) < 1e-6:
        radial_basis = valid_dirs[0]
    radial_basis = normalize_axis(radial_basis)

    def pick_anchor(direction: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        scores = valid_dirs @ direction
        anchor = inner_vertices[valid][int(np.argmax(scores))]
        radial_direction = anchor - center
        radial_direction = radial_direction - axis * np.dot(radial_direction, axis)
        return anchor, normalize_axis(radial_direction)

    bbox_min, bbox_max = bounds(mesh)
    bbox_size = bbox_max - bbox_min
    length = float(np.clip(np.max(bbox_size) * 0.18, max(wall_thickness_mm * 5.0, 3.0), 8.0))
    plans = []
    for basis in (radial_basis, -radial_basis):
        anchor, direction = pick_anchor(basis)
        center_point = anchor + direction * (wall_thickness_mm * 0.55)
        plans.append(
            DrainHolePlan(
                center_mm=(float(center_point[0]), float(center_point[1]), float(center_point[2])),
                direction=(float(direction[0]), float(direction[1]), float(direction[2])),
                radius_mm=float(hole_diameter_mm / 2.0),
                length_mm=length,
            )
        )
    return plans


def _python_drain_hole_cutter_mesh(plan: DrainHolePlan, *, sections: int = 32):
    direction = normalize_axis(plan.direction)
    helper = np.array([0.0, 1.0, 0.0], dtype=np.float64)
    if abs(float(np.dot(direction, helper))) > 0.92:
        helper = np.array([1.0, 0.0, 0.0], dtype=np.float64)
    tangent_u = normalize_axis(np.cross(direction, helper))
    tangent_v = normalize_axis(np.cross(direction, tangent_u))
    center = np.asarray(plan.center_mm, dtype=np.float64)
    half = direction * (plan.length_mm / 2.0)
    start = center - half
    end = center + half

    vertices = []
    for base in (start, end):
        for index in range(sections):
            theta = 2.0 * np.pi * index / sections
            vertices.append(base + plan.radius_mm * (np.cos(theta) * tangent_u + np.sin(theta) * tangent_v))
    start_center = len(vertices)
    vertices.append(start)
    end_center = len(vertices)
    vertices.append(end)

    faces = []
    for index in range(sections):
        nxt = (index + 1) % sections
        a = index
        b = nxt
        c = sections + nxt
        d = sections + index
        faces.extend([(a, b, c), (a, c, d), (start_center, b, a), (end_center, d, c)])
    return np.asarray(vertices, dtype=np.float64), np.asarray(faces, dtype=np.int64)


def test_material_weight_conversions_are_deterministic() -> None:
    assert np.isclose(mm3_to_grams(1000.0, "gold_18k"), 15.58)
    assert np.isclose(grams_to_mm3(15.58, "gold_18k"), 1000.0)
    assert np.isclose(mm3_to_grams(1000.0, "unknown"), 15.58)
    table = material_weight_table(1000.0)
    assert list(table) == ["gold_24k", "gold_22k", "gold_18k", "gold_14k", "gold_10k", "silver_925", "platinum"]
    assert table["gold_18k"].volume_mm3 == 1000.0
    assert table["platinum"].weight_g > table["silver_925"].weight_g


def test_manufacturability_report_marks_clean_ring_ready() -> None:
    report = compute_manufacturability_report(ring(), threshold_mm=0.6)

    assert report.export_ready
    assert report.health_score == 100
    assert report.material_weights["gold_18k"].weight_g > 0.0
    assert report.recommendations == ["Mesh is ready for guided manufacturing workflows."]


def test_manufacturability_report_recommends_repair_for_open_mesh() -> None:
    report = compute_manufacturability_report(open_cube(), threshold_mm=0.6)

    assert not report.export_ready
    assert any("Run auto repair" in recommendation for recommendation in report.recommendations)
    assert report.health.holes_count == 1


def test_recommendations_flag_protected_region_violations() -> None:
    mesh = ring()
    sdk = GeometrySDK()
    measurement = sdk.measure_ring(mesh)
    regions = sdk.detect_ring_regions(mesh, measurement, thickness=np.full(mesh.vertex_count, 0.2, dtype=np.float32), threshold_mm=0.6)
    recommendations = build_recommendations(
        MeshHealth(True, 0, 0, 0, 0, True),
        measurement,
        summarize_thickness(np.full(mesh.vertex_count, 0.2, dtype=np.float32), threshold_mm=0.6),
        regions,
        threshold_mm=0.6,
    )

    assert any("Fix thin regions" in recommendation for recommendation in recommendations)
    assert any("Protected detail regions" in recommendation for recommendation in recommendations)


def test_manufacturability_module_is_rust_owned(monkeypatch) -> None:
    report = compute_manufacturability_report(ring(), threshold_mm=0.6)

    assert report.health_score == 100

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel compute_manufacturability_report is required"):
        compute_manufacturability_report(ring(), threshold_mm=0.6)


def test_protected_hollow_scale_field_preserves_protected_regions() -> None:
    mesh = ring_with_head()
    sdk = GeometrySDK()
    measurement = sdk.measure_ring(mesh)
    regions = sdk.detect_ring_regions(mesh, measurement)
    scales = protected_hollow_scale_field(mesh, regions, ["head", "ornament_relief"], 1.0)
    head_indices = np.concatenate([region.vertex_indices for region in regions if region.region_id in {"head", "ornament_relief"}])

    assert scales.shape == (mesh.vertex_count,)
    assert np.min(scales[head_indices]) < 0.5
    assert np.max(scales) <= 1.0


def test_hollow_planning_module_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    mesh = ring_with_head()
    sdk = GeometrySDK()
    regions = sdk.detect_ring_regions(mesh, sdk.measure_ring(mesh))
    expected_scales = _python_protected_hollow_scale_field(mesh, regions, ["head", "ornament_relief"], 1.0)
    expected_directions = _python_inward_directions_for_hollow(mesh)
    expected_preview_scales = _python_protected_hollow_scale_field(mesh, regions, ["head", "ornament_relief"], 0.8)
    expected_preview_vertices = mesh.vertices + expected_directions * (0.8 * expected_preview_scales)[:, None]

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    assert np.allclose(
        protected_hollow_scale_field(mesh, regions, ["head", "ornament_relief"], 1.0),
        expected_scales,
        atol=1e-6,
    )
    assert np.allclose(inward_directions_for_hollow(mesh), expected_directions, atol=1e-6)
    preview = weighted_inner_offset_preview(mesh, regions, ["head", "ornament_relief"], 0.8)
    assert np.allclose(preview.vertices, expected_preview_vertices, atol=1e-6)

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel protected_hollow_scale_field is required"):
        protected_hollow_scale_field(mesh, regions, ["head"], 1.0)


def test_weighted_inner_offset_preview_moves_vertices_without_changing_topology() -> None:
    mesh = ring_with_head()
    sdk = GeometrySDK()
    measurement = sdk.measure_ring(mesh)
    regions = sdk.detect_ring_regions(mesh, measurement)
    preview = weighted_inner_offset_preview(mesh, regions, ["head"], 0.8)

    assert preview.vertices.shape == mesh.vertices.shape
    assert preview.faces.shape == mesh.faces.shape
    assert np.any(np.linalg.norm(preview.vertices - mesh.vertices, axis=1) > 0.0)


def test_drain_hole_planning_returns_opposing_cylinders() -> None:
    mesh = ring()
    sdk = GeometrySDK()
    measurement = sdk.measure_ring(mesh)
    regions = sdk.detect_ring_regions(mesh, measurement)
    plans = plan_drain_holes(mesh, regions, measurement.ring_axis, wall_thickness_mm=0.8, hole_diameter_mm=1.0)

    assert len(plans) == 2
    assert np.isclose(plans[0].radius_mm, 0.5)
    assert plans[0].length_mm >= 4.0
    assert np.dot(np.asarray(plans[0].direction), np.asarray(plans[1].direction)) < -0.95


def test_drain_hole_planning_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    mesh = ring()
    sdk = GeometrySDK()
    measurement = sdk.measure_ring(mesh)
    regions = sdk.detect_ring_regions(mesh, measurement)
    expected = _python_plan_drain_holes(
        mesh,
        regions,
        measurement.ring_axis,
        wall_thickness_mm=0.8,
        hole_diameter_mm=1.0,
    )

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    actual = plan_drain_holes(mesh, regions, measurement.ring_axis, wall_thickness_mm=0.8, hole_diameter_mm=1.0)
    assert len(actual) == len(expected)
    for actual_plan, expected_plan in zip(actual, expected):
        assert np.allclose(actual_plan.center_mm, expected_plan.center_mm, atol=1e-6)
        assert np.allclose(actual_plan.direction, expected_plan.direction, atol=1e-6)
        assert actual_plan.radius_mm == expected_plan.radius_mm
        assert actual_plan.length_mm == expected_plan.length_mm

    expected_vertices, expected_faces = _python_drain_hole_cutter_mesh(expected[0], sections=16)
    cutter = drain_hole_cutter_mesh(actual[0], sections=16)
    assert np.allclose(cutter.vertices, expected_vertices, atol=1e-6)
    assert np.array_equal(cutter.faces, expected_faces)

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel plan_drain_holes is required"):
        plan_drain_holes(mesh, regions, measurement.ring_axis, wall_thickness_mm=0.8)


def test_drain_hole_cutter_mesh_is_closed_and_aligned() -> None:
    plan = DrainHolePlan(center_mm=(0.0, 0.0, 0.0), direction=(1.0, 0.0, 0.0), radius_mm=0.5, length_mm=4.0)
    cutter = drain_hole_cutter_mesh(plan, sections=16)
    health = compute_mesh_health(cutter)
    stats = compute_mesh_stats(cutter)

    assert health.is_closed
    assert cutter.vertex_count == 34
    assert cutter.face_count == 64
    assert np.isclose(stats.bbox_size[0], 4.0)
    assert np.isclose(stats.bbox_size[1], 1.0)
    assert np.isclose(stats.bbox_size[2], 1.0)


def test_drain_hole_cutters_mesh_combines_plans() -> None:
    plans = [
        DrainHolePlan(center_mm=(0.0, 0.0, 0.0), direction=(1.0, 0.0, 0.0), radius_mm=0.4, length_mm=4.0),
        DrainHolePlan(center_mm=(0.0, 0.0, 0.0), direction=(-1.0, 0.0, 0.0), radius_mm=0.4, length_mm=4.0),
    ]
    cutters = drain_hole_cutters_mesh(plans, sections=12)

    assert compute_mesh_health(cutters).is_closed
    assert cutters.metadata["count"] == 2


def test_apply_drain_holes_voxel_removes_material_from_shell() -> None:
    shell = voxel_shell_mesh(cube(size=4.0), wall_thickness_mm=1.0, voxel_size_mm=0.5)
    before = compute_mesh_stats(shell).volume_mm3
    plan = DrainHolePlan(center_mm=(2.0, 0.0, 0.0), direction=(1.0, 0.0, 0.0), radius_mm=0.65, length_mm=5.0)
    drained = apply_drain_holes_voxel(shell, [plan], voxel_size_mm=0.5, padding_mm=1.0, sections=16)
    after = compute_mesh_stats(drained).volume_mm3

    assert compute_mesh_health(drained).is_closed
    assert after < before


def test_service_hollow_mesh_matches_current_meshlib_service_contract(monkeypatch) -> None:
    source = cube(size=4.0)
    shell = service_hollow_mesh(source, wall_thickness_mm=1.0)
    reference = voxel_shell_mesh(source, wall_thickness_mm=1.0, voxel_size_mm=0.25)

    assert service_hollow_voxel_size(source, wall_thickness_mm=1.0) == pytest.approx(0.25)
    assert np.array_equal(shell.faces, reference.faces)
    assert np.allclose(shell.vertices, reference.vertices)
    assert shell.metadata["wall_thickness_mm"] == pytest.approx(1.0)
    assert shell.metadata["voxel_size_mm"] == pytest.approx(0.25)
    assert compute_mesh_health(shell).is_closed
    assert compute_mesh_stats(shell).volume_mm3 < compute_mesh_stats(source).volume_mm3

    sdk = GeometrySDK()
    sdk_shell = sdk.service_hollow(source, wall_thickness_mm=1.0)
    assert np.array_equal(sdk_shell.faces, reference.faces)
    assert sdk.service_hollow_voxel_size(source, wall_thickness_mm=1.0) == pytest.approx(0.25)

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel service_hollow_mesh is required"):
        service_hollow_mesh(source, wall_thickness_mm=1.0)


def test_adaptive_hollow_to_weight_is_rust_owned(monkeypatch) -> None:
    source = cube(size=4.0)
    midpoint_shell = voxel_shell_mesh(source, wall_thickness_mm=0.8, voxel_size_mm=0.8, padding_mm=1.0)
    target_weight_g = mm3_to_grams(compute_mesh_stats(midpoint_shell).volume_mm3, "silver_925")

    hollowed, report = adaptive_hollow_to_weight(
        source,
        target_weight_g=target_weight_g,
        material="silver_925",
        tolerance_g=0.02,
        min_thickness_mm=0.4,
        max_thickness_mm=1.2,
        max_iterations=1,
        voxel_size_mm=0.8,
        padding_mm=1.0,
    )

    assert report.iterations == 1
    assert report.wall_thickness_mm == pytest.approx(0.8)
    assert report.warning is None
    assert abs(report.achieved_weight_g - target_weight_g) < 0.02
    assert compute_mesh_health(hollowed).is_closed

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel adaptive_hollow_to_weight is required"):
        adaptive_hollow_to_weight(source, target_weight_g=target_weight_g, voxel_size_mm=0.8)


def test_protected_hollow_mesh_and_adaptive_weight_are_rust_owned(monkeypatch) -> None:
    source = cube(size=4.0)
    regions = [
        RegionEntry(
            region_id="head",
            label="Head",
            vertex_indices=np.array([0, 1], dtype=np.int32),
            coverage_pct=25.0,
            protected_by_default=True,
            allowed_operations=[],
        )
    ]

    protected = protected_hollow_mesh(
        source,
        regions,
        ["head"],
        wall_thickness_mm=0.8,
        voxel_size_mm=0.8,
        padding_mm=1.0,
    )
    target_weight_g = mm3_to_grams(compute_mesh_stats(protected).volume_mm3, "silver_925")
    hollowed, report = adaptive_protected_hollow_to_weight(
        source,
        regions,
        ["head"],
        target_weight_g=target_weight_g,
        material="silver_925",
        tolerance_g=0.02,
        min_thickness_mm=0.4,
        max_thickness_mm=1.2,
        max_iterations=1,
        voxel_size_mm=0.8,
        padding_mm=1.0,
    )

    assert compute_mesh_health(protected).is_closed
    assert compute_mesh_stats(protected).volume_mm3 < compute_mesh_stats(source).volume_mm3
    assert report.iterations == 1
    assert report.wall_thickness_mm == pytest.approx(0.8)
    assert report.warning is None
    assert abs(report.achieved_weight_g - target_weight_g) < 0.02
    assert compute_mesh_health(hollowed).is_closed

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel protected_hollow_mesh is required"):
        protected_hollow_mesh(source, regions, ["head"], wall_thickness_mm=0.8, voxel_size_mm=0.8)


def test_engine_exposes_manufacturability_and_hollow_planning() -> None:
    sdk = GeometrySDK()
    mesh = ring()
    report = sdk.manufacturability(mesh)
    plans = sdk.plan_drain_holes(mesh, report.regions, report.ring_measurement.ring_axis, wall_thickness_mm=0.8)
    cutters = sdk.drain_hole_cutters_mesh(plans, sections=12)

    assert report.export_ready
    assert len(plans) == 2
    assert sdk.health(cutters).is_closed
