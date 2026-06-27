from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import DecimateMeshResult, MeshDocument, SubdivideMeshResult


def _require_rust_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def offset_verts_mesh(mesh: MeshDocument, offsets_mm: Any) -> MeshDocument:
    offsets = np.asarray(offsets_mm, dtype=np.float32).reshape(-1)
    if offsets.shape[0] != mesh.vertex_count:
        raise ValueError(f"offset count {offsets.shape[0]} does not match vertex count {mesh.vertex_count}")

    payload = _require_rust_kernel("offset_verts_mesh")(
        mesh.vertices,
        mesh.faces,
        offsets,
    )
    return MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=mesh.unit,
        metadata={
            **mesh.metadata,
            "operation": "offset_verts_mesh",
            "source": "rust_offset_verts",
            "meshlib_reference": "MR::offsetVerts",
            "meshlib_source": "MeshLib/source/MRMesh/MROffsetVerts.*",
        },
    )


def decimate_mesh(
    mesh: MeshDocument,
    *,
    strategy: str = "minimize_error",
    max_error: float = np.finfo(np.float64).max,
    max_edge_len: float | None = None,
    max_bd_shift: float | None = None,
    stabilizer: float = 0.001,
    target_face_count: int | None = None,
    target_face_ratio: float | None = None,
    subdivide_parts: int = 1,
    decimate_between_parts: bool = True,
    not_flippable_edges: Any = None,
    edges_to_collapse: Any = None,
    collapse_near_not_flippable: bool = False,
    angle_weighted_dist_to_plane: bool = False,
    max_deleted_vertices: int = 2_147_483_647,
    max_deleted_faces: int = 2_147_483_647,
    max_triangle_aspect_ratio: float = 20.0,
    critical_tri_aspect_ratio: float | None = None,
    tiny_edge_length: float | None = None,
    max_angle_change: float | None = None,
    touch_near_bd_edges: bool = True,
    touch_bd_verts: bool = True,
    optimize_vertex_pos: bool = True,
    pack_mesh: bool = False,
    region_faces: Any = None,
    vertex_uvs: Any = None,
    vertex_colors: Any = None,
    twin_map: Any = None,
) -> DecimateMeshResult:
    edge_len_limit = np.finfo(np.float64).max if max_edge_len is None else float(max_edge_len)
    boundary_shift_limit = np.finfo(np.float64).max if max_bd_shift is None else float(max_bd_shift)
    critical_aspect_limit = (
        np.finfo(np.float64).max
        if critical_tri_aspect_ratio is None
        else float(critical_tri_aspect_ratio)
    )
    tiny_edge_limit = -1.0 if tiny_edge_length is None else float(tiny_edge_length)
    max_angle_change_limit = -1.0 if max_angle_change is None else float(max_angle_change)
    meshlib_default_half_face_limit = (
        float(max_error) >= float(np.finfo(np.float32).max)
        and edge_len_limit >= float(np.finfo(np.float32).max)
        and int(max_deleted_vertices) >= 2_147_483_647
        and int(max_deleted_faces) >= 2_147_483_647
        and target_face_count is None
        and target_face_ratio is None
    )
    not_flippable_array = (
        None
        if not_flippable_edges is None
        else np.asarray(not_flippable_edges, dtype=np.int64).reshape(-1, 2)
    )
    edges_to_collapse_array = (
        None
        if edges_to_collapse is None
        else np.asarray(edges_to_collapse, dtype=np.int64).reshape(-1, 2)
    )
    region_array = None if region_faces is None else np.asarray(region_faces, dtype=np.int64).reshape(-1)
    vertex_uvs_array = _coerce_vertex_uvs(mesh, vertex_uvs)
    vertex_colors_array = _coerce_vertex_colors(mesh, vertex_colors)
    twin_map_array = _coerce_twin_map(mesh, twin_map)
    payload = _require_rust_kernel("decimate_mesh")(
        mesh.vertices,
        mesh.faces,
        strategy,
        float(max_error),
        edge_len_limit,
        boundary_shift_limit,
        float(stabilizer),
        None if target_face_count is None else int(target_face_count),
        None if target_face_ratio is None else float(target_face_ratio),
        int(subdivide_parts),
        bool(decimate_between_parts),
        bool(angle_weighted_dist_to_plane),
        not_flippable_array,
        bool(collapse_near_not_flippable),
        int(max_deleted_vertices),
        int(max_deleted_faces),
        float(max_triangle_aspect_ratio),
        bool(touch_near_bd_edges),
        bool(touch_bd_verts),
        bool(optimize_vertex_pos),
        bool(pack_mesh),
        region_array,
        vertex_uvs_array,
        vertex_colors_array,
        twin_map_array,
        edges_to_collapse_array,
        critical_aspect_limit,
        tiny_edge_limit,
        max_angle_change_limit,
    )
    metadata = {
        **mesh.metadata,
        "operation": "decimate_mesh",
        "strategy": strategy,
        "max_error": float(max_error),
        "max_edge_len": edge_len_limit,
        "max_bd_shift": boundary_shift_limit,
        "stabilizer": float(stabilizer),
        "target_face_count": None if target_face_count is None else int(target_face_count),
        "target_face_ratio": None if target_face_ratio is None else float(target_face_ratio),
        "subdivide_parts": int(subdivide_parts),
        "decimate_between_parts": bool(decimate_between_parts),
        "region_faces": [] if region_array is None else region_array.tolist(),
        "not_flippable_edges": [] if not_flippable_array is None else not_flippable_array.tolist(),
        "edges_to_collapse": [] if edges_to_collapse_array is None else edges_to_collapse_array.tolist(),
        "twin_map": [] if twin_map_array is None else twin_map_array.reshape(-1, 2, 2).tolist(),
        "remapped_not_flippable_edges": np.asarray(
            payload.get("remapped_not_flippable_edges", []),
            dtype=np.int64,
        )
        .reshape(-1, 2)
        .tolist(),
        "remapped_edges_to_collapse": np.asarray(
            payload.get("remapped_edges_to_collapse", []),
            dtype=np.int64,
        )
        .reshape(-1, 2)
        .tolist(),
        "remapped_twin_map": np.asarray(payload.get("remapped_twin_map", []), dtype=np.int64)
        .reshape(-1, 2, 2)
        .tolist(),
        "collapse_near_not_flippable": bool(collapse_near_not_flippable),
        "angle_weighted_dist_to_plane": bool(angle_weighted_dist_to_plane),
        "max_deleted_vertices": int(max_deleted_vertices),
        "max_deleted_faces": int(max_deleted_faces),
        "meshlib_default_half_face_limit": meshlib_default_half_face_limit,
        "max_triangle_aspect_ratio": float(max_triangle_aspect_ratio),
        "critical_tri_aspect_ratio": critical_aspect_limit,
        "tiny_edge_length": tiny_edge_limit,
        "max_angle_change": max_angle_change_limit,
        "touch_near_bd_edges": bool(touch_near_bd_edges),
        "touch_bd_verts": bool(touch_bd_verts),
        "optimize_vertex_pos": bool(optimize_vertex_pos),
        "pack_mesh": bool(pack_mesh),
        "source": "rust_decimate_mesh_qem"
        if strategy == "minimize_error"
        else "rust_decimate_mesh_shortest_edge_first",
        "meshlib_reference": "MR::decimateMesh",
        "meshlib_source": "MeshLib/source/MRMesh/MRMeshDecimate.*",
        "parity_scope": (
            "DecimateStrategy::MinimizeError QEM, target triangle count/percentage maxDeletedFaces stop controls, "
            "stabilizer, angleWeightedDistToPlane, ShortestEdgeFirst, FaceBitSet region subset, "
            "criticalTriAspectRatio aspect-relaxation guard, "
            "tinyEdgeLength endpoint aspect-bypass guard, "
            "maxAngleChange local Delone flip guard, "
            "notFlippable dynamic remapping with remapped_not_flippable_edges metadata, "
            "edgesToCollapse collapse subset and remapping metadata, "
            "twinMap symmetric validation plus paired same-position collapse, paired maxAngleChange Delone flips, "
            "and collapse/flip/pack remapping metadata, "
            "MeshLib preCollapseVertAttribute-style vertex_uvs and vertex_colors interpolation, "
            "subdivideParts part partitioning, and decimateBetweenParts final pass; "
            "arbitrary preCollapse callbacks and true threaded execution remain open"
        ),
    }
    pre_collapse_attributes: list[str] = []
    if "vertex_uvs" in payload:
        metadata["vertex_uvs"] = (
            np.asarray(payload["vertex_uvs"], dtype=np.float64).reshape(-1, 2).tolist()
        )
        pre_collapse_attributes.append("vertex_uvs")
    if "vertex_colors" in payload:
        metadata["vertex_colors"] = (
            np.asarray(payload["vertex_colors"], dtype=np.uint8).reshape(-1, 4).tolist()
        )
        pre_collapse_attributes.append("vertex_colors")
    if pre_collapse_attributes:
        metadata["pre_collapse_vertex_attributes"] = pre_collapse_attributes
    output_mesh = MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=mesh.unit,
        metadata=metadata,
    )
    return DecimateMeshResult(
        mesh=output_mesh,
        verts_deleted=int(payload["verts_deleted"]),
        faces_deleted=int(payload["faces_deleted"]),
        error_introduced=float(payload["error_introduced"]),
        cancelled=bool(payload["cancelled"]),
    )


