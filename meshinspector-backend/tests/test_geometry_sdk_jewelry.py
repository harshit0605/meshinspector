from __future__ import annotations

import numpy as np
import os
import pytest
import trimesh

from geometry_sdk.accelerators import rust
from geometry_sdk.core.mesh import safe_normalize, vertex_neighbors, vertex_normals
from geometry_sdk.deform._distance import nearest_distances
from geometry_sdk.deform.local import falloff_weights, local_scoop, local_thicken, local_thicken_to_minimum, outward_directions, smooth, taubin_smooth
from geometry_sdk.deform.resize import fit_ring_to_diameter, radial_scale, resize_ring
from geometry_sdk.jewelry.regions import detect_ring_regions
from geometry_sdk.jewelry.ring_measurement import closest_ring_size, measure_ring, ring_diameter_for_size
from geometry_sdk.testing.fixtures import ring, ring_with_head
from geometry_sdk.types import MeshDocument


def _python_nearest_distances(vertices: np.ndarray, target_indices: np.ndarray, chunk_size: int = 4096) -> np.ndarray:
    targets = vertices[target_indices]
    distances = np.empty(len(vertices), dtype=np.float64)
    for start in range(0, len(vertices), chunk_size):
        points = vertices[start : start + chunk_size]
        diff = points[:, None, :] - targets[None, :, :]
        distances[start : start + len(points)] = np.sqrt(np.min(np.einsum("ijk,ijk->ij", diff, diff), axis=1))
    return distances


def _python_outward_directions(mesh: MeshDocument) -> np.ndarray:
    vertices = mesh.vertices
    normals = safe_normalize(vertex_normals(mesh))
    center = vertices.mean(axis=0)
    toward_center = safe_normalize(center - vertices)
    return np.where((np.einsum("ij,ij->i", normals, toward_center) >= 0.0)[:, None], -normals, normals)


def _python_falloff_weights(mesh: MeshDocument, seed_indices: np.ndarray, falloff_mm: float) -> np.ndarray:
    seeds = np.unique(np.asarray(seed_indices, dtype=np.int32))
    if seeds.size == 0:
        raise ValueError("seed_indices must not be empty")
    distances = _python_nearest_distances(mesh.vertices, seeds)
    weights = np.exp(-0.5 * np.square(distances / max(falloff_mm, 1e-3)))
    weights[distances > falloff_mm * 3.0] = 0.0
    return weights.astype(np.float32)


def _python_local_offset(mesh: MeshDocument, seed_indices: np.ndarray, amount_mm: float, falloff_mm: float) -> MeshDocument:
    weights = _python_falloff_weights(mesh, seed_indices, falloff_mm)
    displaced = mesh.vertices + _python_outward_directions(mesh) * (amount_mm * weights[:, None])
    return mesh.copy(vertices=displaced)


def _python_local_thicken_to_minimum(
    mesh: MeshDocument,
    seed_indices: np.ndarray,
    thickness_values: np.ndarray,
    *,
    min_target_thickness_mm: float,
    falloff_mm: float,
    deficit_scale: float = 0.75,
) -> MeshDocument:
    weights = _python_falloff_weights(mesh, seed_indices, falloff_mm)
    thickness = np.asarray(thickness_values, dtype=np.float32)
    deficits = np.clip(min_target_thickness_mm - np.nan_to_num(thickness, nan=0.0), 0.0, min_target_thickness_mm)
    displaced = mesh.vertices + _python_outward_directions(mesh) * ((deficits * weights)[:, None] * deficit_scale)
    return mesh.copy(vertices=displaced)


