from __future__ import annotations

import os

import numpy as np
import pytest

from geometry_sdk.accelerators import rust
from geometry_sdk.core.mesh import safe_normalize, vertex_neighbors, vertex_normals
from geometry_sdk.deform.brushes import apply_brush_strokes, brush_stroke_from_regions, region_brush_masks
from geometry_sdk.jewelry.regions import detect_ring_regions
from geometry_sdk.jewelry.ring_measurement import measure_ring
from geometry_sdk.testing.fixtures import ring
from geometry_sdk.types import BrushStroke, MeshDocument


def _python_nearest_distances(vertices: np.ndarray, target_indices: np.ndarray, chunk_size: int = 4096) -> np.ndarray:
    targets = vertices[target_indices]
    distances = np.empty(len(vertices), dtype=np.float64)
    for start in range(0, len(vertices), chunk_size):
        points = vertices[start : start + chunk_size]
        diff = points[:, None, :] - targets[None, :, :]
        distances[start : start + len(points)] = np.sqrt(np.min(np.einsum("ijk,ijk->ij", diff, diff), axis=1))
    return distances


def _brush_strokes(seed: np.ndarray) -> list[BrushStroke]:
    return [
        BrushStroke("thicken", seed, amount_mm=0.18, falloff_mm=2.0),
        BrushStroke("scoop", seed + 8, amount_mm=0.07, falloff_mm=1.5),
        BrushStroke("smooth", seed, falloff_mm=2.0, iterations=2, strength=0.25),
    ]


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


def _python_stroke_weights(mesh: MeshDocument, stroke: BrushStroke) -> np.ndarray:
    weights = _python_falloff_weights(mesh, stroke.seed_indices, stroke.falloff_mm)
    if stroke.mask_indices is not None:
        mask = np.zeros(mesh.vertex_count, dtype=bool)
        mask[np.asarray(stroke.mask_indices, dtype=np.int64)] = True
        weights = np.where(mask, weights, 0.0).astype(np.float32)
    if stroke.protected_indices is not None:
        weights = weights.copy()
        weights[np.asarray(stroke.protected_indices, dtype=np.int64)] = 0.0
    return weights


def _python_offset_with_weights(mesh: MeshDocument, weights: np.ndarray, amount_mm: float) -> MeshDocument:
    displaced = mesh.vertices + _python_outward_directions(mesh) * (float(amount_mm) * weights[:, None])
    return mesh.copy(vertices=displaced)


def _python_smooth_with_weights(mesh: MeshDocument, weights: np.ndarray, *, iterations: int, strength: float) -> MeshDocument:
    vertices = mesh.vertices.copy()
    neighbors = vertex_neighbors(mesh)
    active = np.flatnonzero(weights > 0.02)
    for _ in range(max(1, int(iterations))):
        updated = vertices.copy()
        for index in active:
            neighbor_ids = neighbors[int(index)]
            if not neighbor_ids:
                continue
            neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
            updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * float(strength) * float(weights[index])
        vertices = updated
    return mesh.copy(vertices=vertices)


def _python_apply_brush_strokes(mesh: MeshDocument, strokes: list[BrushStroke]) -> MeshDocument:
    output = mesh
    for stroke in strokes:
        weights = _python_stroke_weights(output, stroke)
        if stroke.operation == "thicken":
            output = _python_offset_with_weights(output, weights, stroke.amount_mm)
        elif stroke.operation == "scoop":
            output = _python_offset_with_weights(output, weights, -stroke.amount_mm)
        else:
            output = _python_smooth_with_weights(
                output,
                weights,
                iterations=stroke.iterations,
                strength=stroke.strength,
            )
    return output


def test_brush_stroke_composition_matches_sequential_local_operations(monkeypatch) -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.arange(0, 8, dtype=np.int32)
    strokes = _brush_strokes(seed)
    sequential = _python_apply_brush_strokes(mesh, strokes)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")

    composed = apply_brush_strokes(mesh, strokes)

    assert np.allclose(composed.vertices, sequential.vertices, atol=1e-10)
    assert np.array_equal(composed.faces, mesh.faces)


def test_empty_brush_stroke_list_returns_copy() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=16, tube_segments=8)

    result = apply_brush_strokes(mesh, [])

    assert result is not mesh
    assert np.array_equal(result.vertices, mesh.vertices)
    assert np.array_equal(result.faces, mesh.faces)


