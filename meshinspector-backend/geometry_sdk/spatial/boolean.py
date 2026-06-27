"""Exact mesh boolean wrappers backed by the Rust geometry kernel."""

from __future__ import annotations

from typing import Literal

from geometry_sdk.accelerators import _rust_mesh_ops
from geometry_sdk.types import ExactBooleanMeshResult, MeshDocument


ExactBooleanOperation = Literal[
    "union",
    "intersection",
    "difference",
    "difference_ab",
    "difference_ba",
    "inside_a",
    "inside_b",
    "outside_a",
    "outside_b",
]


def exact_boolean_mesh(
    a: MeshDocument,
    b: MeshDocument,
    *,
    operation: ExactBooleanOperation,
    leaf_size: int = 16,
    epsilon: float = 1e-8,
) -> ExactBooleanMeshResult:
    return _rust_mesh_ops.exact_boolean_mesh(
        a,
        b,
        operation=operation,
        leaf_size=leaf_size,
        epsilon=epsilon,
    )