def _python_smooth(
    mesh: MeshDocument,
    *,
    iterations: int = 5,
    strength: float = 0.5,
    seed_indices: np.ndarray | None = None,
    falloff_mm: float = 1.8,
) -> MeshDocument:
    iterations = max(1, int(iterations))
    strength = float(np.clip(strength, 0.0, 1.0))
    if seed_indices is None:
        return _python_taubin_smooth(mesh, iterations=iterations, lamb=strength, nu=-0.53)
    weights = _python_falloff_weights(mesh, seed_indices, falloff_mm)
    vertices = mesh.vertices.copy()
    neighbors = vertex_neighbors(mesh)
    active = np.flatnonzero(weights > 0.02)
    for _ in range(iterations):
        updated = vertices.copy()
        for index in active:
            neighbor_ids = neighbors[int(index)]
            if not neighbor_ids:
                continue
            neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
            updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * strength * float(weights[index])
        vertices = updated
    return mesh.copy(vertices=vertices)


def _python_taubin_smooth(mesh: MeshDocument, *, iterations: int, lamb: float, nu: float) -> MeshDocument:
    vertices = mesh.vertices.copy()
    neighbors = vertex_neighbors(mesh)
    for pass_index in range(max(1, int(iterations))):
        updated = vertices.copy()
        factor = float(np.clip(lamb, 0.0, 1.0)) if pass_index % 2 == 0 else -float(nu)
        for index, neighbor_ids in enumerate(neighbors):
            if not neighbor_ids:
                continue
            neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
            updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * factor
        vertices = updated
    return mesh.copy(vertices=vertices)


def test_ring_measurement_detects_axis_and_inner_diameter() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    measurement = measure_ring(mesh)

    assert measurement.inner_diameter_mm is not None
    assert np.isclose(measurement.inner_diameter_mm, 15.6, atol=1.0)
    assert abs(measurement.ring_axis[1]) > 0.9
    assert measurement.bbox_mm[0] > 19.0


def test_ring_measurement_module_is_rust_owned(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "rust")

    measurement = measure_ring(mesh, axis_override=(0.0, 1.0, 0.0))
    regions = detect_ring_regions(mesh, measurement)
    assert measurement.ring_axis_confidence == 1.0
    assert np.isclose(measurement.inner_diameter_mm, 15.6, atol=1e-3)
    assert {region.region_id for region in regions} == {"inner_band", "outer_band", "head", "gem_seat", "ornament_relief", "unknown"}
    assert ring_diameter_for_size(5.0) == 15.67
    assert closest_ring_size(15.6) == 5.0

    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    assert measure_ring(mesh).inner_diameter_mm == measurement.inner_diameter_mm
    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel measure_ring is required"):
        measure_ring(mesh)
    with pytest.raises(RuntimeError, match="Rust kernel detect_ring_regions is required"):
        detect_ring_regions(mesh, measurement)


def test_regions_cover_every_vertex_once() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    measurement = measure_ring(mesh)
    regions = detect_ring_regions(mesh, measurement)

    covered = np.concatenate([region.vertex_indices for region in regions])
    assert len(covered) == mesh.vertex_count
    assert len(np.unique(covered)) == mesh.vertex_count
    assert {region.region_id for region in regions} == {"inner_band", "outer_band", "head", "gem_seat", "ornament_relief", "unknown"}


def test_region_manifest_advertises_ui_backed_local_edit_operations() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    measurement = measure_ring(mesh)
    region_ops = {region.region_id: set(region.allowed_operations) for region in detect_ring_regions(mesh, measurement)}

    for region_id in ("inner_band", "outer_band", "head", "gem_seat", "ornament_relief"):
        assert "thicken" in region_ops[region_id]
        assert "smooth" in region_ops[region_id]

    assert "scoop" in region_ops["inner_band"]
    assert region_ops["unknown"] == set()


def test_region_manifest_includes_ui_protected_gem_seat_region() -> None:
    mesh = ring_with_head()
    measurement = measure_ring(mesh)
    regions = {region.region_id: region for region in detect_ring_regions(mesh, measurement)}

    assert "gem_seat" in regions
    assert regions["gem_seat"].protected_by_default
    assert regions["gem_seat"].vertex_indices.size > 0
    assert {"thicken", "smooth"}.issubset(set(regions["gem_seat"].allowed_operations))


