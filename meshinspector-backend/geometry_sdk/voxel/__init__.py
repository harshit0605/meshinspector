"""Voxel and signed-distance-grid primitives."""

from geometry_sdk.voxel.extract import extract_surface_mesh
from geometry_sdk.voxel.active_box import voxel_active_box
from geometry_sdk.voxel.conversion import (
    voxel_volume_from_meshlib_values,
    voxel_move_mesh_to_max_deriv,
    voxel_to_mesh_dual,
    voxel_to_mesh_dual_vdb_payload,
    voxel_to_mesh_simple,
    voxel_to_mesh_smart,
)
from geometry_sdk.voxel.marching import extract_marching_tetrahedra
from geometry_sdk.voxel.mesh_ops import extract_grid_mesh, voxel_boolean_mesh, voxel_offset_mesh, voxel_partial_offset_mesh, voxel_shell_mesh, voxel_thicken_mesh, voxel_weighted_shell_mesh
from geometry_sdk.voxel.ops import sdf_difference, sdf_intersection, sdf_offset, sdf_shell, sdf_union
from geometry_sdk.voxel.line_graph import voxel_line_graph
from geometry_sdk.voxel.path import voxel_path, voxel_path_build_four
from geometry_sdk.voxel.raw import load_raw_voxels, load_raw_voxels_auto, load_tiff_voxels_dir, voxel_default_iso_value
from geometry_sdk.voxel.refine import laplacian_smooth_vertices, project_vertices_to_sdf, refine_sdf_mesh
from geometry_sdk.voxel.rendering import voxel_volume_render_data, voxel_volume_render_lut, voxel_volume_render_ray
from geometry_sdk.voxel.segmentation import voxel_mask_to_mesh, voxel_segmentation, voxel_segmentation_mesh
from geometry_sdk.voxel.sdf import SDFGrid, estimate_sdf_volume, sample_aligned_sdf_grids, sample_sdf_gradients, sample_sdf_grid, sample_sdf_grid_in_bounds, sample_sdf_values, sdf_cell_values, sdf_occupancy
from geometry_sdk.voxel.slice import voxel_slice

__all__ = [
    "SDFGrid",
    "estimate_sdf_volume",
    "extract_grid_mesh",
    "extract_marching_tetrahedra",
    "extract_surface_mesh",
    "laplacian_smooth_vertices",
    "load_raw_voxels",
    "load_raw_voxels_auto",
    "load_tiff_voxels_dir",
    "voxel_default_iso_value",
    "voxel_volume_from_meshlib_values",
    "voxel_active_box",
    "voxel_volume_render_data",
    "voxel_volume_render_lut",
    "voxel_volume_render_ray",
    "voxel_move_mesh_to_max_deriv",
    "voxel_to_mesh_dual",
    "voxel_to_mesh_dual_vdb_payload",
    "voxel_to_mesh_simple",
    "voxel_to_mesh_smart",
    "voxel_line_graph",
    "voxel_path",
    "voxel_path_build_four",
    "voxel_mask_to_mesh",
    "voxel_segmentation",
    "voxel_segmentation_mesh",
    "voxel_slice",
    "project_vertices_to_sdf",
    "refine_sdf_mesh",
    "sample_aligned_sdf_grids",
    "sample_sdf_gradients",
    "sample_sdf_grid",
    "sample_sdf_grid_in_bounds",
    "sample_sdf_values",
    "sdf_difference",
    "sdf_intersection",
    "sdf_offset",
    "sdf_cell_values",
    "sdf_occupancy",
    "sdf_shell",
    "sdf_union",
    "voxel_boolean_mesh",
    "voxel_offset_mesh",
    "voxel_partial_offset_mesh",
    "voxel_shell_mesh",
    "voxel_thicken_mesh",
    "voxel_weighted_shell_mesh",
]
