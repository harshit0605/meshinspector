"""Stable public facade for the parallel geometry SDK."""

from __future__ import annotations

from collections.abc import Sequence
from pathlib import Path

import numpy as np

from geometry_sdk.analysis.compare import (
    compare_summary,
    nearest_surface_distances,
    service_compare_distances,
    service_compare_summary,
    signed_compare_summary,
    signed_surface_distances,
    version_compare_distances,
    version_compare_summary,
)
from geometry_sdk.analysis.artifacts import (
    compare_overlay_payload,
    save_compare_npz,
    save_thickness_npz,
    thickness_overlay_payload,
)
from geometry_sdk.analysis.health import compute_mesh_health, service_mesh_health
from geometry_sdk.analysis.manufacturability import compute_manufacturability_report
from geometry_sdk.analysis.section import section_contour
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.analysis.quality import (
    boolean_output_failures,
    decimate_output_failures,
    hollow_output_failures,
    offset_shell_failures,
)
from geometry_sdk.analysis.thickness import ray_thickness_at_vertices, service_thickness_at_vertices, summarize_thickness
from geometry_sdk.deform.brushes import apply_brush_strokes, brush_stroke_from_regions
from geometry_sdk.deform.local import local_scoop, local_thicken, local_thicken_to_minimum, smooth, taubin_smooth
from geometry_sdk.deform.resize import fit_ring_to_diameter, radial_scale, resize_ring
from geometry_sdk.deform.thicken import global_thicken
from geometry_sdk.distance_map import (
    DistanceMapDocument,
    IsoLineSegmentsDocument,
    ObjectLinesDocument,
    distance_map_contour_boolean,
    distance_map_from_contours,
    distance_map_from_mesh,
    distance_map_from_tiff,
    distance_map_merge,
    distance_map_to_tiff,
    distance_map_to_iso_segments,
    offset_contours,
    offset_contours_with_origins,
    object_lines_from_contours,
    object_lines_from_mrlines,
    object_lines_from_ply,
    object_lines_from_pts,
    object_lines_from_svg,
    object_lines_to_contours,
    object_lines_to_dxf,
    object_lines_to_mrlines,
    object_lines_to_ply,
    object_lines_to_pts,
)
from geometry_sdk.gcode import (
    GcodeMachineSettings,
    GcodePathDocument,
    load_gcode_source,
    parse_gcode_file_paths,
    parse_gcode_paths,
    write_gcode_source,
)
from geometry_sdk.io.trimesh_adapter import load_mesh, save_mesh
from geometry_sdk.core.mesh import (
    apply_meshlib_selection_modifier,
    bounded_seed_indices,
    extract_selected_faces_as_mesh,
    expand_face_selection_to_components,
    feature_object_descriptors,
    feature_pair_measurements,
    graph_cut_select_region,
    graph_cut_select_region_auto_not_region,
    meshlib_apply_scene_ribbon_action,
    meshlib_group_scene_objects,
    meshlib_rename_scene_object,
    meshlib_scene_feature_object_render_payload,
    meshlib_select_scene_objects,
    meshlib_ungroup_scene_objects,
    mesh_closest_surface_path_targets,
    mesh_fast_marching_surface_path,
    mesh_fast_marching_surface_path_tri_points,
    mesh_geodesic_distance_field,
    mesh_geodesic_extreme_edges,
    mesh_geodesic_iso_region,
    mesh_geodesic_path,
    mesh_geodesic_polyline_path,
    mesh_geodesic_quadrangle_path,
    mesh_cut_measure_edge_path_topology_cut,
    mesh_cut_measure_contours,
    mesh_geodesic_edge_point_path,
    mesh_planar_triangle_strip_path,
    mesh_surface_edge_point_path,
    mesh_surface_path_tri_points,
    mesh_surface_distance_seed_vertices,
    mesh_steepest_descent_edge_step,
    mesh_steepest_descent_path,
    mesh_steepest_descent_triangle_step,
    mesh_steepest_descent_vertex_step,
    mesh_triangle_strip_unfolded_path,
    normalize_axis,
    refine_feature_primitives,
    select_boundary_edges,
    select_boundary_faces,
    select_camera_facing_faces,
    select_crease_edges,
    select_degenerate_faces,
    select_face_by_ray,
    select_faces_by_area,
    select_faces_by_screen_brush,
    select_faces_by_screen_polygon,
    select_faces_by_screen_rect,
    select_inside_part_faces,
    select_largest_component_faces,
    select_not_smooth_faces,
    select_outer_layer_faces,
    select_overlapping_faces,
    select_overhang_faces,
    select_short_edges,
    selection_seed_indices,
)
from geometry_sdk.jewelry.hollow import (
    adaptive_hollow_to_weight,
    adaptive_protected_hollow_to_weight,
    apply_drain_holes_voxel,
    drain_hole_cutter_mesh,
    drain_hole_cutters_mesh,
    plan_drain_holes,
    protected_hollow_mesh,
    protected_hollow_scale_field,
    service_hollow_mesh,
    service_hollow_voxel_size,
    weighted_inner_offset_preview,
)
from geometry_sdk.jewelry.regions import detect_ring_regions
from geometry_sdk.jewelry.ring_measurement import measure_ring
from geometry_sdk.mesh_edit import decimate_mesh, make_delone_edge_flips, offset_verts_mesh, subdivide_mesh
from geometry_sdk.point_cloud import (
    ICPObjectTransform,
    ICPRegistrationResult,
    MultiwayICPRegistrationResult,
    PointCloudClosestPair,
    PointCloudDocument,
    PointCloudLocalFan,
    PointCloudLocalFanTriangles,
    PointCloudLocalTriangulationRepetitions,
    PointCloudMeshProjectionResult,
    PointCloudProjectionResult,
    load_point_cloud_ply,
    multiway_all_object_combined_icp,
    multiway_all_object_point_to_plane_icp,
    multiway_all_object_point_to_point_icp,
    multiway_aabb_cascade_combined_icp,
    multiway_aabb_cascade_point_to_plane_icp,
    multiway_aabb_cascade_point_to_point_icp,
    multiway_combined_icp,
    multiway_point_to_plane_icp,
    multiway_point_to_point_icp,
    multiway_sequential_cascade_combined_icp,
    multiway_sequential_cascade_point_to_plane_icp,
    multiway_sequential_cascade_point_to_point_icp,
    pairwise_point_to_plane_icp,
    pairwise_point_to_point_icp,
    point_cloud_from_ply,
    point_cloud_grid_sample,
    point_cloud_extract_selected_points_as_object,
    point_cloud_triangulate_filled_candidate_mesh,
    point_cloud_local_fan_triangles,
    point_cloud_local_neighbor_fan,
    point_cloud_local_triangulation_repetitions,
    point_cloud_n_closest_neighbors,
    point_cloud_nearest_projections,
    point_cloud_neighbors_in_radius,
    point_cloud_pick_by_ray,
    point_cloud_project_to_mesh,
    point_cloud_select_by_screen_brush,
    point_cloud_select_by_screen_polygon,
    point_cloud_select_by_screen_rect,
    point_cloud_triangulate_candidate_mesh,
    point_cloud_triangulate_cleaned_candidate_mesh,
    point_cloud_triangulate_topology_candidate_mesh,
    point_cloud_to_ply,
    point_cloud_two_closest_points,
    point_cloud_uniform_sample,
    save_point_cloud_ply,
)
from geometry_sdk.repair.basic import (
    DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
    basic_repair,
    crease_edge_diagnostics,
    crease_repair_plan_diagnostics,
    degenerate_face_diagnostics,
    detect_tunnel_faces,
    eliminate_tunnels,
    duplicate_nonmanifold_vertices,
    duplicate_multi_hole_vertices,
    fix_mesh_creases,
    find_disoriented_faces,
    flip_normals,
    mesh_healer_diagnostics,
    multiple_edge_diagnostics,
    not_smooth_face_diagnostics,
    orient_faces_outward,
    prune_small_components,
    weld_coincident_vertices,
    repair_multiple_edges,
    repair_nonmanifold_edges,
    short_edge_diagnostics,
    tunnel_diagnostics,
    unite_close_vertices,
)
from geometry_sdk.repair.holes import (
    fill_planar_holes,
    hole_complicating_faces_diagnostics,
    hole_fill_plan_diagnostics,
    repeated_hole_boundary_vertices_diagnostics,
    remove_hole_complicating_faces,
    service_fill_holes,
)
from geometry_sdk.repair.self_intersections import fix_self_intersections_relax
from geometry_sdk.repair.voxel import rebuild_via_sdf
from geometry_sdk.spatial.closest_point import closest_points_on_mesh, point_mesh_distances
from geometry_sdk.spatial.boolean import exact_boolean_mesh as exact_boolean_mesh_op
from geometry_sdk.spatial.intersections import (
    exact_mesh_intersections as exact_mesh_intersections_op,
    self_intersecting_faces as self_intersecting_faces_op,
)
from geometry_sdk.voxel.extract import extract_surface_mesh
from geometry_sdk.voxel.marching import extract_marching_tetrahedra
from geometry_sdk.voxel.mesh_ops import (
    voxel_boolean_mesh,
    voxel_offset_mesh,
    voxel_partial_offset_mesh,
    voxel_shell_mesh,
    voxel_thicken_mesh,
    voxel_weighted_shell_mesh,
)
from geometry_sdk.voxel.ops import (
    sdf_difference,
    sdf_intersection,
    sdf_offset,
    sdf_shell,
    sdf_union,
    voxel_binary_iso_value,
    voxel_binary_operation,
    voxel_binary_values,
)
from geometry_sdk.voxel.conversion import (
    voxel_volume_from_meshlib_values,
    voxel_move_mesh_to_max_deriv,
    voxel_to_mesh_dual,
    voxel_to_mesh_dual_vdb_payload,
    voxel_to_mesh_simple,
    voxel_to_mesh_smart,
)
from geometry_sdk.voxel.line_graph import voxel_line_graph
from geometry_sdk.voxel.active_box import voxel_active_box
from geometry_sdk.voxel.path import voxel_path, voxel_path_build_four
from geometry_sdk.voxel.raw import load_raw_voxels, load_raw_voxels_auto, load_tiff_voxels_dir, voxel_default_iso_value
from geometry_sdk.voxel.rendering import voxel_volume_render_data, voxel_volume_render_lut, voxel_volume_render_ray
from geometry_sdk.voxel.segmentation import voxel_mask_to_mesh, voxel_segmentation, voxel_segmentation_mesh
from geometry_sdk.voxel.slice import voxel_slice
from geometry_sdk.voxel.sdf import SDFGrid, estimate_sdf_volume, sample_sdf_grid, sdf_occupancy
from geometry_sdk.types import (
    AdaptiveHollowReport,
    BrushStroke,
    ComponentPruneReport,
    CreaseEdgeDiagnostics,
    CreaseRepairPlanDiagnostics,
    DegenerateFaceDiagnostics,
    DecimateMeshResult,
    DuplicateNonManifoldVerticesReport,
    DuplicateMultiHoleVerticesReport,
    DrainHolePlan,
    ExactBooleanMeshResult,
    RingFitResult,
    FixSelfIntersectionsRelaxReport,
    FixMeshCreasesReport,
    HoleComplicatingFacesDiagnostics,
    HoleFillReport,
    HoleFillPlanDiagnostics,
    ManufacturabilityReport,
    MeshDocument,
    MeshCollisionResult,
    MeshHealerReport,
    MeshHealth,
    MeshStats,
    MultipleEdgeDiagnostics,
    MultipleEdgeRepairReport,
    NonManifoldEdgeRepairReport,
    NotSmoothFaceDiagnostics,
    RegionEntry,
    RepairReport,
    RepeatedHoleBoundaryVerticesDiagnostics,
    RemoveHoleComplicatingFacesReport,
    RingMeasurement,
    SectionContourPayload,
    ServiceMeshHealth,
    ShortEdgeDiagnostics,
    SubdivideMeshResult,
    ThicknessSummary,
    TunnelDiagnostics,
    TunnelEliminationReport,
    VersionCompareSummary,
    VoxelActiveBoxResult,
    VoxelLineGraphResult,
    VoxelPathResult,
    VoxelVolumeRenderDataResult,
    VoxelVolumeRenderLutResult,
    VoxelVolumeRenderRayResult,
    VoxelSegmentationResult,
    VoxelSliceResult,
    VoxelVolume,
    VoxelRebuildReport,
)