def _coerce_vertex_uvs(mesh: MeshDocument, vertex_uvs: Any) -> np.ndarray | None:
    source = mesh.metadata.get("vertex_uvs") if vertex_uvs is None else vertex_uvs
    if source is None:
        return None
    array = np.asarray(source, dtype=np.float64).reshape(-1, 2)
    if array.shape[0] == 0:
        return None
    if array.shape[0] != mesh.vertex_count:
        raise ValueError(f"vertex_uvs count {array.shape[0]} does not match vertex count {mesh.vertex_count}")
    return array


def _coerce_vertex_colors(mesh: MeshDocument, vertex_colors: Any) -> np.ndarray | None:
    source = mesh.metadata.get("vertex_colors") if vertex_colors is None else vertex_colors
    if source is None:
        return None
    array = np.asarray(source, dtype=np.int64)
    if array.size == 0:
        return None
    array = array.reshape((array.shape[0], -1))
    if array.shape[1] == 3:
        alpha = np.full((array.shape[0], 1), 255, dtype=np.int64)
        array = np.concatenate([array, alpha], axis=1)
    if array.shape[1] != 4:
        raise ValueError("vertex_colors must have shape (n, 3) or (n, 4)")
    if array.shape[0] != mesh.vertex_count:
        raise ValueError(f"vertex_colors count {array.shape[0]} does not match vertex count {mesh.vertex_count}")
    if np.any((array < 0) | (array > 255)):
        raise ValueError("vertex_colors values must be in the 0..=255 range")
    return np.ascontiguousarray(array, dtype=np.int64)


