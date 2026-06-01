"""Composable brush compatibility wrappers for Rust-owned kernels."""

from __future__ import annotations

from collections.abc import Iterable

from geometry_sdk.accelerators import _rust_brushes
from geometry_sdk.types import BrushOperation, BrushStroke, MeshDocument, RegionEntry


def stroke_influence_weights(mesh: MeshDocument, stroke: BrushStroke):
    return _rust_brushes.brush_stroke_weights(mesh, stroke)


def apply_brush_strokes(mesh: MeshDocument, strokes: Iterable[BrushStroke]) -> MeshDocument:
    return mesh.copy(vertices=_rust_brushes.apply_brush_strokes(mesh, strokes))


def region_brush_masks(
    regions: Iterable[RegionEntry],
    operation: BrushOperation,
    *,
    editable_region_ids: Iterable[str] | None = None,
    protected_region_ids: Iterable[str] | None = None,
    respect_allowed_operations: bool = True,
):
    return _rust_brushes.region_brush_masks(
        regions,
        operation,
        editable_region_ids=editable_region_ids,
        protected_region_ids=protected_region_ids,
        respect_allowed_operations=respect_allowed_operations,
    )


def brush_stroke_from_regions(
    operation: BrushOperation,
    seed_indices,
    regions: Iterable[RegionEntry],
    *,
    amount_mm: float = 0.0,
    falloff_mm: float = 1.5,
    iterations: int = 1,
    strength: float = 0.5,
    editable_region_ids: Iterable[str] | None = None,
    protected_region_ids: Iterable[str] | None = None,
    respect_allowed_operations: bool = True,
) -> BrushStroke:
    mask_indices, protected_indices = region_brush_masks(
        regions,
        operation,
        editable_region_ids=editable_region_ids,
        protected_region_ids=protected_region_ids,
        respect_allowed_operations=respect_allowed_operations,
    )
    return BrushStroke(
        operation,
        seed_indices,
        amount_mm=amount_mm,
        falloff_mm=falloff_mm,
        iterations=iterations,
        strength=strength,
        mask_indices=mask_indices,
        protected_indices=protected_indices,
    )