def test_masked_brush_stroke_preserves_unselected_and_protected_vertices(monkeypatch) -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    seed = np.arange(0, 8, dtype=np.int32)
    mask = np.arange(0, 16, dtype=np.int32)
    protected = np.arange(4, 12, dtype=np.int32)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")

    result = apply_brush_strokes(
        mesh,
        [
            BrushStroke(
                "thicken",
                seed,
                amount_mm=0.2,
                falloff_mm=2.0,
                mask_indices=mask,
                protected_indices=protected,
            )
        ],
    )

    editable = np.setdiff1d(mask, protected)
    untouched = np.setdiff1d(np.arange(mesh.vertex_count, dtype=np.int32), editable)
    displacement = np.linalg.norm(result.vertices - mesh.vertices, axis=1)
    assert np.any(displacement[editable] > 0.0)
    assert np.allclose(displacement[untouched], 0.0)


def test_region_brush_masks_follow_allowed_operations() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    regions = detect_ring_regions(mesh, measure_ring(mesh))
    region_map = {region.region_id: region for region in regions}

    mask, protected = region_brush_masks(regions, "scoop")

    assert np.array_equal(mask, np.sort(region_map["inner_band"].vertex_indices))
    assert set(region_map["outer_band"].vertex_indices).issubset(set(protected))
    assert set(region_map["head"].vertex_indices).issubset(set(protected))


def test_region_brush_stroke_constrains_displacement_to_allowed_region(monkeypatch) -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    regions = detect_ring_regions(mesh, measure_ring(mesh))
    inner = next(region for region in regions if region.region_id == "inner_band")
    seed = inner.vertex_indices[:8]
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    stroke = brush_stroke_from_regions("scoop", seed, regions, amount_mm=0.18, falloff_mm=2.0)

    result = apply_brush_strokes(mesh, [stroke])

    displacement = np.linalg.norm(result.vertices - mesh.vertices, axis=1)
    outside_inner = np.setdiff1d(np.arange(mesh.vertex_count, dtype=np.int32), inner.vertex_indices)
    assert np.any(displacement[inner.vertex_indices] > 0.0)
    assert np.allclose(displacement[outside_inner], 0.0)


def test_region_brush_stroke_allows_explicit_override() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=32, tube_segments=12)
    regions = detect_ring_regions(mesh, measure_ring(mesh))
    region_map = {region.region_id: region for region in regions}
    seed = region_map["outer_band"].vertex_indices[:8]

    stroke = brush_stroke_from_regions(
        "thicken",
        seed,
        regions,
        amount_mm=0.18,
        editable_region_ids=["outer_band"],
        protected_region_ids=["head"],
        respect_allowed_operations=False,
    )

    assert np.array_equal(stroke.mask_indices, np.sort(region_map["outer_band"].vertex_indices))
    assert set(region_map["head"].vertex_indices).issubset(set(stroke.protected_indices))


def test_brush_composition_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=48, tube_segments=12)
    seed = np.arange(0, 12, dtype=np.int32)
    strokes = _brush_strokes(seed)
    python_mesh = _python_apply_brush_strokes(mesh, strokes)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = apply_brush_strokes(mesh, strokes)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-8)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_masked_brush_composition_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=48, tube_segments=12)
    seed = np.arange(0, 12, dtype=np.int32)
    mask = np.arange(0, 32, dtype=np.int32)
    protected = np.arange(8, 18, dtype=np.int32)
    strokes = [
        BrushStroke("thicken", seed, amount_mm=0.18, falloff_mm=2.0, mask_indices=mask, protected_indices=protected),
        BrushStroke("smooth", seed, falloff_mm=2.0, iterations=2, strength=0.25, mask_indices=mask, protected_indices=protected),
    ]
    python_mesh = _python_apply_brush_strokes(mesh, strokes)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = apply_brush_strokes(mesh, strokes)

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-8)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)


def test_region_brush_composition_is_rust_owned_and_matches_reference(monkeypatch) -> None:
    if os.getenv("GEOMETRY_SDK_ACCELERATOR", "auto").strip().lower() == "python":
        pytest.skip("forced Python accelerator mode")
    if not rust.available():
        pytest.skip("Rust extension is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2, radial_segments=48, tube_segments=12)
    regions = detect_ring_regions(mesh, measure_ring(mesh))
    inner = next(region for region in regions if region.region_id == "inner_band")
    seed = inner.vertex_indices[:12]
    stroke = brush_stroke_from_regions("scoop", seed, regions, amount_mm=0.12, falloff_mm=2.0)
    python_mesh = _python_apply_brush_strokes(mesh, [stroke])
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    rust_mesh = apply_brush_strokes(mesh, [stroke])

    assert np.allclose(rust_mesh.vertices, python_mesh.vertices, atol=1e-8)
    assert np.array_equal(rust_mesh.faces, python_mesh.faces)