def _coerce_twin_map(mesh: MeshDocument, twin_map: Any) -> np.ndarray | None:
    source = mesh.metadata.get("twin_map") if twin_map is None else twin_map
    if source is None:
        return None
    array = np.asarray(source, dtype=np.int64)
    if array.size == 0:
        return None
    if array.size % 4 != 0:
        raise ValueError("twin_map must have shape (n, 2, 2) or (n, 4)")
    array = array.reshape(-1, 4)
    if np.any(array < 0):
        raise ValueError("twin_map must contain non-negative vertex indices")
    if np.any(array >= mesh.vertex_count):
        raise ValueError(f"twin_map contains vertex indices outside vertex count {mesh.vertex_count}")
    for row in array:
        if row[0] == row[1] or row[2] == row[3]:
            raise ValueError("twin_map entries must contain non-degenerate edges")
    return np.ascontiguousarray(array, dtype=np.int64)


def subdivide_mesh(
    mesh: MeshDocument,
    *,
    max_edge_len: float,
    max_edge_splits: int = 1000,
    curvature_priority: float = 0.0,
    region_faces: Any = None,
    not_flippable_edges: Any = None,
    subdivide_border: bool = True,
    max_tri_aspect_ratio: float = 0.0,
    max_splittable_tri_aspect_ratio: float | None = None,
    project_on_original_mesh: bool = False,
    smooth_mode: bool = False,
    min_sharp_dihedral_angle: float = 0.5235987755982989,
    max_deviation_after_flip: float | None = None,
    max_angle_change_after_flip: float | None = None,
    critical_tri_aspect_ratio_flip: float | None = None,
) -> SubdivideMeshResult:
    splittable_limit = (
        np.finfo(np.float64).max
        if max_splittable_tri_aspect_ratio is None
        else float(max_splittable_tri_aspect_ratio)
    )
    not_flippable_array = (
        None
        if not_flippable_edges is None
        else np.asarray(not_flippable_edges, dtype=np.int64).reshape(-1, 2)
    )
    payload = _require_rust_kernel("subdivide_mesh")(
        mesh.vertices,
        mesh.faces,
        float(max_edge_len),
        int(max_edge_splits),
        None if region_faces is None else np.asarray(region_faces, dtype=np.int64).reshape(-1),
        not_flippable_array,
        bool(subdivide_border),
        float(max_tri_aspect_ratio),
        splittable_limit,
        float(curvature_priority),
        bool(project_on_original_mesh),
        bool(smooth_mode),
        float(min_sharp_dihedral_angle),
        None if max_deviation_after_flip is None else float(max_deviation_after_flip),
        None if max_angle_change_after_flip is None else float(max_angle_change_after_flip),
        None if critical_tri_aspect_ratio_flip is None else float(critical_tri_aspect_ratio_flip),
    )
    output_mesh = MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=mesh.unit,
        metadata={
            **mesh.metadata,
            "operation": "subdivide_mesh",
            "max_edge_len": float(max_edge_len),
            "max_edge_splits": int(max_edge_splits),
            "curvature_priority": float(curvature_priority),
            "project_on_original_mesh": bool(project_on_original_mesh),
            "smooth_mode": bool(smooth_mode),
            "min_sharp_dihedral_angle": float(min_sharp_dihedral_angle),
            "max_tri_aspect_ratio": float(max_tri_aspect_ratio),
            "max_splittable_tri_aspect_ratio": splittable_limit,
            "max_deviation_after_flip": None if max_deviation_after_flip is None else float(max_deviation_after_flip),
            "max_angle_change_after_flip": None if max_angle_change_after_flip is None else float(max_angle_change_after_flip),
            "critical_tri_aspect_ratio_flip": None if critical_tri_aspect_ratio_flip is None else float(critical_tri_aspect_ratio_flip),
            "region_faces": [] if region_faces is None else np.asarray(region_faces, dtype=np.int64).reshape(-1).tolist(),
            "not_flippable_edges": [] if not_flippable_array is None else not_flippable_array.tolist(),
            "source": "rust_subdivide_mesh",
            "meshlib_reference": "MR::subdivideMesh",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshSubdivide.*",
        },
    )
    region = np.asarray(payload["region_faces"], dtype=np.int64).reshape(-1)
    return SubdivideMeshResult(
        mesh=output_mesh,
        splits_done=int(payload["splits_done"]),
        region_faces=region,
        region_face_count=int(payload["region_face_count"]),
    )


