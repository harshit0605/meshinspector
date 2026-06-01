"""Repair and healing primitives."""

from geometry_sdk.repair.basic import basic_repair, merge_close_vertices, orient_faces_outward, remove_degenerate_faces, remove_unreferenced_vertices
from geometry_sdk.repair.holes import fill_planar_holes, ordered_boundary_loops, service_fill_holes

__all__ = [
    "basic_repair",
    "fill_planar_holes",
    "merge_close_vertices",
    "orient_faces_outward",
    "ordered_boundary_loops",
    "remove_degenerate_faces",
    "remove_unreferenced_vertices",
    "service_fill_holes",
]