class GeometrySDK:
    """Public API boundary for in-house algorithms.

    The facade is intentionally thin in V0. It gives future service migration a
    stable import target while the underlying modules continue to evolve.
    """

    def load_mesh(self, path: str | Path) -> MeshDocument:
        return load_mesh(path)

    def save_mesh(self, mesh: MeshDocument, path: str | Path, *, file_type: str | None = None) -> Path:
        return save_mesh(mesh, path, file_type=file_type)

    def expand_face_selection_to_components(self, mesh: MeshDocument, seed_face_ids: list[int]) -> list[int]:
        return expand_face_selection_to_components(mesh, seed_face_ids)

    def bounded_seed_indices(self, mesh: MeshDocument, indices, max_count: int) -> np.ndarray:
        return bounded_seed_indices(mesh, indices, max_count)

    def selection_seed_indices(
        self,
        mesh: MeshDocument,
        *,
        vertex_ids=None,
        face_ids=None,
        region_vertex_indices=None,
        brush_points_world=None,
    ) -> np.ndarray:
        return selection_seed_indices(
            mesh,
            vertex_ids=vertex_ids,
            face_ids=face_ids,
            region_vertex_indices=region_vertex_indices,
            brush_points_world=brush_points_world,
        )

    def normalize_axis(self, axis) -> np.ndarray:
        return normalize_axis(axis)

    def apply_meshlib_selection_modifier(
        self,
        current_ids: Sequence[int],
        incoming_ids: Sequence[int],
        mode: str,
        *,
        item_count: int | None = None,
    ) -> list[int]:
        return apply_meshlib_selection_modifier(
            current_ids,
            incoming_ids,
            mode,
            item_count=item_count,
        )

    def meshlib_select_scene_objects(
        self,
        mesh: MeshDocument,
        *,
        object_keys,
        mode: str = "select_one",
    ) -> MeshDocument:
        return meshlib_select_scene_objects(mesh, object_keys=object_keys, mode=mode)

    def meshlib_apply_scene_ribbon_action(
        self,
        mesh: MeshDocument,
        *,
        action: str,
    ) -> MeshDocument:
        return meshlib_apply_scene_ribbon_action(mesh, action=action)

    def meshlib_group_scene_objects(
        self,
        mesh: MeshDocument,
        *,
        group_key: str,
    ) -> MeshDocument:
        return meshlib_group_scene_objects(mesh, group_key=group_key)

    def meshlib_ungroup_scene_objects(self, mesh: MeshDocument) -> MeshDocument:
        return meshlib_ungroup_scene_objects(mesh)

    def meshlib_rename_scene_object(
        self,
        mesh: MeshDocument,
        *,
        object_key: str,
        object_name: str,
    ) -> MeshDocument:
        return meshlib_rename_scene_object(mesh, object_key=object_key, object_name=object_name)

    def meshlib_scene_feature_object_render_payload(
        self,
        mesh: MeshDocument,
        *,
        viewport_mask: int = 0xFFFF_FFFF,
        circle_segments: int = 64,
    ) -> dict[str, object]:
        return meshlib_scene_feature_object_render_payload(
            mesh,
            viewport_mask=viewport_mask,
            circle_segments=circle_segments,
        )

    def extract_selected_faces_as_mesh(self, mesh: MeshDocument, selected_face_ids) -> MeshDocument:
        return extract_selected_faces_as_mesh(mesh, selected_face_ids)

    def select_boundary_faces(self, mesh: MeshDocument) -> list[int]:
        return select_boundary_faces(mesh)

    def select_boundary_edges(self, mesh: MeshDocument) -> list[tuple[int, int]]:
        return select_boundary_edges(mesh)

    def select_degenerate_faces(
        self,
        mesh: MeshDocument,
        *,
        min_aspect_ratio: float,
        boundary_only: bool = False,
    ) -> list[int]:
        return select_degenerate_faces(
            mesh,
            min_aspect_ratio=min_aspect_ratio,
            boundary_only=boundary_only,
        )

    def select_short_edges(self, mesh: MeshDocument, *, max_edge_length_mm: float) -> list[tuple[int, int]]:
        return select_short_edges(mesh, max_edge_length_mm=max_edge_length_mm)

    def select_faces_by_area(
        self,
        mesh: MeshDocument,
        *,
        area: float,
        scalar_type: str = "absolute",
        compare_type: str = "less",
    ) -> list[int]:
        return select_faces_by_area(
            mesh,
            area=area,
            scalar_type=scalar_type,
            compare_type=compare_type,
        )

    def select_crease_edges(
        self,
        mesh: MeshDocument,
        *,
        angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
        min_component_length_mm: float | None = None,
        min_branch_length_mm: float | None = None,
    ) -> list[tuple[int, int]]:
        return select_crease_edges(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            min_component_length_mm=min_component_length_mm,
            min_branch_length_mm=min_branch_length_mm,
        )

    def select_largest_component_faces(self, mesh: MeshDocument, *, min_area_mm2: float = 0.0) -> list[int]:
        return select_largest_component_faces(mesh, min_area_mm2=min_area_mm2)

    def select_overhang_faces(
        self,
        mesh: MeshDocument,
        *,
        axis=(0.0, 0.0, 1.0),
        layer_height_mm: float,
        max_overhang_distance_mm: float,
        hops: int = 0,
    ) -> list[int]:
        return select_overhang_faces(
            mesh,
            axis=axis,
            layer_height_mm=layer_height_mm,
            max_overhang_distance_mm=max_overhang_distance_mm,
            hops=hops,
        )

    def select_outer_layer_faces(self, mesh: MeshDocument, *, epsilon: float = 1e-8) -> list[int]:
        return select_outer_layer_faces(mesh, epsilon=epsilon)

    def select_not_smooth_faces(self, mesh: MeshDocument, *, min_angle_radians: float = 0.3) -> list[int]:
        return select_not_smooth_faces(mesh, min_angle_radians=min_angle_radians)

    def select_overlapping_faces(
        self,
        mesh: MeshDocument,
        *,
        max_dist_sq: float = 1e-10,
        max_normal_dot: float = -0.99,
        min_area_fraction: float = 1e-5,
    ) -> list[int]:
        return select_overlapping_faces(
            mesh,
            max_dist_sq=max_dist_sq,
            max_normal_dot=max_normal_dot,
            min_area_fraction=min_area_fraction,
        )

    def graph_cut_select_region(
        self,
        mesh: MeshDocument,
        *,
        source_face_ids,
        sink_face_ids,
        boundary_weight: float = 1.0,
        curvature_preference: str = "geodesic",
    ) -> list[int]:
        return graph_cut_select_region(
            mesh,
            source_face_ids=source_face_ids,
            sink_face_ids=sink_face_ids,
            boundary_weight=boundary_weight,
            curvature_preference=curvature_preference,
        )

    def graph_cut_select_region_auto_not_region(
        self,
        mesh: MeshDocument,
        *,
        source_face_ids,
        uncertainty_distance_mm: float,
        boundary_weight: float = 1.0,
        curvature_preference: str = "geodesic",
    ) -> list[int]:
        return graph_cut_select_region_auto_not_region(
            mesh,
            source_face_ids=source_face_ids,
            uncertainty_distance_mm=uncertainty_distance_mm,
            boundary_weight=boundary_weight,
            curvature_preference=curvature_preference,
        )

    def select_faces_by_screen_polygon(
        self,
        mesh: MeshDocument,
        view_projection_4x4,
        polygon_xy,
        *,
        include_backfaces: bool = True,
        visible_only: bool = False,
    ) -> list[int]:
        return select_faces_by_screen_polygon(
            mesh,
            view_projection_4x4,
            polygon_xy,
            include_backfaces=include_backfaces,
            visible_only=visible_only,
        )

    def select_faces_by_screen_rect(
        self,
        mesh: MeshDocument,
        view_projection_4x4,
        rect_min_xy,
        rect_max_xy,
        *,
        include_backfaces: bool = True,
        visible_only: bool = False,
    ) -> list[int]:
        return select_faces_by_screen_rect(
            mesh,
            view_projection_4x4,
            rect_min_xy,
            rect_max_xy,
            include_backfaces=include_backfaces,
            visible_only=visible_only,
        )

    def select_faces_by_screen_brush(
        self,
        mesh: MeshDocument,
        view_projection_4x4,
        brush_path_xy,
        *,
        radius_px: float,
        include_backfaces: bool = True,
        visible_only: bool = False,
    ) -> list[int]:
        return select_faces_by_screen_brush(
            mesh,
            view_projection_4x4,
            brush_path_xy,
            radius_px=radius_px,
            include_backfaces=include_backfaces,
            visible_only=visible_only,
        )

    def select_face_by_ray(
        self,
        mesh: MeshDocument,
        ray_origin,
        ray_direction,
        *,
        epsilon: float = 1e-8,
        ignore_faces=None,
    ) -> list[int]:
        return select_face_by_ray(
            mesh,
            ray_origin,
            ray_direction,
            epsilon=epsilon,
            ignore_faces=ignore_faces,
        )

    def select_inside_part_faces(self, mesh: MeshDocument) -> list[int]:
        return select_inside_part_faces(mesh)

    def select_camera_facing_faces(
        self,
        mesh: MeshDocument,
        *,
        camera_direction,
        min_dot: float = 0.0,
    ) -> list[int]:
        return select_camera_facing_faces(
            mesh,
            camera_direction=camera_direction,
            min_dot=min_dot,
        )

    def offset_verts_mesh(self, mesh: MeshDocument, offsets_mm) -> MeshDocument:
        return offset_verts_mesh(mesh, offsets_mm)

    def decimate_mesh(
        self,
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
        not_flippable_edges=None,
        edges_to_collapse=None,
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
        region_faces=None,
        vertex_uvs=None,
        vertex_colors=None,
        twin_map=None,
    ) -> DecimateMeshResult:
        return decimate_mesh(
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
        self,
        mesh: MeshDocument,
        *,
        max_edge_len: float,
        max_edge_splits: int = 1000,
        region_faces=None,
        not_flippable_edges=None,
        subdivide_border: bool = True,
        curvature_priority: float = 0.0,
        project_on_original_mesh: bool = False,
        smooth_mode: bool = False,
        min_sharp_dihedral_angle: float = 0.5235987755982989,
        max_tri_aspect_ratio: float = 0.0,
        max_splittable_tri_aspect_ratio: float | None = None,
        max_deviation_after_flip: float | None = None,
        max_angle_change_after_flip: float | None = None,
        critical_tri_aspect_ratio_flip: float | None = None,
    ) -> SubdivideMeshResult:
        return subdivide_mesh(
            mesh,
            max_edge_len=max_edge_len,
            max_edge_splits=max_edge_splits,
            region_faces=region_faces,
            not_flippable_edges=not_flippable_edges,
            subdivide_border=subdivide_border,
            curvature_priority=curvature_priority,
            project_on_original_mesh=project_on_original_mesh,
            smooth_mode=smooth_mode,
            min_sharp_dihedral_angle=min_sharp_dihedral_angle,
            max_tri_aspect_ratio=max_tri_aspect_ratio,
            max_splittable_tri_aspect_ratio=max_splittable_tri_aspect_ratio,
            max_deviation_after_flip=max_deviation_after_flip,
            max_angle_change_after_flip=max_angle_change_after_flip,
            critical_tri_aspect_ratio_flip=critical_tri_aspect_ratio_flip,
        )

    def make_delone_edge_flips(
        self,
        mesh: MeshDocument,
        *,
        num_iters: int = 1,
        region_faces=None,
        max_deviation_after_flip: float | None = None,
        max_angle_change: float | None = None,
        critical_tri_aspect_ratio: float | None = None,
        not_flippable_edges=None,
        vert_region=None,
    ) -> tuple[MeshDocument, int]:
        return make_delone_edge_flips(
            mesh,
            num_iters=num_iters,
            region_faces=region_faces,
            max_deviation_after_flip=max_deviation_after_flip,
            max_angle_change=max_angle_change,
            critical_tri_aspect_ratio=critical_tri_aspect_ratio,
            not_flippable_edges=not_flippable_edges,
            vert_region=vert_region,
        )

    def stats(self, mesh: MeshDocument) -> MeshStats:
        return compute_mesh_stats(mesh)

    def health(self, mesh: MeshDocument) -> MeshHealth:
        return compute_mesh_health(mesh)

    def decimate_output_failures(self, source: MeshDocument, output: MeshDocument) -> list[str]:
        return decimate_output_failures(source, output)

    def hollow_output_failures(
        self, source: MeshDocument, output: MeshDocument, *, wall_thickness_mm: float
    ) -> list[str]:
        return hollow_output_failures(source, output, wall_thickness_mm)

    def offset_shell_failures(self, source: MeshDocument, output: MeshDocument) -> list[str]:
        return offset_shell_failures(source, output)

    def boolean_output_failures(
        self,
        output: MeshDocument,
        *,
        operation: str,
        source_volume_mm3: float,
        target_volume_mm3: float,
    ) -> list[str]:
        return boolean_output_failures(
            output, operation, source_volume_mm3, target_volume_mm3
        )

    def service_health(self, mesh: MeshDocument, *, max_listed_faces: int = 100) -> ServiceMeshHealth:
        return service_mesh_health(mesh, max_listed_faces=max_listed_faces)

    def pairwise_point_to_point_icp(
        self,
        floating: PointCloudDocument,
        reference: PointCloudDocument,
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
    ) -> ICPRegistrationResult:
        return pairwise_point_to_point_icp(
            floating,
            reference,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
        )

    def multiway_point_to_point_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_point_to_point_icp(
            objects,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_point_to_plane_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_point_to_plane_icp(
            objects,
            normals,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_combined_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_combined_icp(
            objects,
            normals,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_all_object_point_to_point_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_all_object_point_to_point_icp(
            objects,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_all_object_point_to_plane_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_all_object_point_to_plane_icp(
            objects,
            normals,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_all_object_combined_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_all_object_combined_icp(
            objects,
            normals,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_sequential_cascade_point_to_point_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        *,
        max_group_size: int = 64,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_sequential_cascade_point_to_point_icp(
            objects,
            max_group_size=max_group_size,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_sequential_cascade_point_to_plane_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_group_size: int = 64,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_sequential_cascade_point_to_plane_icp(
            objects,
            normals,
            max_group_size=max_group_size,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_sequential_cascade_combined_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_group_size: int = 64,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_sequential_cascade_combined_icp(
            objects,
            normals,
            max_group_size=max_group_size,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_aabb_cascade_point_to_point_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        *,
        max_group_size: int = 64,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_aabb_cascade_point_to_point_icp(
            objects,
            max_group_size=max_group_size,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_aabb_cascade_point_to_plane_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_group_size: int = 64,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_aabb_cascade_point_to_plane_icp(
            objects,
            normals,
            max_group_size=max_group_size,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def multiway_aabb_cascade_combined_icp(
        self,
        objects: tuple[PointCloudDocument, ...] | list[PointCloudDocument],
        normals,
        *,
        max_group_size: int = 64,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        fixed_object_index: int | None = None,
    ) -> MultiwayICPRegistrationResult:
        return multiway_aabb_cascade_combined_icp(
            objects,
            normals,
            max_group_size=max_group_size,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            fixed_object_index=fixed_object_index,
        )

    def pairwise_point_to_plane_icp(
        self,
        floating: PointCloudDocument,
        reference: PointCloudDocument,
        reference_normals,
        *,
        max_iterations: int = 20,
        tolerance: float = 1e-8,
        mode: str = "rigid",
        floating_normals=None,
        max_pair_distance: float | None = None,
        cos_threshold: float | None = None,
        far_dist_factor: float | None = None,
        mutual_closest: bool = False,
    ) -> ICPRegistrationResult:
        return pairwise_point_to_plane_icp(
            floating,
            reference,
            reference_normals,
            max_iterations=max_iterations,
            tolerance=tolerance,
            mode=mode,
            floating_normals=floating_normals,
            max_pair_distance=max_pair_distance,
            cos_threshold=cos_threshold,
            far_dist_factor=far_dist_factor,
            mutual_closest=mutual_closest,
        )

    def point_cloud_from_ply(self, source: bytes | bytearray) -> PointCloudDocument:
        return point_cloud_from_ply(source)

    def load_point_cloud_ply(self, path: str | Path) -> PointCloudDocument:
        return load_point_cloud_ply(path)

    def point_cloud_to_ply(self, cloud: PointCloudDocument) -> bytes:
        return point_cloud_to_ply(cloud)

    def save_point_cloud_ply(self, cloud: PointCloudDocument, path: str | Path) -> Path:
        return save_point_cloud_ply(cloud, path)

    def point_cloud_grid_sample(
        self,
        cloud: PointCloudDocument,
        *,
        voxel_size: float,
        max_voxels: int = 500_000,
    ) -> PointCloudDocument:
        sampled = point_cloud_grid_sample(
            cloud,
            voxel_size=voxel_size,
            max_voxels=max_voxels,
        )
        if isinstance(sampled, tuple):
            return sampled[0]
        return sampled

    def point_cloud_uniform_sample(
        self,
        cloud: PointCloudDocument,
        *,
        distance: float,
        min_normal_dot: float = 0.0,
        lexicographical_order: bool = True,
        normals=None,
    ) -> PointCloudDocument:
        sampled = point_cloud_uniform_sample(
            cloud,
            distance=distance,
            min_normal_dot=min_normal_dot,
            lexicographical_order=lexicographical_order,
            normals=normals,
        )
        if isinstance(sampled, tuple):
            return sampled[0]
        return sampled

    def point_cloud_nearest_projections(
        self,
        query_points,
        reference: PointCloudDocument,
        *,
        up_dist_limit_sq: float = np.inf,
        lo_dist_limit_sq: float = 0.0,
        skip_same_index: bool = False,
    ) -> PointCloudProjectionResult:
        return point_cloud_nearest_projections(
            query_points,
            reference,
            up_dist_limit_sq=up_dist_limit_sq,
            lo_dist_limit_sq=lo_dist_limit_sq,
            skip_same_index=skip_same_index,
        )

    def point_cloud_project_to_mesh(
        self,
        query_points,
        mesh: MeshDocument,
        *,
        up_dist_limit_sq: float = np.finfo(np.float64).max,
        lo_dist_limit_sq: float = 0.0,
        point_transform=None,
        mesh_transform=None,
        face_mask=None,
    ) -> PointCloudMeshProjectionResult:
        return point_cloud_project_to_mesh(
            query_points,
            mesh,
            up_dist_limit_sq=up_dist_limit_sq,
            lo_dist_limit_sq=lo_dist_limit_sq,
            point_transform=point_transform,
            mesh_transform=mesh_transform,
            face_mask=face_mask,
        )

    def point_cloud_n_closest_neighbors(
        self,
        cloud: PointCloudDocument,
        *,
        num_neighbors: int,
        up_dist_limit_sq: float = np.finfo(np.float64).max,
    ) -> np.ndarray:
        return point_cloud_n_closest_neighbors(
            cloud,
            num_neighbors=num_neighbors,
            up_dist_limit_sq=up_dist_limit_sq,
        )

    def point_cloud_two_closest_points(
        self,
        cloud: PointCloudDocument,
    ) -> PointCloudClosestPair:
        return point_cloud_two_closest_points(cloud)

    def point_cloud_neighbors_in_radius(
        self,
        cloud: PointCloudDocument,
        *,
        center_index: int,
        radius: float,
        normals=None,
        untrusted_indices=None,
    ) -> np.ndarray:
        return point_cloud_neighbors_in_radius(
            cloud,
            center_index=center_index,
            radius=radius,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_select_by_screen_polygon(
        self,
        cloud: PointCloudDocument,
        view_projection_4x4,
        polygon_xy,
        *,
        normals=None,
        include_backfaces: bool = True,
        visible_only: bool = False,
    ) -> np.ndarray:
        return point_cloud_select_by_screen_polygon(
            cloud,
            view_projection_4x4,
            polygon_xy,
            normals=normals,
            include_backfaces=include_backfaces,
            visible_only=visible_only,
        )

    def point_cloud_select_by_screen_rect(
        self,
        cloud: PointCloudDocument,
        view_projection_4x4,
        rect_min_xy,
        rect_max_xy,
        *,
        normals=None,
        include_backfaces: bool = True,
        visible_only: bool = False,
    ) -> np.ndarray:
        return point_cloud_select_by_screen_rect(
            cloud,
            view_projection_4x4,
            rect_min_xy,
            rect_max_xy,
            normals=normals,
            include_backfaces=include_backfaces,
            visible_only=visible_only,
        )

    def point_cloud_select_by_screen_brush(
        self,
        cloud: PointCloudDocument,
        view_projection_4x4,
        brush_path_xy,
        *,
        radius_px: float,
        normals=None,
        include_backfaces: bool = True,
        visible_only: bool = False,
    ) -> np.ndarray:
        return point_cloud_select_by_screen_brush(
            cloud,
            view_projection_4x4,
            brush_path_xy,
            radius_px=radius_px,
            normals=normals,
            include_backfaces=include_backfaces,
            visible_only=visible_only,
        )

    def point_cloud_pick_by_ray(
        self,
        cloud: PointCloudDocument,
        ray_origin,
        ray_direction,
        *,
        max_distance_to_ray: float,
        max_depth: float = np.inf,
        normals=None,
        include_backfaces: bool = True,
    ) -> np.ndarray:
        return point_cloud_pick_by_ray(
            cloud,
            ray_origin,
            ray_direction,
            max_distance_to_ray=max_distance_to_ray,
            max_depth=max_depth,
            normals=normals,
            include_backfaces=include_backfaces,
        )

    def point_cloud_extract_selected_points_as_object(
        self,
        cloud: PointCloudDocument,
        selected_point_ids,
    ) -> PointCloudDocument:
        return point_cloud_extract_selected_points_as_object(cloud, selected_point_ids)

    def point_cloud_local_neighbor_fan(
        self,
        cloud: PointCloudDocument,
        *,
        center_index: int,
        radius: float,
        num_neighbors: int = 0,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 0,
        crit_angle: float = np.pi * 2.0,
        normals=None,
        untrusted_indices=None,
    ) -> PointCloudLocalFan:
        return point_cloud_local_neighbor_fan(
            cloud,
            center_index=center_index,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_local_fan_triangles(
        self,
        cloud: PointCloudDocument,
        *,
        center_index: int,
        radius: float,
        num_neighbors: int = 0,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 0,
        crit_angle: float = np.pi * 2.0,
        normals=None,
        untrusted_indices=None,
    ) -> PointCloudLocalFanTriangles:
        return point_cloud_local_fan_triangles(
            cloud,
            center_index=center_index,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_local_triangulation_repetitions(
        self,
        cloud: PointCloudDocument,
        *,
        radius: float,
        num_neighbors: int = 0,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 0,
        crit_angle: float = np.pi * 2.0,
        normals=None,
        untrusted_indices=None,
    ) -> PointCloudLocalTriangulationRepetitions:
        return point_cloud_local_triangulation_repetitions(
            cloud,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_triangulate_candidate_mesh(
        self,
        cloud: PointCloudDocument,
        *,
        radius: float = 0.0,
        num_neighbors: int = 16,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 2_147_483_647,
        crit_angle: float = np.pi * 2.0,
        normals=None,
        untrusted_indices=None,
    ) -> MeshDocument:
        return point_cloud_triangulate_candidate_mesh(
            cloud,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_triangulate_cleaned_candidate_mesh(
        self,
        cloud: PointCloudDocument,
        *,
        radius: float = 0.0,
        num_neighbors: int = 16,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 2_147_483_647,
        crit_angle: float = np.pi * 2.0,
        normals=None,
        untrusted_indices=None,
    ) -> MeshDocument:
        return point_cloud_triangulate_cleaned_candidate_mesh(
            cloud,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_triangulate_topology_candidate_mesh(
        self,
        cloud: PointCloudDocument,
        *,
        radius: float = 0.0,
        num_neighbors: int = 16,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 2_147_483_647,
        crit_angle: float = np.pi * 2.0,
        normals=None,
        untrusted_indices=None,
    ) -> MeshDocument:
        return point_cloud_triangulate_topology_candidate_mesh(
            cloud,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def point_cloud_triangulate_filled_candidate_mesh(
        self,
        cloud: PointCloudDocument,
        *,
        radius: float = 0.0,
        num_neighbors: int = 16,
        boundary_angle: float = np.pi * 0.9,
        max_removes: int = 2_147_483_647,
        crit_angle: float = np.pi * 2.0,
        crit_hole_length: float = -1.0,
        normals=None,
        untrusted_indices=None,
    ) -> MeshDocument:
        return point_cloud_triangulate_filled_candidate_mesh(
            cloud,
            radius=radius,
            num_neighbors=num_neighbors,
            boundary_angle=boundary_angle,
            max_removes=max_removes,
            crit_angle=crit_angle,
            crit_hole_length=crit_hole_length,
            normals=normals,
            untrusted_indices=untrusted_indices,
        )

    def distance_map_from_contours(
        self,
        contours,
        *,
        width: int,
        height: int,
        origin: tuple[float, float] = (0.0, 0.0),
        pixel_size: float | tuple[float, float] = 1.0,
        signed: bool = False,
    ) -> DistanceMapDocument:
        return distance_map_from_contours(
            contours,
            width=width,
            height=height,
            origin=origin,
            pixel_size=pixel_size,
            signed=signed,
        )

    def distance_map_from_mesh(
        self,
        mesh: MeshDocument,
        *,
        width: int,
        height: int,
        origin: tuple[float, float, float],
        x_range: tuple[float, float, float],
        y_range: tuple[float, float, float],
        direction: tuple[float, float, float],
        epsilon: float = 1e-8,
    ) -> DistanceMapDocument:
        return distance_map_from_mesh(
            mesh,
            width=width,
            height=height,
            origin=origin,
            x_range=x_range,
            y_range=y_range,
            direction=direction,
            epsilon=epsilon,
        )

    def distance_map_from_tiff(self, path: str | Path) -> DistanceMapDocument:
        return distance_map_from_tiff(path)

    def distance_map_to_tiff(self, distance_map: DistanceMapDocument, path: str | Path) -> Path:
        return distance_map_to_tiff(distance_map, path)

    def distance_map_to_iso_segments(
        self,
        distance_map: DistanceMapDocument,
        *,
        iso_value: float,
    ) -> IsoLineSegmentsDocument:
        return distance_map_to_iso_segments(distance_map, iso_value=iso_value)

    def distance_map_merge(
        self,
        left: DistanceMapDocument,
        right: DistanceMapDocument,
        *,
        mode: str,
    ) -> DistanceMapDocument:
        return distance_map_merge(left, right, mode=mode)

    def distance_map_contour_boolean(
        self,
        contours_a,
        contours_b,
        *,
        mode: str,
        width: int,
        height: int,
        origin: tuple[float, float] = (0.0, 0.0),
        pixel_size: float | tuple[float, float] = 1.0,
        iso_value: float = 0.0,
    ) -> IsoLineSegmentsDocument:
        return distance_map_contour_boolean(
            contours_a,
            contours_b,
            mode=mode,
            width=width,
            height=height,
            origin=origin,
            pixel_size=pixel_size,
            iso_value=iso_value,
        )

    def object_lines_from_contours(
        self,
        contours,
        *,
        line_width: float = 1.0,
        show_points: int = 0,
        smooth_connections: int = 0,
    ) -> ObjectLinesDocument:
        return object_lines_from_contours(
            contours,
            line_width=line_width,
            show_points=show_points,
            smooth_connections=smooth_connections,
        )

    def object_lines_to_contours(self, document: ObjectLinesDocument | dict) -> list[list[tuple[float, float, float]]]:
        return object_lines_to_contours(document)

    def offset_contours(
        self,
        contours,
        *,
        offset: float | None = None,
        offsets=None,
        min_angle_precision: float = float(np.pi / 9.0),
        mode: str = "offset",
        end_type: str = "round",
        corner_type: str = "round",
        max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
        z_restore: str = "default",
        z_value: float | None = None,
        z_values=None,
        relax_iterations: int = 1,
    ) -> list[list[tuple[float, float, float]]]:
        return offset_contours(
            contours,
            offset=offset,
            offsets=offsets,
            min_angle_precision=min_angle_precision,
            mode=mode,
            end_type=end_type,
            corner_type=corner_type,
            max_sharp_angle=max_sharp_angle,
            z_restore=z_restore,
            z_value=z_value,
            z_values=z_values,
            relax_iterations=relax_iterations,
        )

    def offset_contours_with_origins(
        self,
        contours,
        *,
        offset: float | None = None,
        offsets=None,
        min_angle_precision: float = float(np.pi / 9.0),
        mode: str = "offset",
        end_type: str = "round",
        corner_type: str = "round",
        max_sharp_angle: float = float(np.pi * 2.0 / 3.0),
        z_restore: str = "default",
        z_value: float | None = None,
        z_values=None,
        relax_iterations: int = 1,
    ) -> dict:
        return offset_contours_with_origins(
            contours,
            offset=offset,
            offsets=offsets,
            min_angle_precision=min_angle_precision,
            mode=mode,
            end_type=end_type,
            corner_type=corner_type,
            max_sharp_angle=max_sharp_angle,
            z_restore=z_restore,
            z_value=z_value,
            z_values=z_values,
            relax_iterations=relax_iterations,
        )

    def object_lines_to_pts(self, document: ObjectLinesDocument | dict) -> str:
        return object_lines_to_pts(document)

    def object_lines_from_pts(self, source: str) -> ObjectLinesDocument:
        return object_lines_from_pts(source)

    def object_lines_to_dxf(self, document: ObjectLinesDocument | dict) -> str:
        return object_lines_to_dxf(document)

    def object_lines_to_mrlines(self, document: ObjectLinesDocument | dict) -> bytes:
        return object_lines_to_mrlines(document)

    def object_lines_from_mrlines(self, source: bytes) -> ObjectLinesDocument:
        return object_lines_from_mrlines(source)

    def object_lines_to_ply(self, document: ObjectLinesDocument | dict) -> bytes:
        return object_lines_to_ply(document)

    def object_lines_from_ply(self, source: bytes) -> ObjectLinesDocument:
        return object_lines_from_ply(source)

    def object_lines_from_svg(self, source: str) -> ObjectLinesDocument:
        return object_lines_from_svg(source)

    def basic_repair(self, mesh: MeshDocument, *, merge_tolerance: float = 1e-6, area_epsilon: float = 1e-12) -> tuple[MeshDocument, RepairReport]:
        return basic_repair(mesh, merge_tolerance=merge_tolerance, area_epsilon=area_epsilon)

    def unite_close_vertices(
        self,
        mesh: MeshDocument,
        *,
        close_dist: float = 0.0,
        unite_only_boundary: bool = True,
    ) -> tuple[MeshDocument, int]:
        return unite_close_vertices(mesh, close_dist=close_dist, unite_only_boundary=unite_only_boundary)

    def mesh_healer_diagnostics(
        self,
        mesh: MeshDocument,
        *,
        merge_tolerance: float = 1e-6,
        area_epsilon: float = 1e-12,
        detect_self_intersections: bool = True,
        max_self_intersection_faces: int | None = 50000,
        epsilon: float = 1e-8,
    ) -> MeshHealerReport:
        return mesh_healer_diagnostics(
            mesh,
            merge_tolerance=merge_tolerance,
            area_epsilon=area_epsilon,
            detect_self_intersections=detect_self_intersections,
            max_self_intersection_faces=max_self_intersection_faces,
            epsilon=epsilon,
        )

    def tunnel_diagnostics(self, mesh: MeshDocument) -> TunnelDiagnostics:
        return tunnel_diagnostics(mesh)

    def detect_tunnel_faces(self, mesh: MeshDocument) -> list[int]:
        return detect_tunnel_faces(mesh)

    def eliminate_tunnels(self, mesh: MeshDocument) -> tuple[MeshDocument, TunnelEliminationReport]:
        return eliminate_tunnels(mesh)

    def prune_small_components(
        self,
        mesh: MeshDocument,
        *,
        min_area_mm2: float = 0.0,
    ) -> tuple[MeshDocument, ComponentPruneReport]:
        return prune_small_components(mesh, min_area_mm2=min_area_mm2)

    def weld_coincident_vertices(
        self,
        mesh: MeshDocument,
        *,
        eps_mm: float = 1e-5,
    ) -> tuple[MeshDocument, dict]:
        """Merge coincident vertex records and drop the degenerate faces that
        result. Closes zero-area pinch-point pseudo-holes left by voxel/marching
        output so hollow results come out strictly watertight."""
        return weld_coincident_vertices(mesh, eps_mm=eps_mm)

    def short_edge_diagnostics(self, mesh: MeshDocument, *, critical_length_mm: float) -> ShortEdgeDiagnostics:
        return short_edge_diagnostics(mesh, critical_length_mm=critical_length_mm)

    def degenerate_face_diagnostics(
        self,
        mesh: MeshDocument,
        *,
        critical_aspect_ratio: float,
    ) -> DegenerateFaceDiagnostics:
        return degenerate_face_diagnostics(mesh, critical_aspect_ratio=critical_aspect_ratio)

    def multiple_edge_diagnostics(self, mesh: MeshDocument) -> MultipleEdgeDiagnostics:
        return multiple_edge_diagnostics(mesh)

    def repair_multiple_edges(self, mesh: MeshDocument) -> tuple[MeshDocument, MultipleEdgeRepairReport]:
        return repair_multiple_edges(mesh)

    def repair_nonmanifold_edges(self, mesh: MeshDocument) -> tuple[MeshDocument, NonManifoldEdgeRepairReport]:
        return repair_nonmanifold_edges(mesh)

    def duplicate_nonmanifold_vertices(
        self,
        mesh: MeshDocument,
        *,
        region_face_indices: Sequence[int] | None = None,
    ) -> tuple[MeshDocument, DuplicateNonManifoldVerticesReport]:
        return duplicate_nonmanifold_vertices(mesh, region_face_indices=region_face_indices)

    def duplicate_multi_hole_vertices(
        self,
        mesh: MeshDocument,
    ) -> tuple[MeshDocument, DuplicateMultiHoleVerticesReport]:
        return duplicate_multi_hole_vertices(mesh)

    def not_smooth_face_diagnostics(
        self,
        mesh: MeshDocument,
        *,
        min_angle_radians: float = 0.3,
    ) -> NotSmoothFaceDiagnostics:
        return not_smooth_face_diagnostics(mesh, min_angle_radians=min_angle_radians)

    def find_disoriented_faces(
        self,
        mesh: MeshDocument,
        *,
        ray_mode: str = "shallowest",
        epsilon: float = 1e-8,
    ) -> list[int]:
        return find_disoriented_faces(mesh, ray_mode=ray_mode, epsilon=epsilon)

    def crease_edge_diagnostics(
        self,
        mesh: MeshDocument,
        *,
        angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
        min_component_length_mm: float | None = None,
        min_branch_length_mm: float | None = None,
    ) -> CreaseEdgeDiagnostics:
        return crease_edge_diagnostics(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            min_component_length_mm=min_component_length_mm,
            min_branch_length_mm=min_branch_length_mm,
        )

    def crease_repair_plan_diagnostics(
        self,
        mesh: MeshDocument,
        *,
        angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
        critical_tri_aspect_ratio: float = 1e3,
    ) -> CreaseRepairPlanDiagnostics:
        return crease_repair_plan_diagnostics(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            critical_tri_aspect_ratio=critical_tri_aspect_ratio,
        )

    def fix_mesh_creases(
        self,
        mesh: MeshDocument,
        *,
        angle_from_planar_radians: float = DEFAULT_CREASE_ANGLE_FROM_PLANAR_RADIANS,
        critical_tri_aspect_ratio: float = 1e3,
        max_iters: int = 10,
    ) -> tuple[MeshDocument, FixMeshCreasesReport]:
        return fix_mesh_creases(
            mesh,
            angle_from_planar_radians=angle_from_planar_radians,
            critical_tri_aspect_ratio=critical_tri_aspect_ratio,
            max_iters=max_iters,
        )

    def orient_faces_outward(self, mesh: MeshDocument) -> MeshDocument:
        return orient_faces_outward(mesh)

    def flip_normals(self, mesh: MeshDocument) -> MeshDocument:
        return flip_normals(mesh)

    def fill_planar_holes(self, mesh: MeshDocument, *, max_edges: int | None = None) -> tuple[MeshDocument, HoleFillReport]:
        return fill_planar_holes(mesh, max_edges=max_edges)

    def hole_fill_plan_diagnostics(self, mesh: MeshDocument, *, max_edges: int | None = None) -> HoleFillPlanDiagnostics:
        return hole_fill_plan_diagnostics(mesh, max_edges=max_edges)

    def repeated_hole_boundary_vertices_diagnostics(
        self,
        mesh: MeshDocument,
    ) -> RepeatedHoleBoundaryVerticesDiagnostics:
        return repeated_hole_boundary_vertices_diagnostics(mesh)

    def hole_complicating_faces_diagnostics(self, mesh: MeshDocument) -> HoleComplicatingFacesDiagnostics:
        return hole_complicating_faces_diagnostics(mesh)

    def remove_hole_complicating_faces(
        self,
        mesh: MeshDocument,
    ) -> tuple[MeshDocument, RemoveHoleComplicatingFacesReport]:
        return remove_hole_complicating_faces(mesh)

    def service_fill_holes(
        self,
        mesh: MeshDocument,
        *,
        max_edges: int | None = None,
        max_polygon_subdivisions: int | None = None,
        multiple_edges_resolve_mode: str | None = None,
        make_degenerate_band: bool = False,
        stop_before_bad_triangulation: bool = False,
        smooth_bd: bool = True,
        fill_metric: str | None = None,
        fill_metric_up_dir: tuple[float, float, float] | list[float] | None = None,
    ) -> tuple[MeshDocument, HoleFillReport]:
        return service_fill_holes(
            mesh,
            max_edges=max_edges,
            max_polygon_subdivisions=max_polygon_subdivisions,
            multiple_edges_resolve_mode=multiple_edges_resolve_mode,
            make_degenerate_band=make_degenerate_band,
            stop_before_bad_triangulation=stop_before_bad_triangulation,
            smooth_bd=smooth_bd,
            fill_metric=fill_metric,
            fill_metric_up_dir=fill_metric_up_dir,
        )

    def fix_self_intersections_relax(
        self,
        mesh: MeshDocument,
        *,
        relax_iterations: int = 5,
        max_expand: int = 3,
        touch_is_intersection: bool = True,
        force: float = 0.5,
        epsilon: float = 1e-8,
    ) -> tuple[MeshDocument, FixSelfIntersectionsRelaxReport]:
        return fix_self_intersections_relax(
            mesh,
            relax_iterations=relax_iterations,
            max_expand=max_expand,
            touch_is_intersection=touch_is_intersection,
            force=force,
            epsilon=epsilon,
        )

    def rebuild_via_sdf(
        self,
        mesh: MeshDocument,
        *,
        voxel_size_mm: float,
        offset_mm: float = 0.0,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = True,
    ) -> tuple[MeshDocument, VoxelRebuildReport]:
        return rebuild_via_sdf(
            mesh,
            voxel_size_mm=voxel_size_mm,
            offset_mm=offset_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def compare(self, source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
        return compare_summary(source, target)

    def compare_field(self, source: MeshDocument, target: MeshDocument) -> np.ndarray:
        return nearest_surface_distances(source, target)

    def signed_compare(self, source: MeshDocument, target: MeshDocument) -> dict[str, float | None]:
        return signed_compare_summary(source, target)

    def version_compare(self, source: MeshDocument, target: MeshDocument) -> VersionCompareSummary:
        return version_compare_summary(source, target)

    def version_compare_field(self, source: MeshDocument, target: MeshDocument) -> np.ndarray:
        return version_compare_distances(source, target)

    def service_compare_field(self, source: MeshDocument, other: MeshDocument) -> np.ndarray:
        return service_compare_distances(source, other)

    def service_compare(self, source: MeshDocument, other: MeshDocument) -> VersionCompareSummary:
        return service_compare_summary(source, other)

    def signed_compare_field(self, source: MeshDocument, target: MeshDocument) -> np.ndarray:
        return signed_surface_distances(source, target)

    def exact_mesh_intersections(
        self,
        first: MeshDocument,
        second: MeshDocument,
        *,
        leaf_size: int = 16,
        epsilon: float = 1e-8,
        first_intersection_only: bool = False,
        max_pairs: int | None = None,
    ) -> MeshCollisionResult:
        return exact_mesh_intersections_op(
            first,
            second,
            leaf_size=leaf_size,
            epsilon=epsilon,
            first_intersection_only=first_intersection_only,
            max_pairs=max_pairs,
        )

    def self_intersecting_faces(
        self,
        mesh: MeshDocument,
        *,
        epsilon: float = 1e-8,
        touch_is_intersection: bool = True,
    ) -> set[int]:
        return self_intersecting_faces_op(
            mesh,
            epsilon=epsilon,
            touch_is_intersection=touch_is_intersection,
        )

    def closest_points_on_mesh(self, points, mesh: MeshDocument):
        return closest_points_on_mesh(points, mesh)

    def feature_pair_measurements(self, features, pairs) -> list[dict[str, object]]:
        return feature_pair_measurements(features, pairs)

    def feature_object_descriptors(
        self,
        features,
        *,
        infinite_extent_mm: float = 1000.0,
    ) -> list[dict[str, object]]:
        return feature_object_descriptors(features, infinite_extent_mm=infinite_extent_mm)

    def refine_feature_primitives(
        self,
        mesh: MeshDocument,
        features,
        *,
        distance_limit_mm: float = 0.1,
        normal_tolerance_degrees: float = 30.0,
        max_iterations: int = 10,
    ) -> list[dict[str, object]]:
        return refine_feature_primitives(
            mesh,
            features,
            distance_limit_mm=distance_limit_mm,
            normal_tolerance_degrees=normal_tolerance_degrees,
            max_iterations=max_iterations,
        )

    def mesh_geodesic_path(
        self,
        mesh: MeshDocument,
        *,
        start_vertex: int,
        end_vertex: int,
        max_path_len_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_geodesic_path(
            mesh,
            start_vertex=start_vertex,
            end_vertex=end_vertex,
            max_path_len_mm=max_path_len_mm,
        )

    def mesh_geodesic_polyline_path(
        self,
        mesh: MeshDocument,
        *,
        control_vertices,
        close_path: bool = False,
        max_path_len_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_geodesic_polyline_path(
            mesh,
            control_vertices=control_vertices,
            close_path=close_path,
            max_path_len_mm=max_path_len_mm,
        )

    def mesh_cut_measure_contours(
        self,
        mesh: MeshDocument,
        *,
        control_vertices,
        close_path: bool = False,
        max_path_len_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_cut_measure_contours(
            mesh,
            control_vertices=control_vertices,
            close_path=close_path,
            max_path_len_mm=max_path_len_mm,
        )

    def mesh_cut_measure_edge_path_topology_cut(
        self,
        mesh: MeshDocument,
        *,
        control_vertices,
        close_path: bool = False,
        max_path_len_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_cut_measure_edge_path_topology_cut(
            mesh,
            control_vertices=control_vertices,
            close_path=close_path,
            max_path_len_mm=max_path_len_mm,
        )

    def mesh_geodesic_quadrangle_path(
        self,
        mesh: MeshDocument,
        *,
        start_vertex: int,
        end_vertex: int,
    ) -> dict[str, object]:
        return mesh_geodesic_quadrangle_path(
            mesh,
            start_vertex=start_vertex,
            end_vertex=end_vertex,
        )

    def mesh_planar_triangle_strip_path(
        self,
        *,
        start,
        portals,
        end,
    ) -> dict[str, object]:
        return mesh_planar_triangle_strip_path(
            start=start,
            portals=portals,
            end=end,
        )

    def mesh_surface_edge_point_path(
        self,
        mesh: MeshDocument,
        *,
        edges,
        positions,
    ) -> dict[str, object]:
        return mesh_surface_edge_point_path(
            mesh,
            edges=edges,
            positions=positions,
        )

    def mesh_geodesic_edge_point_path(
        self,
        mesh: MeshDocument,
        *,
        start_point,
        edges,
        positions,
        end_point,
    ) -> dict[str, object]:
        return mesh_geodesic_edge_point_path(
            mesh,
            start_point=start_point,
            edges=edges,
            positions=positions,
            end_point=end_point,
        )

    def mesh_triangle_strip_unfolded_path(
        self,
        mesh: MeshDocument,
        *,
        start_face_index: int,
        crossed_edges,
        end_face_index: int,
        start_point,
        end_point,
    ) -> dict[str, object]:
        return mesh_triangle_strip_unfolded_path(
            mesh,
            start_face_index=start_face_index,
            crossed_edges=crossed_edges,
            end_face_index=end_face_index,
            start_point=start_point,
            end_point=end_point,
        )

    def mesh_steepest_descent_triangle_step(
        self,
        mesh: MeshDocument,
        *,
        vertex_scalars,
        face_index: int,
        start_barycentric,
    ) -> dict[str, object]:
        return mesh_steepest_descent_triangle_step(
            mesh,
            vertex_scalars=vertex_scalars,
            face_index=face_index,
            start_barycentric=start_barycentric,
        )

    def mesh_steepest_descent_edge_step(
        self,
        mesh: MeshDocument,
        *,
        vertex_scalars,
        edge,
        edge_position: float,
    ) -> dict[str, object]:
        return mesh_steepest_descent_edge_step(
            mesh,
            vertex_scalars=vertex_scalars,
            edge=edge,
            edge_position=edge_position,
        )

    def mesh_steepest_descent_vertex_step(
        self,
        mesh: MeshDocument,
        *,
        vertex_scalars,
        vertex_index: int,
    ) -> dict[str, object]:
        return mesh_steepest_descent_vertex_step(
            mesh,
            vertex_scalars=vertex_scalars,
            vertex_index=vertex_index,
        )

    def mesh_steepest_descent_path(
        self,
        mesh: MeshDocument,
        *,
        vertex_scalars,
        face_index: int,
        start_barycentric,
        max_steps: int = 1024,
    ) -> dict[str, object]:
        return mesh_steepest_descent_path(
            mesh,
            vertex_scalars=vertex_scalars,
            face_index=face_index,
            start_barycentric=start_barycentric,
            max_steps=max_steps,
        )

    def mesh_fast_marching_surface_path(
        self,
        mesh: MeshDocument,
        *,
        start_vertex: int,
        end_vertex: int,
        max_steps: int = 1024,
    ) -> dict[str, object]:
        return mesh_fast_marching_surface_path(
            mesh,
            start_vertex=start_vertex,
            end_vertex=end_vertex,
            max_steps=max_steps,
        )

    def mesh_fast_marching_surface_path_tri_points(
        self,
        mesh: MeshDocument,
        *,
        start_face_index: int,
        start_barycentric: tuple[float, float, float],
        end_face_index: int,
        end_barycentric: tuple[float, float, float],
        max_steps: int = 1024,
    ) -> dict[str, object]:
        return mesh_fast_marching_surface_path_tri_points(
            mesh,
            start_face_index=start_face_index,
            start_barycentric=start_barycentric,
            end_face_index=end_face_index,
            end_barycentric=end_barycentric,
            max_steps=max_steps,
        )

    def mesh_surface_path_tri_points(
        self,
        mesh: MeshDocument,
        *,
        start_face_index: int,
        start_barycentric: tuple[float, float, float],
        end_face_index: int,
        end_barycentric: tuple[float, float, float],
        max_geodesic_iters: int = 5,
    ) -> dict[str, object]:
        return mesh_surface_path_tri_points(
            mesh,
            start_face_index=start_face_index,
            start_barycentric=start_barycentric,
            end_face_index=end_face_index,
            end_barycentric=end_barycentric,
            max_geodesic_iters=max_geodesic_iters,
        )

    def mesh_geodesic_distance_field(
        self,
        mesh: MeshDocument,
        *,
        seed_vertices,
        max_distance_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_geodesic_distance_field(
            mesh,
            seed_vertices=seed_vertices,
            max_distance_mm=max_distance_mm,
        )

    def mesh_closest_surface_path_targets(
        self,
        mesh: MeshDocument,
        *,
        start_vertices,
        end_vertices,
        max_distance_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_closest_surface_path_targets(
            mesh,
            start_vertices=start_vertices,
            end_vertices=end_vertices,
            max_distance_mm=max_distance_mm,
        )

    def mesh_surface_distance_seed_vertices(
        self,
        mesh: MeshDocument,
        *,
        seed_vertices=None,
        seed_edges=None,
        seed_face_ids=None,
    ) -> dict[str, object]:
        return mesh_surface_distance_seed_vertices(
            mesh,
            seed_vertices=seed_vertices,
            seed_edges=seed_edges,
            seed_face_ids=seed_face_ids,
        )

    def mesh_geodesic_iso_region(
        self,
        mesh: MeshDocument,
        *,
        seed_vertices,
        iso_value_mm: float,
        max_distance_mm: float | None = None,
    ) -> dict[str, object]:
        return mesh_geodesic_iso_region(
            mesh,
            seed_vertices=seed_vertices,
            iso_value_mm=iso_value_mm,
            max_distance_mm=max_distance_mm,
        )

    def mesh_geodesic_extreme_edges(
        self,
        mesh: MeshDocument,
        *,
        scalars,
        extreme_type: str = "ridge",
    ) -> dict[str, object]:
        return mesh_geodesic_extreme_edges(
            mesh,
            scalars=scalars,
            extreme_type=extreme_type,
        )

    def point_mesh_distances(self, points, mesh: MeshDocument):
        return point_mesh_distances(points, mesh)

    def section_contour(
        self,
        mesh: MeshDocument,
        *,
        section_constant: float,
        plane_axis: tuple[float, float, float] = (0.0, 1.0, 0.0),
        selected_vertex_indices: list[int] | np.ndarray | None = None,
        epsilon: float = 1e-5,
    ) -> SectionContourPayload:
        return section_contour(
            mesh,
            section_constant=section_constant,
            plane_axis=plane_axis,
            selected_vertex_indices=selected_vertex_indices,
            epsilon=epsilon,
        )

    def ray_thickness(self, mesh: MeshDocument, *, threshold_mm: float = 0.6) -> tuple[np.ndarray, ThicknessSummary]:
        field = ray_thickness_at_vertices(mesh)
        return field, summarize_thickness(field, threshold_mm=threshold_mm)

    def service_thickness(self, mesh: MeshDocument, *, threshold_mm: float = 0.6) -> tuple[np.ndarray, ThicknessSummary]:
        field = service_thickness_at_vertices(mesh)
        return field, summarize_thickness(field, threshold_mm=threshold_mm)

    def save_thickness_npz(self, path: str | Path, thickness: np.ndarray, *, vertex_count: int, threshold_mm: float) -> Path:
        return save_thickness_npz(path, thickness, vertex_count=vertex_count, threshold_mm=threshold_mm)

    def save_compare_npz(self, path: str | Path, values: np.ndarray, *, vertex_count: int, other_version_id: str) -> Path:
        return save_compare_npz(path, values, vertex_count=vertex_count, other_version_id=other_version_id)

    def thickness_overlay_payload(self, path: str | Path) -> dict[str, object]:
        return thickness_overlay_payload(path)

    def compare_overlay_payload(self, path: str | Path, *, other_version_id: str | None = None) -> dict[str, object]:
        return compare_overlay_payload(path, other_version_id=other_version_id)

    def sample_sdf_grid(
        self,
        mesh: MeshDocument,
        *,
        voxel_size_mm: float,
        padding_mm: float | None = None,
    ) -> SDFGrid:
        return sample_sdf_grid(mesh, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm)

    def sdf_occupancy(self, grid: SDFGrid, *, iso_value: float = 0.0) -> np.ndarray:
        return sdf_occupancy(grid, iso_value=iso_value)

    def estimate_sdf_volume(self, grid: SDFGrid, *, iso_value: float = 0.0) -> float:
        return estimate_sdf_volume(grid, iso_value=iso_value)

    def sdf_union(self, a: SDFGrid, b: SDFGrid) -> SDFGrid:
        return sdf_union(a, b)

    def sdf_intersection(self, a: SDFGrid, b: SDFGrid) -> SDFGrid:
        return sdf_intersection(a, b)

    def sdf_difference(self, a: SDFGrid, b: SDFGrid) -> SDFGrid:
        return sdf_difference(a, b)

    def sdf_offset(self, grid: SDFGrid, offset_mm: float) -> SDFGrid:
        return sdf_offset(grid, offset_mm)

    def sdf_shell(self, grid: SDFGrid, wall_thickness_mm: float) -> SDFGrid:
        return sdf_shell(grid, wall_thickness_mm)

    def voxel_binary_values(self, a: SDFGrid, b: SDFGrid, *, operation: str) -> SDFGrid:
        return voxel_binary_values(a, b, operation=operation)

    def voxel_binary_iso_value(
        self,
        left_iso_value: float,
        right_iso_value: float,
        *,
        operation: str,
    ) -> float:
        return voxel_binary_iso_value(left_iso_value, right_iso_value, operation=operation)

    def voxel_binary_operation(
        self,
        a: SDFGrid,
        b: SDFGrid,
        *,
        operation: str,
        left_iso_value: float = 0.0,
        right_iso_value: float = 0.0,
    ) -> tuple[SDFGrid, float]:
        return voxel_binary_operation(
            a,
            b,
            operation=operation,
            left_iso_value=left_iso_value,
            right_iso_value=right_iso_value,
        )

    def voxel_default_iso_value(self, values) -> float:
        return voxel_default_iso_value(values)

    def voxel_path(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        start: tuple[int, int, int],
        finish: tuple[int, int, int],
        metric: str = "difference",
        max_dist_ratio: float = 1.5,
        plane: str = "none",
        quarters_mask: int = 0x0F,
        exponent_modifier: float = -1.0,
    ) -> VoxelPathResult:
        return voxel_path(
            volume_or_values,
            shape=shape,
            start=start,
            finish=finish,
            metric=metric,
            max_dist_ratio=max_dist_ratio,
            plane=plane,
            quarters_mask=quarters_mask,
            exponent_modifier=exponent_modifier,
        )

    def voxel_path_build_four(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        start: tuple[int, int, int],
        finish: tuple[int, int, int],
        metric: str = "exponent",
        max_dist_ratio: float = 1.5,
        plane: str = "none",
        exponent_modifier: float = -1.0,
    ) -> list[dict[str, object]]:
        return voxel_path_build_four(
            volume_or_values,
            shape=shape,
            start=start,
            finish=finish,
            metric=metric,
            max_dist_ratio=max_dist_ratio,
            plane=plane,
            exponent_modifier=exponent_modifier,
        )

    def voxel_slice(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        plane: str,
        slice_index: int,
        min_value: float,
        max_value: float,
    ) -> VoxelSliceResult:
        return voxel_slice(
            volume_or_values,
            shape=shape,
            plane=plane,
            slice_index=slice_index,
            min_value=min_value,
            max_value=max_value,
        )

    def voxel_line_graph(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        axis: str,
        fixed_coordinate: tuple[int, int, int],
    ) -> VoxelLineGraphResult:
        return voxel_line_graph(
            volume_or_values,
            shape=shape,
            axis=axis,
            fixed_coordinate=fixed_coordinate,
        )

    def voxel_active_box(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        min_corner: tuple[int, int, int],
        dimensions: tuple[int, int, int],
    ) -> VoxelActiveBoxResult:
        return voxel_active_box(
            volume_or_values,
            shape=shape,
            min_corner=min_corner,
            dimensions=dimensions,
        )

    def voxel_volume_render_data(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        voxel_size: tuple[float, float, float] | None = None,
        active_min_corner: tuple[int, int, int] = (0, 0, 0),
        active_dimensions: tuple[int, int, int] | None = None,
        source_min_value: float | None = None,
        source_max_value: float | None = None,
    ) -> VoxelVolumeRenderDataResult:
        return voxel_volume_render_data(
            volume_or_values,
            shape=shape,
            voxel_size=voxel_size,
            active_min_corner=active_min_corner,
            active_dimensions=active_dimensions,
            source_min_value=source_min_value,
            source_max_value=source_max_value,
        )

    def voxel_volume_render_lut(
        self,
        *,
        lut_type: str = "rainbow",
        alpha_type: str = "constant",
        alpha_limit: int = 10,
        one_color: tuple[int, int, int, int] | None = None,
    ) -> VoxelVolumeRenderLutResult:
        return voxel_volume_render_lut(
            lut_type=lut_type,
            alpha_type=alpha_type,
            alpha_limit=alpha_limit,
            one_color=one_color,
        )

    def voxel_volume_render_ray(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        voxel_size: tuple[float, float, float] | None = None,
        min_corner: tuple[int, int, int] = (0, 0, 0),
        ray_start: tuple[float, float, float],
        ray_direction: tuple[float, float, float],
        sampling_step: float,
        min_value: float = 0.0,
        max_value: float = 1.0,
        lut_type: str = "rainbow",
        alpha_type: str = "constant",
        alpha_limit: int = 10,
        one_color: tuple[int, int, int, int] | None = None,
        clipping_plane: tuple[float, float, float, float] | None = None,
        shading_mode: str = "none",
        light_pos_eye: tuple[float, float, float] | None = None,
        ambient_strength: float = 0.1,
        specular_strength: float = 0.5,
        spec_exp: float = 35.0,
        active_indices: tuple[int, ...] | None = None,
        max_steps: int = 4096,
    ) -> VoxelVolumeRenderRayResult:
        return voxel_volume_render_ray(
            volume_or_values,
            shape=shape,
            voxel_size=voxel_size,
            min_corner=min_corner,
            ray_start=ray_start,
            ray_direction=ray_direction,
            sampling_step=sampling_step,
            min_value=min_value,
            max_value=max_value,
            lut_type=lut_type,
            alpha_type=alpha_type,
            alpha_limit=alpha_limit,
            one_color=one_color,
            clipping_plane=clipping_plane,
            shading_mode=shading_mode,
            light_pos_eye=light_pos_eye,
            ambient_strength=ambient_strength,
            specular_strength=specular_strength,
            spec_exp=spec_exp,
            active_indices=active_indices,
            max_steps=max_steps,
        )

    def voxel_segmentation(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        inside_seeds: Sequence[tuple[int, int, int]],
        outside_seeds: Sequence[tuple[int, int, int]] = (),
        exponent_modifier: float = 3000.0,
        voxels_expansion: int = 25,
        include_boundary_outside: bool = True,
    ) -> VoxelSegmentationResult:
        return voxel_segmentation(
            volume_or_values,
            shape=shape,
            inside_seeds=inside_seeds,
            outside_seeds=outside_seeds,
            exponent_modifier=exponent_modifier,
            voxels_expansion=voxels_expansion,
            include_boundary_outside=include_boundary_outside,
        )

    def voxel_segmentation_mesh(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        inside_seeds: Sequence[tuple[int, int, int]],
        outside_seeds: Sequence[tuple[int, int, int]] = (),
        exponent_modifier: float = 3000.0,
        voxels_expansion: int = 25,
        include_boundary_outside: bool = True,
        voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0),
    ) -> MeshDocument:
        return voxel_segmentation_mesh(
            volume_or_values,
            shape=shape,
            inside_seeds=inside_seeds,
            outside_seeds=outside_seeds,
            exponent_modifier=exponent_modifier,
            voxels_expansion=voxels_expansion,
            include_boundary_outside=include_boundary_outside,
            voxel_size=voxel_size,
        )

    def voxel_mask_to_mesh(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        mask_coordinates: Sequence[tuple[int, int, int]],
        voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0),
        mask_expansion: int = 25,
        smooth_band_radius: int = 3,
    ) -> MeshDocument:
        return voxel_mask_to_mesh(
            volume_or_values,
            shape=shape,
            mask_coordinates=mask_coordinates,
            voxel_size=voxel_size,
            mask_expansion=mask_expansion,
            smooth_band_radius=smooth_band_radius,
        )

    def voxel_to_mesh_simple(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        voxel_size: tuple[float, float, float] | None = None,
        iso_value: float | None = None,
    ) -> MeshDocument:
        return voxel_to_mesh_simple(
            volume_or_values,
            shape=shape,
            voxel_size=voxel_size,
            iso_value=iso_value,
        )

    def voxel_volume_from_meshlib_values(
        self,
        values,
        *,
        dimensions: tuple[int, int, int],
        voxel_size: tuple[float, float, float],
        grid_level_set: bool,
        scalar_type: str,
        iso_value: float | None = None,
        min_value: float | None = None,
        max_value: float | None = None,
    ):
        return voxel_volume_from_meshlib_values(
            values,
            dimensions=dimensions,
            voxel_size=voxel_size,
            grid_level_set=grid_level_set,
            scalar_type=scalar_type,
            iso_value=iso_value,
            min_value=min_value,
            max_value=max_value,
        )

    def voxel_to_mesh_dual(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        voxel_size: tuple[float, float, float] | None = None,
        iso_value: float | None = None,
        adaptivity: float = 0.0,
        max_faces: int | None = None,
        max_vertices: int | None = None,
        relax_disoriented_triangles: bool = True,
    ) -> MeshDocument:
        return voxel_to_mesh_dual(
            volume_or_values,
            shape=shape,
            voxel_size=voxel_size,
            iso_value=iso_value,
            adaptivity=adaptivity,
            max_faces=max_faces,
            max_vertices=max_vertices,
            relax_disoriented_triangles=relax_disoriented_triangles,
        )

    def voxel_to_mesh_dual_vdb_payload(
        self,
        model_bytes: bytes,
        *,
        dimensions: tuple[int, int, int] = (1, 1, 1),
        voxel_size: tuple[float, float, float] = (1.0, 1.0, 1.0),
        iso_value: float = 0.0,
        adaptivity: float = 0.0,
        max_faces: int | None = None,
        max_vertices: int | None = None,
        relax_disoriented_triangles: bool = True,
    ) -> MeshDocument:
        return voxel_to_mesh_dual_vdb_payload(
            model_bytes,
            dimensions=dimensions,
            voxel_size=voxel_size,
            iso_value=iso_value,
            adaptivity=adaptivity,
            max_faces=max_faces,
            max_vertices=max_vertices,
            relax_disoriented_triangles=relax_disoriented_triangles,
        )

    def voxel_move_mesh_to_max_deriv(
        self,
        mesh: MeshDocument,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        voxel_size: tuple[float, float, float] | None = None,
        iters: int = 30,
        sample_points: int = 6,
        degree: int = 3,
        outlier_threshold: float = 1.0,
        intermediate_smooth_force: float = 0.3,
        preparation_smooth_force: float = 0.1,
        smooth_shift_iterations: int = 15,
        final_relax_iterations: int = 15,
        final_relax_force: float = 0.01,
    ) -> MeshDocument:
        return voxel_move_mesh_to_max_deriv(
            mesh,
            volume_or_values,
            shape=shape,
            voxel_size=voxel_size,
            iters=iters,
            sample_points=sample_points,
            degree=degree,
            outlier_threshold=outlier_threshold,
            intermediate_smooth_force=intermediate_smooth_force,
            preparation_smooth_force=preparation_smooth_force,
            smooth_shift_iterations=smooth_shift_iterations,
            final_relax_iterations=final_relax_iterations,
            final_relax_force=final_relax_force,
        )

    def voxel_to_mesh_smart(
        self,
        volume_or_values,
        *,
        shape: tuple[int, int, int] | None = None,
        voxel_size: tuple[float, float, float] | None = None,
        iso_value: float | None = None,
        iters: int = 30,
        sample_points: int = 6,
        degree: int = 3,
        outlier_threshold: float = 1.0,
        intermediate_smooth_force: float = 0.3,
        preparation_smooth_force: float = 0.1,
        smooth_shift_iterations: int = 15,
        final_relax_iterations: int = 15,
        final_relax_force: float = 0.01,
    ) -> MeshDocument:
        return voxel_to_mesh_smart(
            volume_or_values,
            shape=shape,
            voxel_size=voxel_size,
            iso_value=iso_value,
            iters=iters,
            sample_points=sample_points,
            degree=degree,
            outlier_threshold=outlier_threshold,
            intermediate_smooth_force=intermediate_smooth_force,
            preparation_smooth_force=preparation_smooth_force,
            smooth_shift_iterations=smooth_shift_iterations,
            final_relax_iterations=final_relax_iterations,
            final_relax_force=final_relax_force,
        )

    def load_raw_voxels(
        self,
        path: str | Path,
        *,
        dimensions: tuple[int, int, int],
        voxel_size: tuple[float, float, float],
        scalar_type: str,
        grid_level_set: bool = False,
    ) -> VoxelVolume:
        return load_raw_voxels(
            path,
            dimensions=dimensions,
            voxel_size=voxel_size,
            scalar_type=scalar_type,
            grid_level_set=grid_level_set,
        )

    def load_raw_voxels_auto(self, path: str | Path) -> VoxelVolume:
        return load_raw_voxels_auto(path)

    def load_tiff_voxels_dir(
        self,
        directory: str | Path,
        *,
        voxel_size: tuple[float, float, float],
        grid_level_set: bool = False,
    ) -> VoxelVolume:
        return load_tiff_voxels_dir(
            directory,
            voxel_size=voxel_size,
            grid_level_set=grid_level_set,
        )

    def extract_sdf_surface(self, grid: SDFGrid, *, iso_value: float = 0.0) -> MeshDocument:
        return extract_surface_mesh(grid, iso_value=iso_value)

    def extract_sdf_isosurface(self, grid: SDFGrid, *, iso_value: float = 0.0) -> MeshDocument:
        return extract_marching_tetrahedra(grid, iso_value=iso_value)

    def voxel_offset_mesh(
        self,
        mesh: MeshDocument,
        *,
        offset_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_offset_mesh(mesh, offset_mm=offset_mm, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm, extractor=extractor, refine=refine)

    def voxel_shell_mesh(
        self,
        mesh: MeshDocument,
        *,
        wall_thickness_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_shell_mesh(mesh, wall_thickness_mm=wall_thickness_mm, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm, extractor=extractor, refine=refine)

    def voxel_thicken_mesh(
        self,
        mesh: MeshDocument,
        *,
        thickness_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_thicken_mesh(mesh, thickness_mm=thickness_mm, voxel_size_mm=voxel_size_mm, padding_mm=padding_mm, extractor=extractor, refine=refine)

    def voxel_weighted_shell_mesh(
        self,
        mesh: MeshDocument,
        *,
        regions: list[RegionEntry],
        region_weights: dict[str, float],
        offset_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        interpolation_distance_mm: float = 0.0,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_weighted_shell_mesh(
            mesh,
            regions=regions,
            region_weights=region_weights,
            offset_mm=offset_mm,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            interpolation_distance_mm=interpolation_distance_mm,
            extractor=extractor,
            refine=refine,
        )

    def voxel_partial_offset_mesh(
        self,
        mesh: MeshDocument,
        *,
        regions: list[RegionEntry],
        selected_region_ids: list[str],
        offset_mm: float,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_partial_offset_mesh(
            mesh,
            regions=regions,
            selected_region_ids=selected_region_ids,
            offset_mm=offset_mm,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def voxel_boolean_mesh(
        self,
        a: MeshDocument,
        b: MeshDocument,
        *,
        operation: str,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        origin_phase: tuple[float, float, float] | None = (0.125, 0.125, 0.125),
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return voxel_boolean_mesh(
            a,
            b,
            operation=operation,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            origin_phase=origin_phase,
            extractor=extractor,
            refine=refine,
        )

    def exact_boolean_mesh(
        self,
        a: MeshDocument,
        b: MeshDocument,
        *,
        operation: str,
        leaf_size: int = 16,
        epsilon: float = 1e-8,
    ) -> ExactBooleanMeshResult:
        return exact_boolean_mesh_op(
            a,
            b,
            operation=operation,
            leaf_size=leaf_size,
            epsilon=epsilon,
        )

    def global_thicken(self, mesh: MeshDocument, *, min_target_thickness_mm: float) -> MeshDocument:
        return global_thicken(mesh, min_target_thickness_mm=min_target_thickness_mm)

    def service_hollow(self, mesh: MeshDocument, *, wall_thickness_mm: float) -> MeshDocument:
        return service_hollow_mesh(mesh, wall_thickness_mm=wall_thickness_mm)

    def service_hollow_voxel_size(self, mesh: MeshDocument, *, wall_thickness_mm: float) -> float:
        return service_hollow_voxel_size(mesh, wall_thickness_mm=wall_thickness_mm)

    def manufacturability(self, mesh: MeshDocument, *, threshold_mm: float = 0.6) -> ManufacturabilityReport:
        return compute_manufacturability_report(mesh, threshold_mm=threshold_mm)

    def measure_ring(
        self,
        mesh: MeshDocument,
        *,
        axis_override: tuple[float, float, float] | np.ndarray | None = None,
    ) -> RingMeasurement:
        return measure_ring(mesh, axis_override=axis_override)

    def detect_ring_regions(
        self,
        mesh: MeshDocument,
        measurement: RingMeasurement,
        *,
        thickness: np.ndarray | None = None,
        threshold_mm: float = 0.6,
    ) -> list[RegionEntry]:
        return detect_ring_regions(mesh, measurement, thickness=thickness, threshold_mm=threshold_mm)

    def protected_hollow_scale_field(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        protect_region_ids: list[str],
        *,
        base_thickness_mm: float,
    ) -> np.ndarray:
        return protected_hollow_scale_field(mesh, regions, protect_region_ids, base_thickness_mm)

    def weighted_inner_offset_preview(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        protect_region_ids: list[str],
        *,
        wall_thickness_mm: float,
    ) -> MeshDocument:
        return weighted_inner_offset_preview(mesh, regions, protect_region_ids, wall_thickness_mm)

    def protected_hollow_mesh(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        protect_region_ids: list[str],
        *,
        wall_thickness_mm: float,
        voxel_size_mm: float = 0.5,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> MeshDocument:
        return protected_hollow_mesh(
            mesh,
            regions,
            protect_region_ids,
            wall_thickness_mm=wall_thickness_mm,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def plan_drain_holes(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        ring_axis: tuple[float, float, float] | np.ndarray,
        *,
        wall_thickness_mm: float,
        hole_diameter_mm: float = 0.8,
    ) -> list[DrainHolePlan]:
        return plan_drain_holes(
            mesh,
            regions,
            ring_axis,
            wall_thickness_mm=wall_thickness_mm,
            hole_diameter_mm=hole_diameter_mm,
        )

    def drain_hole_cutter_mesh(self, plan: DrainHolePlan, *, sections: int = 32) -> MeshDocument:
        return drain_hole_cutter_mesh(plan, sections=sections)

    def drain_hole_cutters_mesh(self, plans: list[DrainHolePlan], *, sections: int = 32) -> MeshDocument:
        return drain_hole_cutters_mesh(plans, sections=sections)

    def apply_drain_holes_voxel(
        self,
        shell_mesh: MeshDocument,
        plans: list[DrainHolePlan],
        *,
        voxel_size_mm: float,
        padding_mm: float | None = None,
        sections: int = 32,
        extractor: str = "marching",
    ) -> MeshDocument:
        return apply_drain_holes_voxel(
            shell_mesh,
            plans,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            sections=sections,
            extractor=extractor,
        )

    def adaptive_hollow_to_weight(
        self,
        mesh: MeshDocument,
        *,
        target_weight_g: float,
        material: str = "gold_18k",
        tolerance_g: float = 0.1,
        min_thickness_mm: float = 0.5,
        max_thickness_mm: float = 3.0,
        max_iterations: int = 20,
        voxel_size_mm: float = 0.5,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> tuple[MeshDocument, AdaptiveHollowReport]:
        return adaptive_hollow_to_weight(
            mesh,
            target_weight_g=target_weight_g,
            material=material,
            tolerance_g=tolerance_g,
            min_thickness_mm=min_thickness_mm,
            max_thickness_mm=max_thickness_mm,
            max_iterations=max_iterations,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def adaptive_protected_hollow_to_weight(
        self,
        mesh: MeshDocument,
        regions: list[RegionEntry],
        protect_region_ids: list[str],
        *,
        target_weight_g: float,
        material: str = "gold_18k",
        tolerance_g: float = 0.1,
        min_thickness_mm: float = 0.5,
        max_thickness_mm: float = 3.0,
        max_iterations: int = 20,
        voxel_size_mm: float = 0.5,
        padding_mm: float | None = None,
        extractor: str = "marching",
        refine: bool = False,
    ) -> tuple[MeshDocument, AdaptiveHollowReport]:
        return adaptive_protected_hollow_to_weight(
            mesh,
            regions,
            protect_region_ids,
            target_weight_g=target_weight_g,
            material=material,
            tolerance_g=tolerance_g,
            min_thickness_mm=min_thickness_mm,
            max_thickness_mm=max_thickness_mm,
            max_iterations=max_iterations,
            voxel_size_mm=voxel_size_mm,
            padding_mm=padding_mm,
            extractor=extractor,
            refine=refine,
        )

    def radial_scale(
        self,
        mesh: MeshDocument,
        scale_factor: float,
        *,
        ring_axis: np.ndarray | tuple[float, float, float] | None = None,
        preserve_indices: np.ndarray | None = None,
    ) -> MeshDocument:
        return radial_scale(mesh, scale_factor, ring_axis=ring_axis, preserve_indices=preserve_indices)

    def resize_ring(
        self,
        mesh: MeshDocument,
        *,
        current_size: float,
        target_size: float,
        ring_axis: np.ndarray | tuple[float, float, float] | None = None,
        preserve_indices: np.ndarray | None = None,
    ) -> MeshDocument:
        return resize_ring(
            mesh,
            current_size=current_size,
            target_size=target_size,
            ring_axis=ring_axis,
            preserve_indices=preserve_indices,
        )

    def fit_ring_to_diameter(
        self,
        mesh: MeshDocument,
        *,
        measured_diameter_mm: float,
        target_diameter_mm: float,
        ring_axis: np.ndarray | tuple[float, float, float] | None = None,
        preserve_indices: np.ndarray | None = None,
        max_preserve_scale_ratio: float = 1.5,
    ) -> RingFitResult:
        return fit_ring_to_diameter(
            mesh,
            measured_diameter_mm,
            target_diameter_mm,
            ring_axis=ring_axis,
            preserve_indices=preserve_indices,
            max_preserve_scale_ratio=max_preserve_scale_ratio,
        )

    def local_thicken(
        self,
        mesh: MeshDocument,
        seed_indices: np.ndarray,
        *,
        amount_mm: float,
        falloff_mm: float = 1.5,
    ) -> MeshDocument:
        return local_thicken(mesh, seed_indices, amount_mm=amount_mm, falloff_mm=falloff_mm)

    def local_thicken_to_minimum(
        self,
        mesh: MeshDocument,
        seed_indices: np.ndarray,
        thickness_values: np.ndarray,
        *,
        min_target_thickness_mm: float,
        falloff_mm: float = 1.5,
        deficit_scale: float = 0.75,
    ) -> MeshDocument:
        return local_thicken_to_minimum(
            mesh,
            seed_indices,
            thickness_values,
            min_target_thickness_mm=min_target_thickness_mm,
            falloff_mm=falloff_mm,
            deficit_scale=deficit_scale,
        )

    def local_scoop(
        self,
        mesh: MeshDocument,
        seed_indices: np.ndarray,
        *,
        depth_mm: float,
        falloff_mm: float = 1.5,
    ) -> MeshDocument:
        return local_scoop(mesh, seed_indices, depth_mm=depth_mm, falloff_mm=falloff_mm)

    def smooth(
        self,
        mesh: MeshDocument,
        *,
        iterations: int = 5,
        strength: float = 0.5,
        seed_indices: np.ndarray | None = None,
        falloff_mm: float = 1.8,
        nu: float = -0.53,
    ) -> MeshDocument:
        return smooth(mesh, iterations=iterations, strength=strength, seed_indices=seed_indices, falloff_mm=falloff_mm, nu=nu)

    def taubin_smooth(
        self,
        mesh: MeshDocument,
        *,
        iterations: int = 10,
        lamb: float = 0.5,
        nu: float = -0.53,
    ) -> MeshDocument:
        return taubin_smooth(mesh, iterations=iterations, lamb=lamb, nu=nu)

    def apply_brush_strokes(self, mesh: MeshDocument, strokes: list[BrushStroke]) -> MeshDocument:
        return apply_brush_strokes(mesh, strokes)

    def parse_gcode_paths(
        self,
        source: str,
        *,
        machine_settings: GcodeMachineSettings | dict | None = None,
    ) -> GcodePathDocument:
        return parse_gcode_paths(source, machine_settings=machine_settings)

    def load_gcode_source(self, path: str | Path) -> list[str]:
        return load_gcode_source(path)

    def write_gcode_source(self, source: list[str], path: str | Path) -> Path:
        return write_gcode_source(source, path)

    def parse_gcode_file_paths(
        self,
        path: str | Path,
        *,
        machine_settings: GcodeMachineSettings | dict | None = None,
    ) -> GcodePathDocument:
        return parse_gcode_file_paths(path, machine_settings=machine_settings)

    def brush_stroke_from_regions(
        self,
        operation: str,
        seed_indices: np.ndarray,
        regions: list[RegionEntry],
        *,
        amount_mm: float = 0.0,
        falloff_mm: float = 1.5,
        iterations: int = 1,
        strength: float = 0.5,
        editable_region_ids: list[str] | None = None,
        protected_region_ids: list[str] | None = None,
        respect_allowed_operations: bool = True,
    ) -> BrushStroke:
        return brush_stroke_from_regions(
            operation,
            seed_indices,
            regions,
            amount_mm=amount_mm,
            falloff_mm=falloff_mm,
            iterations=iterations,
            strength=strength,
            editable_region_ids=editable_region_ids,
            protected_region_ids=protected_region_ids,
            respect_allowed_operations=respect_allowed_operations,
        )


default_sdk = GeometrySDK()