def make_delone_edge_flips(
    mesh: MeshDocument,
    *,
    num_iters: int = 1,
    region_faces: Any = None,
    max_deviation_after_flip: float | None = None,
    max_angle_change: float | None = None,
    critical_tri_aspect_ratio: float | None = None,
    not_flippable_edges: Any = None,
    vert_region: Any = None,
) -> tuple[MeshDocument, int]:
    region_array = None if region_faces is None else np.asarray(region_faces, dtype=np.int64).reshape(-1)
    not_flippable_array = (
        None
        if not_flippable_edges is None
        else np.asarray(not_flippable_edges, dtype=np.int64).reshape(-1, 2)
    )
    vert_region_array = None if vert_region is None else np.asarray(vert_region, dtype=np.int64).reshape(-1)
    payload = _require_rust_kernel("make_delone_edge_flips")(
        mesh.vertices,
        mesh.faces,
        int(num_iters),
        region_array,
        max_deviation_after_flip,
        max_angle_change,
        critical_tri_aspect_ratio,
        not_flippable_array,
        vert_region_array,
    )
    output_mesh = MeshDocument(
        np.asarray(payload["vertices"], dtype=np.float64).reshape(-1, 3),
        np.asarray(payload["faces"], dtype=np.int64).reshape(-1, 3),
        unit=mesh.unit,
        metadata={
            **mesh.metadata,
            "operation": "make_delone_edge_flips",
            "num_iters": int(num_iters),
            "region_faces": [] if region_array is None else region_array.tolist(),
            "max_deviation_after_flip": max_deviation_after_flip,
            "max_angle_change": max_angle_change,
            "critical_tri_aspect_ratio": critical_tri_aspect_ratio,
            "not_flippable_edges": [] if not_flippable_array is None else not_flippable_array.tolist(),
            "vert_region": [] if vert_region_array is None else vert_region_array.tolist(),
            "region_face_count": int(payload["region_face_count"]),
            "source": "rust_make_delone_edge_flips",
            "meshlib_reference": "MR::makeDeloneEdgeFlips",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshDelone.*",
        },
    )
    return output_mesh, int(payload["flips_done"])
