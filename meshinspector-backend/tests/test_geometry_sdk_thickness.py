from __future__ import annotations

import numpy as np
import pytest

from geometry_sdk import GeometrySDK
from geometry_sdk.accelerators import _rust_common
from geometry_sdk.accelerators import rust
from geometry_sdk.analysis.thickness import (
    insphere_thickness_at_vertices,
    ray_thickness_at_vertices,
    service_thickness_at_vertices,
    summarize_thickness,
)
from geometry_sdk.testing.fixtures import cube, ring


def test_ray_thickness_returns_finite_values_for_closed_cube() -> None:
    mesh = cube(size=2.0)
    thickness = ray_thickness_at_vertices(mesh)
    summary = summarize_thickness(thickness, threshold_mm=10.0)

    assert thickness.shape == (mesh.vertex_count,)
    assert np.all(np.isfinite(thickness))
    assert np.all(thickness > 0.0)
    assert summary.valid_vertex_count == mesh.vertex_count
    assert summary.violation_count == mesh.vertex_count


def test_ray_thickness_handles_ring_fixture() -> None:
    mesh = ring(major_radius=9.0, minor_radius=1.2)
    thickness = ray_thickness_at_vertices(mesh)
    summary = summarize_thickness(thickness, threshold_mm=0.6)

    assert np.count_nonzero(np.isfinite(thickness)) > mesh.vertex_count * 0.8
    assert summary.min_mm is not None
    assert summary.min_mm > 0.0


def test_service_thickness_combines_insphere_and_ray_fields() -> None:
    mesh = cube(size=2.0)
    ray = ray_thickness_at_vertices(mesh)
    insphere = insphere_thickness_at_vertices(mesh, max_radius=0.5)
    combined = service_thickness_at_vertices(mesh, max_radius=0.5)

    assert combined.shape == (mesh.vertex_count,)
    assert np.all(np.isfinite(insphere))
    assert np.all(np.isfinite(combined))
    assert np.all(combined > 0.0)
    assert np.all(combined <= np.nan_to_num(ray, nan=np.inf) + 1e-6)
    assert np.all(combined <= insphere + 1e-6)
    assert np.nanmax(combined) <= 1.0 + 1e-6


def test_engine_exposes_ray_thickness_summary() -> None:
    sdk = GeometrySDK()
    field, summary = sdk.ray_thickness(cube(size=2.0), threshold_mm=0.6)

    assert field.shape == (8,)
    assert summary.valid_vertex_count == 8


def test_engine_exposes_service_thickness_summary() -> None:
    sdk = GeometrySDK()
    field, summary = sdk.service_thickness(cube(size=2.0), threshold_mm=0.6)

    assert field.shape == (8,)
    assert summary.valid_vertex_count == 8


def test_thickness_module_is_rust_owned(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    mesh = ring(major_radius=9.0, minor_radius=1.2)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    values = ray_thickness_at_vertices(mesh)
    combined = service_thickness_at_vertices(mesh)
    summary = summarize_thickness(values, threshold_mm=0.6)

    assert values.shape == (mesh.vertex_count,)
    assert combined.shape == (mesh.vertex_count,)
    assert summary.valid_vertex_count > 0

    monkeypatch.setattr(_rust_common, "_rs", None)
    with pytest.raises(RuntimeError, match="Rust kernel ray_thickness_at_vertices is required"):
        ray_thickness_at_vertices(mesh)
    with pytest.raises(RuntimeError, match="Rust kernel service_thickness_at_vertices is required"):
        service_thickness_at_vertices(mesh)


def test_thickness_summary_uses_rust_kernel(monkeypatch) -> None:
    if not rust.available():
        pytest.skip("Rust geometry accelerator is not installed")

    values = np.asarray([2.0, np.nan, 0.25, -1.0, 0.75, np.inf], dtype=np.float32)
    monkeypatch.setenv("GEOMETRY_SDK_ACCELERATOR", "python")
    summary = summarize_thickness(values, threshold_mm=0.6)

    assert summary.min_mm == 0.25
    assert summary.max_mm == 2.0
    assert summary.valid_vertex_count == 3
    assert summary.violation_count == 1