def test_nearest_distances_are_rust_owned_and_match_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    targets = np.array([0, 7, 23, 64], dtype=np.int64)
    expected = _python_nearest_distances(mesh.vertices, targets)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")

    actual = nearest_distances(mesh.vertices, targets)

    assert np.allclose(actual, expected, atol=1e-12)

    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel nearest_distances_to_indices is required"):
        nearest_distances(mesh.vertices, targets)


def test_radial_scale_changes_ring_radius_not_axis_width() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    scaled = radial_scale(mesh, 1.1, ring_axis=(0.0, 1.0, 0.0))

    original_width_y = np.ptp(mesh.vertices[:, 1])
    scaled_width_y = np.ptp(scaled.vertices[:, 1])
    assert np.isclose(original_width_y, scaled_width_y)
    assert np.ptp(scaled.vertices[:, 0]) > np.ptp(mesh.vertices[:, 0])


def test_resize_module_is_rust_owned(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    scaled = radial_scale(mesh, 1.05, ring_axis=(0.0, 1.0, 0.0))
    resized = resize_ring(mesh, current_size=5.0, target_size=6.0, ring_axis=(0.0, 1.0, 0.0))

    assert scaled.vertices.shape == mesh.vertices.shape
    assert resized.vertices.shape == mesh.vertices.shape

    monkeypatch.setattr(rust._common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel radial_scale_vertices is required"):
        radial_scale(mesh, 1.05, ring_axis=(0.0, 1.0, 0.0))


def test_radial_scale_preserves_requested_vertices() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    preserve = np.array([0, 1, 2, 3], dtype=np.int64)

    unprotected = radial_scale(mesh, 1.2, ring_axis=(0.0, 1.0, 0.0))
    protected = radial_scale(mesh, 1.2, ring_axis=(0.0, 1.0, 0.0), preserve_indices=preserve)
    unprotected_motion = np.linalg.norm(unprotected.vertices[preserve] - mesh.vertices[preserve], axis=1)
    protected_motion = np.linalg.norm(protected.vertices[preserve] - mesh.vertices[preserve], axis=1)

    assert protected.vertices.shape == mesh.vertices.shape
    assert np.all(protected_motion < unprotected_motion)


def test_fit_ring_extreme_ratio_uniformly_scales_to_preserve_head() -> None:
    """Regression for the snake-head deformation.

    Fitting an unscaled miniature up to a real ring size is an extreme ratio that
    leaves the safe preserve band. The fit must then scale the whole piece
    *isotropically* (a similarity transform about the centroid) so a protruding
    head keeps its shape, instead of the old radial-only scale that stretched the
    head across the ring plane (~scale_factor) while leaving its axial extent
    unchanged (~1x).
    """
    mesh = ring_with_head(major_radius=2.5, minor_radius=0.6, head_radius=1.4)
    measurement = measure_ring(mesh)
    measured = measurement.inner_diameter_mm
    target = measured * 3.5  # well outside the [1/1.5, 1.5] safe band
    head = np.arange(len(mesh.vertices) - 6, len(mesh.vertices), dtype=np.int64)

    result = fit_ring_to_diameter(
        mesh,
        measured_diameter_mm=measured,
        target_diameter_mm=target,
        ring_axis=measurement.ring_axis,
        preserve_indices=head,
    )

    assert result.applied_uniform_fallback
    # Every vertex must be the exact isotropic similarity image about the centroid;
    # this proves the head is scaled by the same factor on all three axes (shape
    # preserved), not stretched anisotropically.
    s = result.scale_factor
    center = mesh.vertices.mean(axis=0)
    expected = center + s * (mesh.vertices - center)
    assert np.allclose(result.mesh.vertices, expected, atol=1e-6)

    # The head's internal pairwise distances scale by ~s on every axis.
    head_before = mesh.vertices[head] - mesh.vertices[head].mean(axis=0)
    head_after = result.mesh.vertices[head] - result.mesh.vertices[head].mean(axis=0)
    extent_ratio = np.ptp(head_after, axis=0) / np.ptp(head_before, axis=0)
    assert np.allclose(extent_ratio, s, rtol=1e-5)


def test_local_deformations_keep_topology_and_move_vertices() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    seed = np.array([0, 1, 2, 3], dtype=np.int32)

    thickened = local_thicken(mesh, seed, amount_mm=0.2, falloff_mm=2.0)
    scooped = local_scoop(mesh, seed, depth_mm=0.2, falloff_mm=2.0)
    smoothed = smooth(mesh, iterations=2, strength=0.25, seed_indices=seed)

    for output in (thickened, scooped, smoothed):
        assert output.faces.shape == mesh.faces.shape
        assert output.vertices.shape == mesh.vertices.shape
        assert np.any(np.linalg.norm(output.vertices - mesh.vertices, axis=1) > 0.0)


def test_global_smooth_matches_service_taubin_contract(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    service_mesh = trimesh.Trimesh(vertices=mesh.vertices.copy(), faces=mesh.faces.copy(), process=False)
    trimesh.smoothing.filter_taubin(service_mesh, lamb=0.35, nu=-0.53, iterations=3)
    reference = _python_taubin_smooth(mesh, iterations=3, lamb=0.35, nu=-0.53)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")

    rust_mesh = smooth(mesh, iterations=3, strength=0.35)
    direct_mesh = taubin_smooth(mesh, iterations=3, lamb=0.35, nu=-0.53)

    assert np.allclose(reference.vertices, np.asarray(service_mesh.vertices), atol=1e-10)
    assert np.allclose(rust_mesh.vertices, reference.vertices, atol=1e-9)
    assert np.allclose(direct_mesh.vertices, reference.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, mesh.faces)


def test_local_smooth_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.array([0, 1, 2, 3, 4, 5], dtype=np.int32)
    python_mesh = _python_smooth(mesh, iterations=2, strength=0.35, seed_indices=seed, falloff_mm=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = smooth(mesh, iterations=2, strength=0.35, seed_indices=seed, falloff_mm=2.0)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-9)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_falloff_weights_are_rust_owned_and_match_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.array([0, 7, 13, 23], dtype=np.int32)
    python_weights = _python_falloff_weights(mesh, seed, 2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_weights = falloff_weights(mesh, seed, 2.0)

    assert np.allclose(rust_weights, python_weights, atol=1e-6)


def test_outward_directions_are_rust_owned_and_match_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    python_directions = _python_outward_directions(mesh)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_directions = outward_directions(mesh)

    assert np.allclose(rust_directions, python_directions, atol=1e-10)


def test_local_thicken_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.array([0, 1, 2, 3, 4, 5], dtype=np.int32)
    python_mesh = _python_local_offset(mesh, seed, amount_mm=0.18, falloff_mm=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = local_thicken(mesh, seed, amount_mm=0.18, falloff_mm=2.0)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-8)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_local_thicken_to_minimum_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.array([0, 1, 2, 3, 4, 5], dtype=np.int32)
    thickness = np.linspace(0.25, 1.4, mesh.vertex_count, dtype=np.float32)
    python_mesh = _python_local_thicken_to_minimum(
        mesh,
        seed,
        thickness,
        min_target_thickness_mm=1.0,
        falloff_mm=2.0,
    )
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = local_thicken_to_minimum(
        mesh,
        seed,
        thickness,
        min_target_thickness_mm=1.0,
        falloff_mm=2.0,
    )

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-8)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_local_scoop_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.array([0, 1, 2, 3, 4, 5], dtype=np.int32)
    python_mesh = _python_local_offset(mesh, seed, amount_mm=-0.18, falloff_mm=2.0)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = local_scoop(mesh, seed, depth_mm=0.18, falloff_mm=2.0)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-8)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)
