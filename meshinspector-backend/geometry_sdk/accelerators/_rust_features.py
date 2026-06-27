from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common


SUPPORTED_FEATURE_KINDS = {"point", "sphere", "line", "plane", "circle", "cylinder", "cone"}


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def feature_pair_measurements(features, pairs) -> list[dict[str, Any]]:
    kernel = _require_core_kernel("feature_pair_measurements")
    normalized = [_normalize_feature(index, feature) for index, feature in enumerate(features)]
    feature_ids = [feature["feature_id"] for feature in normalized]
    kinds = [feature["kind"] for feature in normalized]
    centers = np.asarray([feature["center"] for feature in normalized], dtype=np.float64)
    directions = np.asarray([feature["direction"] for feature in normalized], dtype=np.float64)
    radii = np.asarray([feature["radius"] for feature in normalized], dtype=np.float64)
    lengths = np.asarray([feature["length"] for feature in normalized], dtype=np.float64)
    pair_indices = np.asarray(_pair_indices(normalized, pairs), dtype=np.int64).reshape((-1, 2))
    payload: list[dict[str, Any]] = kernel(feature_ids, kinds, centers, directions, radii, lengths, pair_indices)
    return [_normalize_measurement(row) for row in payload]


def feature_object_descriptors(features, *, infinite_extent_mm: float = 1000.0) -> list[dict[str, Any]]:
    kernel = _require_core_kernel("feature_object_descriptors")
    normalized = [_normalize_feature(index, feature) for index, feature in enumerate(features)]
    feature_ids = [feature["feature_id"] for feature in normalized]
    kinds = [feature["kind"] for feature in normalized]
    centers = np.asarray([feature["center"] for feature in normalized], dtype=np.float64)
    directions = np.asarray([feature["direction"] for feature in normalized], dtype=np.float64)
    radii = np.asarray([feature["radius"] for feature in normalized], dtype=np.float64)
    lengths = np.asarray([feature["length"] for feature in normalized], dtype=np.float64)
    payload: list[dict[str, Any]] = kernel(
        feature_ids,
        kinds,
        centers,
        directions,
        radii,
        lengths,
        float(infinite_extent_mm),
    )
    return [_normalize_feature_object_descriptor(row) for row in payload]


def refine_feature_primitives(
    mesh,
    features,
    *,
    distance_limit_mm: float = 0.1,
    normal_tolerance_degrees: float = 30.0,
    max_iterations: int = 10,
) -> list[dict[str, Any]]:
    kernel = _require_core_kernel("refine_feature_primitives")
    normalized = [_normalize_feature(index, feature) for index, feature in enumerate(features)]
    feature_ids = [feature["feature_id"] for feature in normalized]
    kinds = [feature["kind"] for feature in normalized]
    centers = np.asarray([feature["center"] for feature in normalized], dtype=np.float64)
    directions = np.asarray([feature["direction"] for feature in normalized], dtype=np.float64)
    radii = np.asarray([feature["radius"] for feature in normalized], dtype=np.float64)
    lengths = np.asarray([feature["length"] for feature in normalized], dtype=np.float64)
    payload: list[dict[str, Any]] = kernel(
        np.asarray(mesh.vertices, dtype=np.float64),
        np.asarray(mesh.faces, dtype=np.int64),
        feature_ids,
        kinds,
        centers,
        directions,
        radii,
        lengths,
        float(distance_limit_mm),
        float(normal_tolerance_degrees),
        int(max_iterations),
    )
    return [_normalize_refinement(row) for row in payload]


def _normalize_feature(index: int, feature: Any) -> dict[str, Any]:
    if not isinstance(feature, dict):
        raise ValueError("feature primitives must be dictionaries")
    kind = str(feature.get("kind", "")).lower()
    if kind not in SUPPORTED_FEATURE_KINDS:
        raise ValueError(f"unsupported feature primitive kind {kind!r}")
    feature_id = str(feature.get("feature_id") or feature.get("id") or f"feature_{index}")
    center = _point3(feature.get("center", (0.0, 0.0, 0.0)), "center")
    direction = _point3(feature.get("direction") or feature.get("normal") or (0.0, 0.0, 0.0), "direction")
    radius = float(feature.get("radius", feature.get("radius_mm", 0.0)) or 0.0)
    length = float(feature.get("length", feature.get("length_mm", 0.0)) or 0.0)
    return {
        "feature_id": feature_id,
        "kind": kind,
        "center": center,
        "direction": direction,
        "radius": radius,
        "length": length,
    }


