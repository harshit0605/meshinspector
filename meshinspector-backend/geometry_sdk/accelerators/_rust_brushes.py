from __future__ import annotations

from collections.abc import Iterable
from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import BrushOperation, BrushStroke, MeshDocument, RegionEntry


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _indices(values: Any) -> np.ndarray:
    return np.unique(np.asarray(values, dtype=np.int64).reshape(-1))


def _operation_code(operation: BrushOperation) -> int:
    if operation not in _common.BRUSH_OPERATION_CODES:
        raise ValueError("operation must be 'thicken', 'scoop', or 'smooth'")
    return _common.BRUSH_OPERATION_CODES[operation]


def region_brush_masks(
    regions: Iterable[RegionEntry],
    operation: BrushOperation,
    *,
    editable_region_ids: Iterable[str] | None = None,
    protected_region_ids: Iterable[str] | None = None,
    respect_allowed_operations: bool = True,
) -> tuple[np.ndarray, np.ndarray]:
    kernel = _require_rust_kernel("region_brush_masks")
    region_list = list(regions)
    region_ids: list[str] = []
    vertex_offsets = [0]
    flat_vertex_indices: list[int] = []
    allowed_offsets = [0]
    flat_allowed_operations: list[int] = []

    for region in region_list:
        region_ids.append(str(region.region_id))
        flat_vertex_indices.extend(int(index) for index in np.asarray(region.vertex_indices, dtype=np.int64).reshape(-1))
        vertex_offsets.append(len(flat_vertex_indices))
        flat_allowed_operations.extend(
            _common.BRUSH_OPERATION_CODES[allowed_operation]
            for allowed_operation in region.allowed_operations
            if allowed_operation in _common.BRUSH_OPERATION_CODES
        )
        allowed_offsets.append(len(flat_allowed_operations))

    editable_ids = None if editable_region_ids is None else [str(region_id) for region_id in editable_region_ids]
    protected_ids = None if protected_region_ids is None else [str(region_id) for region_id in protected_region_ids]
    editable, protected = kernel(
        _operation_code(operation),
        region_ids,
        np.asarray(vertex_offsets, dtype=np.int64),
        np.asarray(flat_vertex_indices, dtype=np.int64),
        np.asarray(allowed_offsets, dtype=np.int64),
        np.asarray(flat_allowed_operations, dtype=np.int64),
        editable_ids,
        protected_ids,
        editable_ids is not None,
        protected_ids is not None,
        bool(respect_allowed_operations),
    )
    return (
        np.asarray(editable, dtype=np.int64).reshape(-1),
        np.asarray(protected, dtype=np.int64).reshape(-1),
    )


def brush_stroke_weights(mesh: MeshDocument, stroke: BrushStroke) -> np.ndarray:
    kernel = _require_rust_kernel("brush_stroke_weights")
    mask_indices = np.zeros(0, dtype=np.int64) if stroke.mask_indices is None else _indices(stroke.mask_indices)
    protected_indices = (
        np.zeros(0, dtype=np.int64)
        if stroke.protected_indices is None
        else _indices(stroke.protected_indices)
    )
    weights = kernel(
        mesh.vertices,
        _indices(stroke.seed_indices),
        float(stroke.falloff_mm),
        stroke.mask_indices is not None,
        mask_indices,
        protected_indices,
        3.0,
    )
    return np.asarray(weights, dtype=np.float32).reshape(-1)


def apply_brush_strokes(mesh: MeshDocument, strokes: Iterable[BrushStroke]) -> np.ndarray:
    kernel = _require_rust_kernel("apply_brush_strokes")
    stroke_list = list(strokes)
    if not stroke_list:
        return mesh.vertices.copy()

    operations: list[int] = []
    seed_offsets = [0]
    flat_seed_indices: list[int] = []
    mask_enabled: list[int] = []
    mask_offsets = [0]
    flat_mask_indices: list[int] = []
    protected_offsets = [0]
    flat_protected_indices: list[int] = []
    amounts: list[float] = []
    falloffs: list[float] = []
    iterations: list[int] = []
    strengths: list[float] = []

    for stroke in stroke_list:
        seeds = _indices(stroke.seed_indices)
        if seeds.size == 0:
            raise ValueError("seed_indices must not be empty")
        operations.append(_operation_code(stroke.operation))
        flat_seed_indices.extend(int(seed) for seed in seeds)
        seed_offsets.append(len(flat_seed_indices))

        if stroke.mask_indices is None:
            mask_enabled.append(0)
        else:
            mask_enabled.append(1)
            flat_mask_indices.extend(int(index) for index in _indices(stroke.mask_indices))
        mask_offsets.append(len(flat_mask_indices))

        if stroke.protected_indices is not None:
            flat_protected_indices.extend(int(index) for index in _indices(stroke.protected_indices))
        protected_offsets.append(len(flat_protected_indices))

        amounts.append(float(stroke.amount_mm))
        falloffs.append(float(stroke.falloff_mm))
        iterations.append(max(1, int(stroke.iterations)))
        strengths.append(float(np.clip(stroke.strength, 0.0, 1.0)))

    displaced = kernel(
        mesh.vertices,
        mesh.faces,
        np.asarray(operations, dtype=np.int64),
        np.asarray(seed_offsets, dtype=np.int64),
        np.asarray(flat_seed_indices, dtype=np.int64),
        np.asarray(mask_enabled, dtype=np.int64),
        np.asarray(mask_offsets, dtype=np.int64),
        np.asarray(flat_mask_indices, dtype=np.int64),
        np.asarray(protected_offsets, dtype=np.int64),
        np.asarray(flat_protected_indices, dtype=np.int64),
        np.asarray(amounts, dtype=np.float64),
        np.asarray(falloffs, dtype=np.float64),
        np.asarray(iterations, dtype=np.int64),
        np.asarray(strengths, dtype=np.float64),
        3.0,
    )
    return np.asarray(displaced, dtype=np.float64).reshape(-1, 3)
