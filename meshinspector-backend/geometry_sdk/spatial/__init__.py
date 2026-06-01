"""Spatial query primitives."""

from geometry_sdk.spatial.aabb_tree import (
    AABBNode,
    AABBTree,
    build_aabb_tree,
    closest_candidate_faces,
    overlapping_face_pairs,
    point_aabb_distance_sq,
    ray_candidate_faces,
    ray_intersects_aabb,
)
from geometry_sdk.spatial.closest_point import closest_point_on_triangle, closest_points_on_mesh, point_mesh_distances
from geometry_sdk.spatial.intersections import self_intersecting_faces, triangles_intersect
from geometry_sdk.spatial.raycast import RayHit, first_ray_hit, first_ray_hits, ray_triangle_hits
from geometry_sdk.spatial.signed_distance import (
    point_inside_mesh,
    point_inside_mesh_winding,
    signed_point_mesh_distances,
    supports_winding_sign,
    winding_numbers,
)

__all__ = [
    "AABBNode",
    "AABBTree",
    "build_aabb_tree",
    "closest_candidate_faces",
    "closest_point_on_triangle",
    "closest_points_on_mesh",
    "first_ray_hit",
    "first_ray_hits",
    "overlapping_face_pairs",
    "point_aabb_distance_sq",
    "point_mesh_distances",
    "RayHit",
    "ray_candidate_faces",
    "ray_intersects_aabb",
    "ray_triangle_hits",
    "self_intersecting_faces",
    "point_inside_mesh",
    "point_inside_mesh_winding",
    "signed_point_mesh_distances",
    "supports_winding_sign",
    "triangles_intersect",
    "winding_numbers",
]