def _normalize_measurement(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "first_index": int(row["first_index"]),
        "second_index": int(row["second_index"]),
        "first_feature_id": str(row["first_feature_id"]),
        "second_feature_id": str(row["second_feature_id"]),
        "first_kind": str(row["first_kind"]),
        "second_kind": str(row["second_kind"]),
        "distance": _normalize_part(row["distance"]),
        "center_distance": _normalize_part(row["center_distance"]),
        "angle": _normalize_part(row["angle"]),
        "intersections": [_normalize_intersection(intersection) for intersection in row.get("intersections", [])],
        "meshlib_reference": str(row["meshlib_reference"]),
    }


def _normalize_refinement(row: dict[str, Any]) -> dict[str, Any]:
    primitive = _normalize_primitive(row["primitive"])
    return {
        "feature_id": str(row["feature_id"]),
        "kind": str(row["kind"]),
        "primitive": primitive,
        "selected_vertex_indices": [int(index) for index in row.get("selected_vertex_indices", [])],
        "selected_count": int(row["selected_count"]),
        "iterations": int(row["iterations"]),
        "converged": bool(row["converged"]),
        "meshlib_reference": str(row["meshlib_reference"]),
    }


def _normalize_feature_object_descriptor(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "feature_id": str(row["feature_id"]),
        "source_kind": str(row["source_kind"]),
        "object_type": str(row["object_type"]),
        "class_name": str(row["class_name"]),
        "class_name_plural": str(row["class_name_plural"]),
        "shared_properties": [
            _normalize_feature_object_property(property)
            for property in row.get("shared_properties", [])
        ],
        "meshlib_reference": str(row["meshlib_reference"]),
    }


def _normalize_feature_object_property(property: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": str(property["name"]),
        "kind": str(property["kind"]),
        "scalar_value": float(property["scalar_value"]) if property.get("scalar_value") is not None else None,
        "vector_value": (
            tuple(float(value) for value in property["vector_value"])
            if property.get("vector_value") is not None
            else None
        ),
    }


def _normalize_primitive(primitive: dict[str, Any]) -> dict[str, Any]:
    direction = primitive.get("direction")
    return {
        "feature_id": str(primitive["feature_id"]),
        "kind": str(primitive["kind"]),
        "center": _point3(primitive["center"], "center"),
        "direction": _point3(direction, "direction") if direction is not None else None,
        "radius": float(primitive.get("radius", primitive.get("radius_mm", 0.0)) or 0.0),
        "length": float(primitive.get("length", primitive.get("length_mm", 0.0)) or 0.0),
        "radius_mm": float(primitive.get("radius", primitive.get("radius_mm", 0.0)) or 0.0),
        "length_mm": float(primitive.get("length", primitive.get("length_mm", 0.0)) or 0.0),
    }


def _pair_indices(features: list[dict[str, Any]], pairs: Any) -> list[tuple[int, int]]:
    id_to_index = {feature["feature_id"]: index for index, feature in enumerate(features)}
    normalized = []
    for pair in pairs:
        if isinstance(pair, dict):
            first = pair.get("first_feature_id") or pair.get("firstFeatureId") or pair.get("a")
            second = pair.get("second_feature_id") or pair.get("secondFeatureId") or pair.get("b")
        else:
            first, second = pair
        normalized.append((_feature_index(first, id_to_index), _feature_index(second, id_to_index)))
    return normalized


def _feature_index(value: Any, id_to_index: dict[str, int]) -> int:
    if isinstance(value, str):
        if value not in id_to_index:
            raise ValueError(f"unknown feature id {value!r}")
        return id_to_index[value]
    index = int(value)
    if index < 0:
        raise ValueError("feature pair indices must be non-negative")
    return index


def _normalize_part(part: dict[str, Any]) -> dict[str, Any]:
    normalized = dict(part)
    for key in ("closest_point_a", "closest_point_b", "point_a", "point_b", "direction_a", "direction_b"):
        if key in normalized and normalized[key] is not None:
            normalized[key] = tuple(float(value) for value in normalized[key])
    for key in ("distance_mm", "angle_radians", "angle_degrees"):
        if key in normalized and normalized[key] is not None:
            normalized[key] = float(normalized[key])
    return normalized


def _normalize_intersection(intersection: dict[str, Any]) -> dict[str, Any]:
    normalized = dict(intersection)
    for key in ("center", "direction", "start_point", "end_point"):
        if key in normalized and normalized[key] is not None:
            normalized[key] = tuple(float(value) for value in normalized[key])
    for key in ("radius_mm", "length_mm"):
        if key in normalized and normalized[key] is not None:
            normalized[key] = float(normalized[key])
    return normalized


def _point3(value: Any, name: str) -> tuple[float, float, float]:
    values = tuple(float(coordinate) for coordinate in value)
    if len(values) != 3:
        raise ValueError(f"{name} must have exactly three coordinates")
    return values
