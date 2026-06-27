"""Mesh edit and simplification operations backed by Rust kernels."""

from typing import Any

from geometry_sdk.accelerators import _rust_mesh_edit
from geometry_sdk.types import DecimateMeshResult, MeshDocument, SubdivideMeshResult


def offset_verts_mesh(mesh: MeshDocument, offsets_mm: Any) -> MeshDocument:
    return _rust_mesh_edit.offset_verts_mesh(mesh, offsets_mm)


def decimate_mesh(
    mesh: MeshDocument,
    *,
    strategy: str = "minimize_error",
    max_error: float = 1.7976931348623157e308,
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
    return _rust_mesh_edit.decimate_mesh(
        mesh,
        strategy=strategy,
        max_error=max_error,
        max_edge_len=max_edge_len,
        max_bd_shift=max_bd_shift,
        stabilizer=stabilizer,
        target_face_count=target_face_count,
        target_face_ratio=target_face_ratio,
        subdivide_parts=subdivide_parts,
        decimate_between_parts=decimate_between_parts,
        not_flippable_edges=not_flippable_edges,
        edges_to_collapse=edges_to_collapse,
        collapse_near_not_flippable=collapse_near_not_flippable,
        angle_weighted_dist_to_plane=angle_weighted_dist_to_plane,
        max_deleted_vertices=max_deleted_vertices,
        max_deleted_faces=max_deleted_faces,
        max_triangle_aspect_ratio=max_triangle_aspect_ratio,
        critical_tri_aspect_ratio=critical_tri_aspect_ratio,
        tiny_edge_length=tiny_edge_length,
        max_angle_change=max_angle_change,
        touch_near_bd_edges=touch_near_bd_edges,
        touch_bd_verts=touch_bd_verts,
        optimize_vertex_pos=optimize_vertex_pos,
        pack_mesh=pack_mesh,
        region_faces=region_faces,
        vertex_uvs=vertex_uvs,
        vertex_colors=vertex_colors,
        twin_map=twin_map,
    )


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
    return _rust_mesh_edit.subdivide_mesh(
        mesh,
        max_edge_len=max_edge_len,
        max_edge_splits=max_edge_splits,
        curvature_priority=curvature_priority,
        region_faces=region_faces,
        not_flippable_edges=not_flippable_edges,
        subdivide_border=subdivide_border,
        max_tri_aspect_ratio=max_tri_aspect_ratio,
        max_splittable_tri_aspect_ratio=max_splittable_tri_aspect_ratio,
        project_on_original_mesh=project_on_original_mesh,
        smooth_mode=smooth_mode,
        min_sharp_dihedral_angle=min_sharp_dihedral_angle,
        max_deviation_after_flip=max_deviation_after_flip,
        max_angle_change_after_flip=max_angle_change_after_flip,
        critical_tri_aspect_ratio_flip=critical_tri_aspect_ratio_flip,
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
    return _rust_mesh_edit.make_delone_edge_flips(
        mesh,
        num_iters=num_iters,
        region_faces=region_faces,
        max_deviation_after_flip=max_deviation_after_flip,
        max_angle_change=max_angle_change,
        critical_tri_aspect_ratio=critical_tri_aspect_ratio,
        not_flippable_edges=not_flippable_edges,
        vert_region=vert_region,
    )


__all__ = ["decimate_mesh", "make_delone_edge_flips", "offset_verts_mesh", "subdivide_mesh"]
