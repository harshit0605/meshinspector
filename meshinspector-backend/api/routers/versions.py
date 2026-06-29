"""Version and artifact routes."""

from __future__ import annotations

import base64
import binascii
import json
import math
import tempfile
from pathlib import Path

from fastapi import APIRouter, Depends, HTTPException, Query
from fastapi.concurrency import run_in_threadpool
from fastapi.responses import FileResponse
from sqlalchemy.orm import Session

from api.serializers import serialize_artifact, serialize_inspection_snapshot, serialize_job, serialize_snapshot, serialize_version
from core.db import get_db
from core.config import settings
from domain.models import ModelArtifactRecord, ModelVersionRecord
from domain.schemas import (
    BranchVersionRequest,
    CollisionDetectRequest,
    CollisionDetectResponse,
    CollisionFacePair,
    CompareCacheEntry,
    CompareResponse,
    DistanceMapContourBooleanRequest,
    DistanceMapContoursRequest,
    DistanceMapFromMeshRequest,
    DistanceMapIsoLinesRequest,
    DistanceMapMergeRequest,
    DistanceMapResponse,
    DistanceMapTiffExportRequest,
    DistanceMapTiffExportResponse,
    DistanceMapTiffImportRequest,
    ExactBooleanRequest,
    ExactBooleanResponse,
    GcodeLoadSourceRequest,
    GcodeParseFilePathsRequest,
    GcodeParsePathsRequest,
    GcodeParsePathsResponse,
    GcodeSourceResponse,
    GcodeWriteSourceRequest,
    InspectionSnapshotResponse,
    InspectionSnapshotState,
    IsoLineSegmentsResponse,
    JobResponse,
    ManufacturabilitySnapshot,
    MeasureInspectPairResult,
    MeasureInspectFeatureAngleResult,
    MeasureInspectFeatureDistanceResult,
    MeasureInspectFeatureIntersectionResult,
    MeasureInspectFeatureObjectPropertyResult,
    MeasureInspectFeatureObjectResult,
    MeasureInspectFeaturePairResult,
    MeasureInspectFeatureRefinementResult,
    MeasureInspectPointResult,
    MeasureInspectRequest,
    MeasureInspectResponse,
    MeasureInspectSurfaceDistanceResult,
    MeshToVoxelsSdfRequest,
    MeshToVoxelsSdfResponse,
    MeshCutMeasureTopologyRequest,
    MeshCutMeasureTopologyResponse,
    MeshLibOfficialParityFeature,
    MeshLibWorkbenchCommandCapability,
    MeshLibWorkbenchManifest,
    ModelVersionSummary,
    ObjectLinesBinaryExportRequest,
    ObjectLinesBinaryExportResponse,
    ObjectLinesBinaryLoadRequest,
    ObjectLinesFromContoursRequest,
    ObjectLinesPtsLoadRequest,
    ObjectLinesResponse,
    ObjectLinesSvgLoadRequest,
    ObjectLinesTextExportRequest,
    ObjectLinesTextExportResponse,
    ObjectLinesToContoursRequest,
    ObjectLinesToContoursResponse,
    OffsetMeshRequest,
    OffsetContoursRequest,
    OffsetContoursResponse,
    OffsetShellMeshResponse,
    OffsetSmoothingRequest,
    OffsetVertsRequest,
    PartialOffsetRequest,
    PointCloudIcpRequest,
    PointCloudIcpResponse,
    PointCloudMultiwayIcpRequest,
    PointCloudMultiwayIcpResponse,
    PointCloudTriangulationRequest,
    PointCloudTriangulationResponse,
    SectionContourPayload,
    SectionContourSegment,
    SelectionCommitRequest,
    SelectionCommitResponse,
    ShellMeshRequest,
    ThickenMeshRequest,
    TextureArtifactManifest,
    VersionDetailResponse,
    ViewerManifest,
    VoxelActiveBoxRequest,
    VoxelActiveBoxResponse,
    VoxelBinaryOperationsRequest,
    VoxelBinaryOperationsResponse,
    VoxelBooleanRequest,
    VoxelBooleanResponse,
    VoxelLineGraphRequest,
    VoxelLineGraphResponse,
    VoxelMaskToMeshRequest,
    VoxelMaskToMeshResponse,
    VoxelPathBuildFourEntry,
    VoxelPathBuildFourRequest,
    VoxelPathBuildFourResponse,
    VoxelPathPayload,
    VoxelPathRequest,
    VoxelPathResponse,
    VoxelRawLoadRequest,
    VoxelSegmentationRequest,
    VoxelSegmentationResponse,
    VoxelSliceRequest,
    VoxelSliceResponse,
    VoxelTiffLoadRequest,
    VoxelToMeshDualRequest,
    VoxelToMeshDualResponse,
    VoxelToMeshSimpleRequest,
    VoxelToMeshSimpleResponse,
    VoxelToMeshSmartRequest,
    VoxelToMeshSmartResponse,
    VoxelVolumeLoadResponse,
    VoxelVolumeRenderDataRequest,
    VoxelVolumeRenderDataResponse,
    VoxelVolumeRenderLutRequest,
    VoxelVolumeRenderLutResponse,
    VoxelVolumeRenderRayRequest,
    VoxelVolumeRenderRayResponse,
    WeightedShellRegionWeight,
    WeightedShellRequest,
)
from geometry_sdk import DistanceMapDocument, PointCloudDocument, RegionEntry, default_sdk
from geometry_sdk.types import SDFGrid
from geometry_sdk.core.mesh import (
    apply_meshlib_selection_modifier,
    expand_face_selection_to_components,
    graph_cut_select_region,
    graph_cut_select_region_auto_not_region,
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
)
from services.versioning import duplicate_version, register_file_artifact
from storage.object_store import object_store
from storage.repositories import create_snapshot_record, create_version, get_artifact_by_type, get_snapshot, get_version_artifacts, list_snapshots_by_prefix, upsert_snapshot
from services.manufacturability import compute_manufacturability_snapshot

router = APIRouter()

WORKBENCH_BUILT_IN_UI = [
    "ribbon",
    "scene_tree",
    "feature_search",
    "toolbar",
    "view_cube",
    "scale_bar",
    "notifications",
    "viewport_tags",
]

WORKBENCH_INTERACTIVE_TOOLS = [
    "select_mark_region",
    "thicken_brush",
    "scoop_brush",
    "smooth_brush",
    "measure_inspect",
]

WORKBENCH_COMMAND_CAPABILITIES = [
    {
        "command_id": "upload-new",
        "label": "Upload New",
        "group": "file",
        "notes": [
            "Frontend workspace navigation; model ingest is handled by /api/models.",
            "PLY uploads now route through default_sdk.load_mesh into Rust mesh_from_ply for MeshLib-style ASCII and binary little-/big-endian parsing with normals, edge payloads, UV/color including polygon face colors per source face row, tri-corner polygon texcoord list packing, TextureFile metadata with miniply comment trimming, first-existing TextureFile image loading, texture artifact handoff to viewer/workbench manifests including ordered multi-texture artifact manifests, normalized PLY UV/TextureFile export, preview GLB TEXCOORD_0 export for vertex and tri-corner UV sampling, viewer material texture application, MeshLib texturePerFace viewer material groups, native MeshLib texture-array shader sampling, and Rust-backed MeshLib ObjectMeshHolder/ObjectLinesHolder/ObjectPointsHolder-style scene JSON serialization plus serializeObjectTree-style .mru package export/import, ObjectMesh multi-object hierarchy import/export round-trip with object XF transforms, nested object-tree export preservation, Link shared-model reuse, ObjectLines scene object import/export with Polyline.Points and flat Polyline.Lines preservation, ObjectPoints scene object import/export with MeshLib PointsSave/PointsLoad-style point PLY, normals, vertex colors, PointSize, MaxRenderingPoints, and state preservation, Rust-backed scene-object transform, reparent, state, and reorder editing, and artifact registration; OBJ uploads now route through Rust mesh_from_obj for MeshLib MRMeshLoadObj-style vertex parsing, negative index resolution, object-name metadata, polygon fan triangulation, mtllib/usemtl material scopes, Kd diffuse color conversion, OBJ vt UV import into preview-ready tri-corner UVs, map_Kd texture-per-face metadata, map_Kd texture image loading with MeshLib Linear/Clamp texture settings, OBJ texture artifact provenance handoff, ordered multi-texture artifact manifests, viewer material texture application, MeshLib texturePerFace viewer material groups, native MeshLib texture-array shader sampling, and Rust-backed MeshLib ObjectMeshHolder/ObjectLinesHolder/ObjectPointsHolder-style scene JSON serialization plus serializeObjectTree-style .mru package export/import, ObjectMesh multi-object hierarchy import/export round-trip with object XF transforms, nested object-tree export preservation, Link shared-model reuse, ObjectLines scene object import/export with Polyline.Points and flat Polyline.Lines preservation, ObjectPoints scene object import/export with MeshLib PointsSave/PointsLoad-style point PLY, normals, vertex colors, PointSize, MaxRenderingPoints, and state preservation, Rust-backed scene-object transform, reparent, state, and reorder editing, and artifact registration.",
        ],
    },
    {
        "command_id": "download-stl",
        "label": "Download STL",
        "group": "file",
        "endpoint_url_key": "artifact_endpoint_url",
        "notes": ["Downloads the manufacturing STL artifact generated by the SDK pipeline."],
    },
    {
        "command_id": "export-section",
        "label": "Export Section SVG",
        "group": "file",
        "endpoint_url_key": "section_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["section_contour"],
    },
    {
        "command_id": "repair",
        "label": "Auto Repair",
        "group": "prepare",
        "endpoint_url_key": "repair_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["mesh_healer_diagnostics", "hole_fill_plan_diagnostics", "repeated_hole_boundary_vertices_diagnostics", "hole_complicating_faces_diagnostics", "remove_hole_complicating_faces", "short_edge_diagnostics", "degenerate_face_diagnostics", "multiple_edge_diagnostics", "repair_multiple_edges", "repair_nonmanifold_edges", "duplicate_nonmanifold_vertices", "duplicate_multi_hole_vertices", "not_smooth_face_diagnostics", "find_disoriented_faces", "flip_normals", "crease_edge_diagnostics", "crease_repair_plan_diagnostics", "fix_mesh_creases", "unite_close_vertices", "basic_repair", "service_fill_holes", "prune_small_components", "tunnel_diagnostics", "detect_tunnel_faces", "eliminate_tunnels", "fix_self_intersections_relax", "rebuild_via_sdf"],
    },
    {
        "command_id": "fit-size",
        "label": "Fit To Size",
        "group": "prepare",
        "endpoint_url_key": "resize_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["resize_ring"],
    },
    {
        "command_id": "reduce-weight",
        "label": "Reduce Weight",
        "group": "prepare",
        "endpoint_url_key": "hollow_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["adaptive_protected_hollow_to_weight"],
    },
    {
        "command_id": "prepare-casting",
        "label": "Prepare For Casting",
        "group": "prepare",
        "endpoint_url_key": "hollow_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["protected_hollow_mesh", "plan_drain_holes", "apply_drain_holes_voxel"],
    },
    {
        "command_id": "make-manufacturable",
        "label": "Make Manufacturable",
        "group": "prepare",
        "endpoint_url_key": "make_manufacturable_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["mesh_healer_diagnostics", "hole_fill_plan_diagnostics", "repeated_hole_boundary_vertices_diagnostics", "hole_complicating_faces_diagnostics", "remove_hole_complicating_faces", "short_edge_diagnostics", "degenerate_face_diagnostics", "multiple_edge_diagnostics", "repair_multiple_edges", "repair_nonmanifold_edges", "duplicate_nonmanifold_vertices", "duplicate_multi_hole_vertices", "not_smooth_face_diagnostics", "find_disoriented_faces", "flip_normals", "crease_edge_diagnostics", "crease_repair_plan_diagnostics", "fix_mesh_creases", "unite_close_vertices", "basic_repair", "prune_small_components", "tunnel_diagnostics", "detect_tunnel_faces", "eliminate_tunnels", "fix_self_intersections_relax", "rebuild_via_sdf", "resize_ring", "adaptive_protected_hollow_to_weight", "service_health"],
    },
    {
        "command_id": "resize",
        "label": "Resize",
        "group": "modify",
        "endpoint_url_key": "resize_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["resize_ring"],
    },
    {
        "command_id": "protected-hollow",
        "label": "Protected Hollow",
        "group": "modify",
        "endpoint_url_key": "hollow_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["protected_hollow_mesh"],
    },
    {
        "command_id": "offset-mesh",
        "label": "Offset Mesh",
        "group": "modify",
        "endpoint_url_key": "offset_mesh_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_offset_mesh"],
        "notes": [
            "Runs Rust voxel_offset_mesh with MeshLib generalOffsetMesh-style voxel offset semantics and creates a ready child version for review.",
        ],
    },
    {
        "command_id": "shell-mesh",
        "label": "Shell Mesh",
        "group": "modify",
        "endpoint_url_key": "shell_mesh_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_shell_mesh"],
        "notes": [
            "Runs Rust voxel_shell_mesh for the official MeshInspector Shell mode and creates a ready child version for review.",
        ],
    },
    {
        "command_id": "thicken-mesh",
        "label": "Thickening",
        "group": "modify",
        "endpoint_url_key": "thicken_mesh_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_thicken_mesh"],
        "notes": [
            "Runs Rust voxel_thicken_mesh with MeshLib thickenMesh signed-thickness semantics and creates a ready child version for review.",
        ],
    },
    {
        "command_id": "weighted-shell",
        "label": "Weighted Shell",
        "group": "modify",
        "endpoint_url_key": "weighted_shell_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_weighted_shell_mesh"],
        "notes": [
            "Runs Rust voxel_weighted_shell_mesh with MeshLib WeightedShell::meshShell-style region additive weights and creates a ready child version for review.",
        ],
    },
    {
        "command_id": "partial-offset",
        "label": "Partial Offset",
        "group": "modify",
        "endpoint_url_key": "partial_offset_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_partial_offset_mesh"],
        "notes": [
            "Runs Rust voxel_partial_offset_mesh with MeshLib partialOffsetMesh unsigned selected-part offset plus union semantics and creates a ready child version for review.",
        ],
    },
    {
        "command_id": "offset-verts",
        "label": "Offset Verts",
        "group": "modify",
        "endpoint_url_key": "offset_verts_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["offset_verts_mesh"],
        "notes": [
            "Runs Rust offset_verts_mesh with MeshLib MR::offsetVerts pseudonormal vertex shifting and creates a ready child version for review.",
        ],
    },
    {
        "command_id": "expand-shrink",
        "label": "Expand/Shrink",
        "group": "modify",
        "endpoint_url_key": "expand_shrink_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_offset_mesh"],
        "notes": [
            "Runs Rust voxel_offset_mesh outward and then inward, matching the official MeshInspector Expand/Shrink smoothing mode.",
        ],
    },
    {
        "command_id": "shrink-expand",
        "label": "Shrink/Expand",
        "group": "modify",
        "endpoint_url_key": "shrink_expand_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_offset_mesh"],
        "notes": [
            "Runs Rust voxel_offset_mesh inward and then outward, matching the official MeshInspector Shrink/Expand smoothing mode.",
        ],
    },
    {
        "command_id": "hollow-drains",
        "label": "Hollow + Drains",
        "group": "modify",
        "endpoint_url_key": "hollow_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["protected_hollow_mesh", "plan_drain_holes", "apply_drain_holes_voxel"],
    },
    {
        "command_id": "thicken-violations",
        "label": "Thicken Violations",
        "group": "modify",
        "endpoint_url_key": "thicken_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["local_thicken_to_minimum"],
    },
    {
        "command_id": "thicken-region",
        "label": "Thicken Region",
        "group": "modify",
        "endpoint_url_key": "thicken_endpoint_url",
        "runtime_tool_id": "thicken_brush",
        "rust_backed": True,
        "sdk_operations": ["local_thicken_to_minimum"],
    },
    {
        "command_id": "batch-thicken",
        "label": "Batch Thicken",
        "group": "modify",
        "endpoint_url_key": "thicken_endpoint_url",
        "runtime_tool_id": "thicken_brush",
        "rust_backed": True,
        "sdk_operations": ["local_thicken_to_minimum"],
    },
    {
        "command_id": "scoop",
        "label": "Scoop",
        "group": "modify",
        "endpoint_url_key": "scoop_endpoint_url",
        "runtime_tool_id": "scoop_brush",
        "rust_backed": True,
        "sdk_operations": ["local_scoop"],
    },
    {
        "command_id": "smooth",
        "label": "Smooth",
        "group": "modify",
        "endpoint_url_key": "smooth_endpoint_url",
        "runtime_tool_id": "smooth_brush",
        "rust_backed": True,
        "sdk_operations": ["smooth"],
    },
    {
        "command_id": "batch-smooth",
        "label": "Batch Smooth",
        "group": "modify",
        "endpoint_url_key": "smooth_endpoint_url",
        "runtime_tool_id": "smooth_brush",
        "rust_backed": True,
        "sdk_operations": ["smooth"],
    },
    {
        "command_id": "decimate-mesh",
        "label": "Decimate Mesh",
        "group": "modify",
        "endpoint_url_key": "decimate_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["decimate_mesh"],
        "notes": [
            "Rust-backed MeshLib MR::decimateMesh DecimateStrategy::MinimizeError QEM with target triangle count/percentage stop controls through maxDeletedFaces, stabilizer and angleWeightedDistToPlane face-plane weighting plus ShortestEdgeFirst subset with maxError, FaceBitSet region masks, maxEdgeLen, maxBdShift, maxTriangleAspectRatio, criticalTriAspectRatio aspect-relaxation guard, tinyEdgeLength endpoint aspect-bypass guard, maxAngleChange local Delone flip guard, touchNearBdEdges, touchBdVerts, notFlippable adjacent-collapse guards with crease-form QEM weighting, deletion limits including MeshLib's unbounded-default half-face guard, optimized collapse positions, notFlippable dynamic remapping with remapped_not_flippable_edges metadata, edgesToCollapse collapse subset and remapping metadata, twinMap symmetric validation plus paired same-position collapse, paired maxAngleChange Delone flips, and collapse/flip/pack remapping metadata, MeshLib preCollapseVertAttribute-style vertex_uvs and vertex_colors interpolation, packMesh output, subdivideParts part partitioning, and decimateBetweenParts final pass; arbitrary preCollapse callbacks and true threaded execution remain future parity gates."
        ],
    },
    {
        "command_id": "subdivide-mesh",
        "label": "Subdivide Mesh",
        "group": "modify",
        "endpoint_url_key": "subdivide_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["subdivide_mesh"],
        "notes": [
            "Rust-backed MeshLib-style SubdivideSettings maxEdgeLen, curvaturePriority, maxEdgeSplits, FaceBitSet region masks, notFlippable protected Delone-ring edge guards with split-edge remapping, maxDeviationAfterFlip, maxAngleChangeAfterFlip, criticalAspectRatioFlip, maxTriAspectRatio, maxSplittableTriAspectRatio, projectOnOriginalMesh, smoothMode cotan positioning with minSharpDihedralAngle sharp-vertex fixing, MeshBuilder edge-rank ordering, split face-ID/orientation preservation, and chained local Delone topology are implemented; broader smoothMode crease-topology oracles remain future gates."
        ],
    },
    {
        "command_id": "make-delone",
        "label": "Make Delone",
        "group": "modify",
        "endpoint_url_key": "make_delone_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["make_delone_edge_flips"],
        "notes": [
            "Rust-backed MeshLib MR::makeDeloneEdgeFlips local edge-flip pass with region face masks, iteration count, maxDeviationAfterFlip diagonal-deviation guard, maxAngleChange dihedral-delta guard, criticalTriAspectRatio angle-guard override, notFlippable edge constraints, and vertRegion vertex constraints is wired as a standalone official Mesh Repair / Mesh Edit command."
        ],
    },
    {
        "command_id": "section",
        "label": "Section",
        "group": "inspect",
        "endpoint_url_key": "section_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["section_contour"],
    },
    {
        "command_id": "heatmap",
        "label": "Heatmap",
        "group": "inspect",
        "endpoint_url_key": "thickness_overlay_url",
        "rust_backed": True,
        "sdk_operations": ["thickness_overlay_payload"],
    },
    {
        "command_id": "regions",
        "label": "Regions",
        "group": "inspect",
        "endpoint_url_key": "selection_endpoint_url",
        "runtime_tool_id": "select_mark_region",
        "rust_backed": True,
        # Inspect regions core SDK marker: "sdk_operations": ["detect_ring_regions", "closest_points_on_mesh"]
        "sdk_operations": [
            "apply_meshlib_selection_modifier",
            "detect_ring_regions",
            "closest_points_on_mesh",
            "expand_face_selection_to_components",
            "graph_cut_select_region",
            "graph_cut_select_region_auto_not_region",
            "meshlib_select_scene_objects",
            "select_boundary_faces",
            "select_boundary_edges",
            "select_camera_facing_faces",
            "select_crease_edges",
            "select_degenerate_faces",
            "select_face_by_ray",
            "select_faces_by_area",
            "select_faces_by_screen_brush",
            "select_faces_by_screen_polygon",
            "select_faces_by_screen_rect",
            "select_inside_part_faces",
            "select_largest_component_faces",
            "select_not_smooth_faces",
            "select_outer_layer_faces",
            "select_overlapping_faces",
            "select_overhang_faces",
            "select_short_edges",
            "self_intersecting_faces",
        ],
    },
    {
        "command_id": "measure-inspect",
        "label": "Measure / Inspect",
        "group": "inspect",
        "endpoint_url_key": "measurement_endpoint_url",
        "runtime_tool_id": "measure_inspect",
        "rust_backed": True,
        "sdk_operations": ["closest_points_on_mesh", "feature_pair_measurements", "mesh_geodesic_path", "mesh_geodesic_polyline_path", "mesh_cut_measure_contours", "mesh_geodesic_quadrangle_path", "mesh_fast_marching_surface_path", "mesh_fast_marching_surface_path_tri_points", "mesh_surface_path_tri_points", "object_lines_from_contours", "mesh_geodesic_distance_field", "mesh_closest_surface_path_targets", "mesh_surface_distance_seed_vertices", "mesh_geodesic_iso_region", "mesh_geodesic_extreme_edges", "thickness_overlay_payload"],
    },
    {
        "command_id": "mesh-cut-measure-path",
        "label": "Mesh Cut & Measure Path",
        "group": "inspect",
        "endpoint_url_key": "mesh_cut_measure_topology_endpoint_url",
        "runtime_tool_id": "mesh_cut_measure_path",
        "rust_backed": True,
        "sdk_operations": ["mesh_geodesic_path", "mesh_geodesic_polyline_path", "mesh_cut_measure_contours", "mesh_cut_measure_edge_path_topology_cut", "mesh_geodesic_quadrangle_path", "object_lines_from_contours"],
        "notes": [
            "Official Mesh Cut & Measure geodesic path/export surface backed by Rust MeshLib-style measurement and topology endpoints. The topology endpoint creates a child version for the edge-aligned MR::convertSurfacePathsToMeshContours / MR::cutMesh seam subset, while Measure / Inspect continues to return MR::buildShortestPath metrics, OneMeshContour cut-input payloads, and ObjectLines/ObjectPoints export payloads."
        ],
    },
    {
        "command_id": "wireframe",
        "label": "Wireframe",
        "group": "inspect",
        "notes": ["Frontend-only topology overlay over the loaded MeshLib viewport mesh."],
    },
    {
        "command_id": "snapshots",
        "label": "Inspection Snapshots",
        "group": "inspect",
        "endpoint_url_key": "inspection_snapshots_endpoint_url",
        "notes": ["Persists UI inspection state; no geometry mutation is performed."],
    },
    {
        "command_id": "compare-versions",
        "label": "Compare Versions",
        "group": "review",
        "endpoint_url_key": "compare_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["service_compare_field", "service_compare"],
    },
    {
        "command_id": "point-cloud-icp",
        "label": "Point Cloud / ICP",
        "group": "inspect",
        "endpoint_url_key": "point_cloud_icp_endpoint_url",
        "rust_backed": True,
        "sdk_operations": [
            "point_cloud_nearest_projections",
            "point_cloud_project_to_mesh",
            "point_cloud_n_closest_neighbors",
            "point_cloud_two_closest_points",
            "point_cloud_neighbors_in_radius",
            "point_cloud_select_by_screen_polygon",
            "point_cloud_select_by_screen_rect",
            "point_cloud_select_by_screen_brush",
            "point_cloud_pick_by_ray",
            "point_cloud_extract_selected_points_as_object",
            "point_cloud_local_neighbor_fan",
            "point_cloud_local_fan_triangles",
            "point_cloud_local_triangulation_repetitions",
            "point_cloud_triangulate_candidate_mesh",
            "point_cloud_triangulate_cleaned_candidate_mesh",
            "point_cloud_triangulate_topology_candidate_mesh",
            "point_cloud_triangulate_filled_candidate_mesh",
            "point_cloud_uniform_sample",
            "point_cloud_grid_sample",
            "pairwise_point_to_point_icp",
            "pairwise_point_to_plane_icp",
            "multiway_point_to_point_icp",
            "multiway_point_to_plane_icp",
            "multiway_combined_icp",
            "multiway_all_object_point_to_point_icp",
            "multiway_all_object_point_to_plane_icp",
            "multiway_all_object_combined_icp",
            "multiway_sequential_cascade_point_to_point_icp",
            "multiway_sequential_cascade_point_to_plane_icp",
            "multiway_sequential_cascade_combined_icp",
            "multiway_aabb_cascade_point_to_point_icp",
            "multiway_aabb_cascade_point_to_plane_icp",
            "multiway_aabb_cascade_combined_icp",
        ],
        "notes": [
            "Rust point-cloud nearest projections, single-mesh projection payloads with rigid object/reference transforms, MeshLib-style face-region masks, MeshLib-style face/edge/vertex pseudonormal normals, MRSelectScreenLasso::findVertsInViewportArea-style point primitive screen selection through point_cloud_select_by_screen_polygon / point_cloud_select_by_screen_rect / point_cloud_select_by_screen_brush, MeshLib pickRenderObject/ObjectPointsHolder-style primitive Pick selection through point_cloud_pick_by_ray, MeshLib ObjectPoints::cloneRegion/PointCloud::addPartByMask-style selected-point extraction through point_cloud_extract_selected_points_as_object, point-cloud Selection to Object child-version creation with normalized_point_cloud_ply artifacts, closest-neighbor primitives, radius-neighbor filtering, local fan boundary detection, fan-triangle emission, local-triangulation repetition accounting, repeated-triangle candidate mesh assembly, MeshLib-style two-phase topology edge filtering with MeshBuilder-style half-edge origin-ring insertion guards, hole-complicating bad-triangle removal, MeshLib-thresholded small-hole fill composition with MultipleEdgesResolveMode None/Simple/Strong dispatch, Simple-mode duplicate-edge avoidance, Strong-mode reused generated chord repair, outNewFaces new-face index reporting, maxPolygonSubdivisions split sampling, makeDegenerateBand duplicate-boundary band creation, stopBeforeBadTriangulation bad-patch guarding, smoothBd boundary-edge metric control, getMinAreaMetric double-area triangulation, getEdgeLengthFillMetric edge-length triangulation, getUniversalMetric universal smooth triangulation, getMaxDihedralAngleMetric max-dihedral-angle triangulation, getParallelPlaneFillMetric parallel-plane projection triangulation, getComplexFillMetric aspect-area edge-penalty triangulation, getMinTriAngleMetric minimum-angle triangulation, getPlaneFillMetric plane-normal triangulation, getPlaneNormalizedFillMetric plane-normalized aspect triangulation, getComplexStitchMetric aspect-ratio/dihedral stitch triangulation, getEdgeLengthStitchMetric edge-length stitch triangulation, getVerticalStitchMetric caller-supplied upDir vertical stitch triangulation, and getVerticalStitchMetricEdgeBased caller-supplied upDir vertical edge-projection stitch triangulation, max-removes optimization, uniform/grid sampling, pairwise point-to-point/point-to-plane ICP kernels with MeshLib-style distance, normal-cosine, and reciprocal closest pair filtering, MeshLib maxGroupSize=1-style independent multiway point-to-point/point-to-plane/combined ICP, MeshLib maxGroupSize=0-style all-object multiway point-to-point/point-to-plane/combined ICP, MeshLib maxGroupSize>1 sequential cascade multiway point-to-point/point-to-plane/combined ICP, and MeshLib AABBTreeBased cascade multiway point-to-point/point-to-plane/combined ICP are Rust-backed; full MeshLib mesh-topology materialization, arbitrary callback FillHoleMetric parameterization, and non-rigid tree-accelerated/multi-object mesh projection workflows remain future parity items."
        ],
    },
    {
        "command_id": "point-cloud-triangulate",
        "label": "Point Cloud Triangulate",
        "group": "inspect",
        "endpoint_url_key": "point_cloud_triangulation_endpoint_url",
        "rust_backed": True,
        "sdk_operations": [
            "point_cloud_triangulate_candidate_mesh",
            "point_cloud_triangulate_cleaned_candidate_mesh",
            "point_cloud_triangulate_topology_candidate_mesh",
            "point_cloud_triangulate_filled_candidate_mesh",
        ],
    },
    {
        "command_id": "point-cloud-multiway-icp",
        "label": "Point Cloud Multiway ICP",
        "group": "inspect",
        "endpoint_url_key": "point_cloud_multiway_icp_endpoint_url",
        "rust_backed": True,
        "sdk_operations": [
            "multiway_point_to_point_icp",
            "multiway_point_to_plane_icp",
            "multiway_combined_icp",
            "multiway_all_object_point_to_point_icp",
            "multiway_all_object_point_to_plane_icp",
            "multiway_all_object_combined_icp",
            "multiway_sequential_cascade_point_to_point_icp",
            "multiway_sequential_cascade_point_to_plane_icp",
            "multiway_sequential_cascade_combined_icp",
            "multiway_aabb_cascade_point_to_point_icp",
            "multiway_aabb_cascade_point_to_plane_icp",
            "multiway_aabb_cascade_combined_icp",
        ],
    },
    {
        "command_id": "distance-map-contours",
        "label": "Contour Distance Map",
        "group": "inspect",
        "endpoint_url_key": "distance_map_contours_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_from_contours"],
        "notes": ["Rust contour-to-distance-map kernel follows MeshLib pixel-center sampling; ObjectLines persistence is exposed through separate Rust-backed commands."],
    },
    {
        "command_id": "object-lines-from-contours",
        "label": "ObjectLines From Contours",
        "group": "inspect",
        "endpoint_url_key": "object_lines_from_contours_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_from_contours"],
        "notes": ["Rust ObjectLines builder follows MeshLib ObjectLinesHolder scene JSON shape with Polyline.Points and flat Polyline.Lines vertex-index pairs."],
    },
    {
        "command_id": "object-lines-to-contours",
        "label": "ObjectLines To Contours",
        "group": "inspect",
        "endpoint_url_key": "object_lines_to_contours_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_to_contours"],
        "notes": ["Rust ObjectLines contour restoration follows MeshLib PolylineTopology contour traversal for open and closed polyline components."],
    },
    {
        "command_id": "offset-contours",
        "label": "Offset Contours",
        "group": "inspect",
        "endpoint_url_key": "offset_contours_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["offset_contours", "offset_contours_with_origins"],
        "notes": ["Rust offset_contours follows MeshLib MROffsetContours closed clockwise signed Type::Offset round-corner fixed-offset, positive CornerType::Sharp fixed-offset with maxSharpAngle limiting, default 3D signed fixed/variable Type::Offset, sharp max-angle, fixed/variable shell Z restore/one-pass default relaxation, explicit relaxIterations, constant/custom source-Z restore plus callable zCallback output/index/origin context, positive closed fixed/variable non-intersection, closed fixed zero-offset identity indicesMap/origin output, plus negative and shell-inner closed fixed/variable intersection indicesMap/origin output, signed variable-offset Type::Offset round/sharp-corner with maxSharpAngle limiting, positive fixed/variable including unequal-variable and mixed-signed Type::Offset final-outline self-overlap remap with indicesMap intersections, signed variable-offset Type::Shell round/sharp-corner with maxSharpAngle limiting including empty negative-shell output, signed fixed-offset Type::Shell, open fixed bent/zig and variable bent/zig round-end indicesMap/origin output, open fixed cut-end connected collinear seam-preserving axis/non-axis plus axis/non-axis shifted parallel global-outline composition, axis-aligned perpendicular crossing, horizontal/vertical/non-axis touching-chain including horizontal direction variants, direction-reversed vertical and diagonal origin maps, and first-direction-reversed vertical/diagonal outline ordering, axis/non-axis overlapping-parallel, and axis/non-axis collinear-overlap plus direction-reversed horizontal collinear-overlap including first-source and both-reversed ordering, vertical direction variants, diagonal direction variants, and three-segment horizontal/vertical/diagonal collinear-overlap chains including diagonal chain direction variants global-outline indicesMap/origin output, and open EndType::Round/Cut behavior for validated slices."],
    },
    {
        "command_id": "object-lines-load-mrlines",
        "label": "ObjectLines Load MrLines",
        "group": "inspect",
        "endpoint_url_key": "object_lines_load_mrlines_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_from_mrlines"],
        "notes": ["Rust MrLines loader follows MeshLib LinesLoad::fromMrLines binary PolylineTopology and Vector3f point payload parsing."],
    },
    {
        "command_id": "object-lines-save-mrlines",
        "label": "ObjectLines Save MrLines",
        "group": "inspect",
        "endpoint_url_key": "object_lines_save_mrlines_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_to_mrlines"],
        "notes": ["Rust MrLines exporter follows MeshLib LinesSave::toMrLines binary PolylineTopology and point type 3 Vector3f serialization."],
    },
    {
        "command_id": "object-lines-load-ply",
        "label": "ObjectLines Load PLY",
        "group": "inspect",
        "endpoint_url_key": "object_lines_load_ply_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_from_ply"],
        "notes": ["Rust PLY line loader follows MeshLib LinesLoad::fromPly vertex/edge payload parsing with MeshLib-style magic-line whitespace, format-version whitespace, minor punctuation-suffix tolerance and alpha-suffix rejection, format-line, element-line, and property-line trailing-token tolerance plus element-count alpha or underscore suffix rejection and property-name prefix suffix tolerance, end_header trailing-whitespace handling, strict header directive, leading keyword whitespace, and identifier validation, strict scalar type alias validation, Vector3f coordinate narrowing and source scalar conversion, MeshLib-style scalar-to-int edge endpoint conversion plus ASCII row integer-prefix suffix, narrow integer wrapping, and unsigned scalar sign-cast handling, PolylineTopology-style invalid edge skipping plus MeshLib-style edge elements without vertex1/vertex2 skipping, plus optional RGB vertex colors with MeshLib-style r/g/b short-name precedence over red/green/blue and scalar-to-uchar color conversion including integer wrapping, unneeded list-property skipping for ASCII files, MeshLib-style binary list-count scalar conversion for binary files, MeshLib-generated binary little-endian files, and binary big-endian files."],
    },
    {
        "command_id": "object-lines-save-ply",
        "label": "ObjectLines Save PLY",
        "group": "inspect",
        "endpoint_url_key": "object_lines_save_ply_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_to_ply"],
        "notes": ["Rust PLY line exporter follows MeshLib LinesSave::toPly binary little-endian vertex, optional RGB vertex color, and edge payload serialization."],
    },
    {
        "command_id": "object-lines-load-pts",
        "label": "ObjectLines Load PTS",
        "group": "inspect",
        "endpoint_url_key": "object_lines_load_pts_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_from_pts"],
        "notes": ["Rust PTS line loader follows MeshLib LinesLoad::fromPts BEGIN_Polyline/END_Polyline block parsing, Vector3f coordinate narrowing, trailing point-field tolerance, and last-coordinate numeric-prefix suffix tolerance."],
    },
    {
        "command_id": "object-lines-load-svg",
        "label": "ObjectLines Load SVG",
        "group": "inspect",
        "endpoint_url_key": "object_lines_load_svg_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_from_svg"],
        "notes": ["Rust SVG line loader follows MeshLib MRIOExtras LinesLoad::fromSvg for <line>, <polyline>, compact signed <polyline>/<polygon> points, <polygon>, <circle>, <ellipse>, simple/rounded <rect>, <path> commands (M/m, L/l, H/h, V/v, C/c, S/s, Q/q, T/t, A/a, Z/z), and transform attributes with MeshLib's post-parse Y-axis flip into Vector3f ObjectLines coordinates."],
    },
    {
        "command_id": "object-lines-save-pts",
        "label": "ObjectLines Save PTS",
        "group": "inspect",
        "endpoint_url_key": "object_lines_save_pts_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_to_pts"],
        "notes": ["Rust PTS line exporter follows MeshLib LinesSave::toPts BEGIN_Polyline/END_Polyline block emission."],
    },
    {
        "command_id": "object-lines-save-dxf",
        "label": "ObjectLines Save DXF",
        "group": "inspect",
        "endpoint_url_key": "object_lines_save_dxf_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["object_lines_to_dxf"],
        "notes": ["Rust DXF line exporter follows MeshLib LinesSave::toDxf POLYLINE/VERTEX/SEQEND entity emission."],
    },
    {
        "command_id": "distance-map-from-mesh",
        "label": "Mesh Distance Map",
        "group": "inspect",
        "endpoint_url_key": "distance_map_from_mesh_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_from_mesh"],
        "notes": ["Rust orthographic ray distance-map kernel follows MeshLib computeDistanceMap pixel-center ray sampling for explicit mesh projection frames."],
    },
    {
        "command_id": "distance-map-iso-lines",
        "label": "Distance Map Iso-Lines",
        "group": "inspect",
        "endpoint_url_key": "distance_map_iso_lines_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_to_iso_segments"],
        "notes": ["Rust marching-squares iso-line extraction follows MeshLib distanceMapTo2DIsoPolyline pixel-center interpolation."],
    },
    {
        "command_id": "distance-map-merge",
        "label": "Distance Map Merge",
        "group": "inspect",
        "endpoint_url_key": "distance_map_merge_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_merge"],
        "notes": ["Rust cell-wise min, max, and subtraction follow MeshLib DistanceMap merge and subtraction invalid-cell behavior."],
    },
    {
        "command_id": "distance-map-contour-boolean",
        "label": "Contour Boolean From Distance Maps",
        "group": "inspect",
        "endpoint_url_key": "distance_map_contour_boolean_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_contour_boolean"],
        "notes": ["Rust contour union, intersection, and subtraction follow MeshLib contourUnion/contourIntersection/contourSubtract signed-distance composition."],
    },
    {
        "command_id": "distance-map-from-tiff",
        "label": "TIFF Distance Map Import",
        "group": "inspect",
        "endpoint_url_key": "distance_map_from_tiff_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_from_tiff"],
        "notes": ["Rust TIFF distance-map import follows MeshLib DistanceMapLoad::fromTiff scalar/RGB/RGBA-to-float conversion, MeshLib raw-value handling for WhiteIsZero images, and full MeshLib ModelTransformationTag metadata with 2D origin/pixel-size projection."],
    },
    {
        "command_id": "distance-map-to-tiff",
        "label": "TIFF Distance Map Export",
        "group": "inspect",
        "endpoint_url_key": "distance_map_to_tiff_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["distance_map_to_tiff"],
        "notes": ["Rust TIFF distance-map export follows MeshLib DistanceMapSave::toTiff float scalar image output, WhiteIsZero photometric tag, GDAL NoData sentinel tagging, and full MeshLib ModelTransformationTag preservation when model_transform is present."],
    },
    {
        "command_id": "gcode-parse-paths",
        "label": "G-code Path Parser",
        "group": "inspect",
        "endpoint_url_key": "gcode_parse_paths_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["parse_gcode_paths"],
        "notes": ["Rust G-code path parser follows MeshLib GcodeLoad/GcodeProcessor frame, comment, strtof command-value narrowing including leading command-value whitespace, special, and hexadecimal float tokens, no-motion feedrateMax updates, zero-idle feedrate post-pass rewriting, radius-only G2/G3 no-op handling, G28 home zero-length idle actions, MeshLib-style arc radius-mismatch warning formatting, modal G0/G1, G17/G18/G19 G2/G3 arc, G50/G51 scaling, default/custom CNC home/feedrate/axis/order/limit settings, CNCMachineSettings MRSerializer object plus decimal/hex-float/numeric-prefix whitespace-string vector settings JSON with MeshLib stream-default partial assignment, rotary-axis sampling, tool-direction, unit, and feedrate semantics."],
    },
    {
        "command_id": "mesh-to-voxels-sdf",
        "label": "Mesh to Voxels / SDF",
        "group": "inspect",
        "endpoint_url_key": "voxelize_mesh_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["sample_sdf_grid", "sdf_occupancy", "estimate_sdf_volume", "extract_sdf_isosurface"],
        "notes": [
            "Rust SDF conversion follows MeshLib meshToLevelSet-style voxel-size and positive surface-offset contract for signed closed meshes.",
            "Unsigned mode follows MeshLib meshToDistanceField-style distance-field semantics and reports unsigned distances.",
        ],
    },
    {
        "command_id": "voxel-binary-operations",
        "label": "Binary Operations",
        "group": "inspect",
        "endpoint_url_key": "voxel_binary_operations_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_binary_values", "voxel_binary_iso_value"],
        "notes": [
            "Rust voxel_binary_values and voxel_binary_iso_value expose MeshLib BinaryOperations scalar-grid Max/Min/Sum/Multiply/Divide behavior plus level-set Union/Intersection/Difference iso-value semantics.",
        ],
    },
    {
        "command_id": "open-raw-voxels",
        "label": "Open RAW Voxels",
        "group": "inspect",
        "endpoint_url_key": "open_raw_voxels_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["load_raw_voxels", "load_raw_voxels_auto", "voxel_default_iso_value"],
        "notes": [
            "Rust load_raw_voxels and load_raw_voxels_auto expose MeshLib VoxelsLoad::fromRaw explicit and filename-auto parameter behavior for dimensions, voxel size, gridLevelSet, ScalarType conversion, and ObjectVoxels histogram one-third-bin default iso-value selection.",
        ],
    },
    {
        "command_id": "open-voxels-from-tiff",
        "label": "Open Voxels From TIFF",
        "group": "inspect",
        "endpoint_url_key": "open_voxels_from_tiff_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["load_tiff_voxels_dir", "voxel_default_iso_value"],
        "notes": [
            "Rust load_tiff_voxels_dir exposes MeshLib VoxelsLoad::loadTiffDir directory filtering, numeric scan-name sorting, per-slice TIFF parameter consistency, scalar/RGB/RGBA float conversion, voxel size, DenseGrid/LevelSet selection, and ObjectVoxels histogram one-third-bin default iso-value selection.",
        ],
    },
    {
        "command_id": "voxel-slice",
        "label": "Voxels Slice",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_slice_endpoint_url",
        "sdk_operations": ["voxel_slice"],
        "notes": [
            "Rust voxel_slice exposes MeshLib MRVoxelsSave::saveSliceToImage and MRMarkedVoxelSlice-style YZ/ZX/XY texture ordering with min/max value normalization.",
        ],
    },
    {
        "command_id": "voxel-line-graph",
        "label": "Voxels Line Graph",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_line_graph_endpoint_url",
        "sdk_operations": ["voxel_line_graph"],
        "notes": [
            "Rust voxel_line_graph exposes the official MeshInspector Voxels Line Graph CT tool as MeshLib x-fastest axis-probe sampling over ObjectVoxels dense values.",
        ],
    },
    {
        "command_id": "voxel-active-box",
        "label": "Set Active Voxel Box",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_active_box_endpoint_url",
        "sdk_operations": ["voxel_active_box"],
        "notes": [
            "Rust voxel_active_box exposes MeshLib ObjectVoxels::setActiveBounds max-excluded active-box semantics and the official Create New Object crop payload.",
        ],
    },
    {
        "command_id": "voxel-volume-render-data",
        "label": "Voxels Volume Rendering Data",
        "group": "inspect",
        "endpoint_url_key": "voxel_volume_render_data_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_volume_render_data"],
        "notes": [
            "Rust voxel_volume_render_data exposes MeshLib ObjectVoxels::prepareDataForVolumeRendering / vdbVolumeToSimpleVolumeNorm prepared-data semantics: active-box max-excluded dimensions, MeshLib x-fastest voxel indexing, voxelSize propagation, and source-scale linear normalization clamped to [0, 1]. GL shader compositing remains a viewer parity item.",
        ],
    },
    {
        "command_id": "voxel-volume-render-lut",
        "label": "Voxels Volume Rendering LUT",
        "group": "inspect",
        "endpoint_url_key": "voxel_volume_render_lut_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_volume_render_lut"],
        "notes": [
            "Rust voxel_volume_render_lut exposes MeshLib RenderVolumeObject::bindVolume_ denseMap behavior for ObjectVoxels::VolumeRenderingParams::LutType and VolumeRenderingParams::AlphaType: GrayShades, Rainbow, OneColor, Constant, LinearIncreasing, LinearDecreasing, and alphaLimit byte transfer-function values. GL ray-march shader compositing remains a viewer parity item.",
        ],
    },
    {
        "command_id": "voxel-volume-render-ray",
        "label": "Voxels Volume Rendering Ray",
        "group": "inspect",
        "endpoint_url_key": "voxel_volume_render_ray_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_volume_render_ray"],
        "notes": [
            "Rust voxel_volume_render_ray exposes MRVolumeShader ray compositing for samplingStep > 0 fixed-step traversal and step <= 0 rayVoxelIntersection voxel-boundary traversal: clipping-plane discard, active-voxel masking, min/max density gating, denseMap transfer-function lookup, front-to-back alpha compositing, shadingMode == 1 value-gradient zero-normal sample rejection, shadingMode == 2 alpha-gradient normal sampling, and optional MeshLib shadeColor lighting modulation.",
        ],
    },
    {
        "command_id": "voxel-segmentation",
        "label": "Voxels Segmentation",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_segmentation_endpoint_url",
        "sdk_operations": ["voxel_segmentation", "voxel_segmentation_mesh"],
        "notes": [
            "Rust voxel_segmentation and voxel_segmentation_mesh expose MeshLib MRVoxelGraphCut and MRVolumeSegment seed-based CT segmentation plus createMeshFromSegmentation finalization through the /voxels/segmentation endpoint: inside/outside seeds, VolumeSegmenter crop expansion, boundary outside seeds, directed density edge capacities, simple 1/0 mask meshing at iso-value 0.5, and minVoxel*voxelSize mesh shift.",
        ],
    },
    {
        "command_id": "voxel-mask-to-mesh",
        "label": "Voxels Mask to Mesh",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_mask_to_mesh_endpoint_url",
        "sdk_operations": ["voxel_mask_to_mesh"],
        "notes": [
            "Rust voxel_mask_to_mesh exposes MeshLib MRVolumeSegment meshFromVoxelsMask smooth mask conversion through the /voxels/mask-to-mesh endpoint: whole-volume mask crop expansion, prepareVolumePart VolumeMaskMeshingMode::Smooth inside/outside density averaging, expand/shrink smoothing bands, gridToMesh iso-value 0.5, and minVoxel*voxelSize mesh shift.",
        ],
    },
    {
        "command_id": "voxel-to-mesh-simple",
        "label": "Voxels to Mesh Simple",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_to_mesh_simple_endpoint_url",
        "sdk_operations": ["voxel_to_mesh_simple"],
        "notes": [
            "Rust voxel_to_mesh_simple exposes the official MeshInspector Voxels to Mesh Simple Conversion contract around MeshLib ObjectVoxels::recalculateIsoSurface: iso-value extraction, anisotropic voxelSize scaling, lessInside=false high-density interior convention for dense volumes, lessInside=true signed-distance convention for LevelSet grids, and MeshLib x-fastest voxel indexing. Dense dual-contouring extraction is exposed separately through voxel_to_mesh_dual / Dual Marching Cubes; exact sparse OpenVDB VolumeToMesh topology and Smart Conversion gradient refinement remain open parity items.",
        ],
    },
    {
        "command_id": "voxel-to-mesh-dual",
        "label": "Voxels to Mesh Dual",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_to_mesh_dual_endpoint_url",
        "sdk_operations": ["voxel_to_mesh_dual"],
        "notes": [
            "Rust voxel_to_mesh_dual exposes a dense dual-contouring slice of the official MeshInspector ObjectVoxels::recalculateIsoSurface / openvdb::tools::VolumeToMesh path: iso-value extraction, MeshLib x-fastest dense values, anisotropic voxelSize scaling, lessInside=false high-density interiors, lessInside=true LevelSet grids, MeshLib maxVertices/maxFaces limit errors, dense planar adaptivity coalescing, direct .vdb FloatGrid dense-payload decoding with OpenVDB active bbox origin preservation, distinct OpenVDB topology and value-buffer masks, tight sparse active-bbox, active-window boundary, and full-leaf-span sparse active-mask background halo padding, and MeshLib relaxDisorientedTriangles-style closed-surface ray-count face relaxation. Exact sparse OpenVDB VolumeToMesh topology and curved/sparse adaptivity remain open parity items.",
        ],
    },
    {
        "command_id": "voxel-to-mesh-smart",
        "label": "Voxels to Mesh Smart",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_to_mesh_smart_endpoint_url",
        "sdk_operations": ["voxel_to_mesh_smart", "voxel_move_mesh_to_max_deriv"],
        "notes": [
            "Rust voxel_to_mesh_smart and voxel_move_mesh_to_max_deriv expose official MeshInspector Smart Conversion refinement via MeshLib MR::moveMeshToVoxelMaxDeriv through the /voxels/to-mesh/smart endpoint: samplePoints=6 default, degree=3 default with MeshLib degree=3..6 support, polynomial density fitting along vertex normals, derivative-minimum shift, outlier threshold, clamped 0.1 voxel shift, intermediate shift smoothing, and final relax. Exact sparse OpenVDB VolumeToMesh topology and full scene workflow remain open.",
        ],
    },
    {
        "command_id": "voxel-path",
        "label": "Voxels Path",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_path_endpoint_url",
        "sdk_operations": ["voxel_path"],
        "notes": [
            "Rust voxel_path exposes MeshLib MRVoxelPath buildSmallestMetricPath behavior for CT voxel-path inspection with voxelsSumDiffsMetric Difference and voxelsExponentMetric Exponent modes.",
        ],
    },
    {
        "command_id": "voxel-path-build-four",
        "label": "Voxels Path Build Four",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "voxel_path_build_four_endpoint_url",
        "sdk_operations": ["voxel_path_build_four"],
        "notes": [
            "Rust voxel_path_build_four exposes the official MeshInspector Build four mode by running MeshLib QuarterBit masks 1, 2, 4, and 8 through MRVoxelPath buildSmallestMetricPath-style path construction.",
        ],
    },
    {
        "command_id": "voxel-boolean",
        "label": "Voxel Boolean",
        "group": "inspect",
        "endpoint_url_key": "voxel_boolean_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["voxel_boolean_mesh"],
        "notes": [
            "Runs Rust voxel_boolean_mesh with MeshLib MRVoxels MeshVoxelsConverter-style mesh-to-level-set boolean semantics.",
            "Creates a ready child version with a persisted normalized PLY artifact for voxel union, intersection, or difference review.",
        ],
    },
    {
        "command_id": "collision-detect",
        "label": "Collision Detection",
        "group": "inspect",
        "endpoint_url_key": "collision_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["exact_mesh_intersections"],
        "notes": [
            "Rust exact face-pair collision detection follows MeshLib findCollidingTriangles broad-phase pair filtering and exact triangle-intersection semantics.",
            "first_intersection_only limits the returned payload to one pair; exact boolean assembly remains a separate parity track.",
        ],
    },
    {
        "command_id": "exact-boolean",
        "label": "Exact Boolean",
        "group": "inspect",
        "endpoint_url_key": "exact_boolean_endpoint_url",
        "rust_backed": True,
        "sdk_operations": ["exact_boolean_mesh"],
        "notes": [
            "Runs Rust exact_boolean_mesh with MeshLib MR::boolean-style union, intersection, difference, inside, and outside operations against another ready normalized mesh version.",
            "Creates a ready child version with a persisted normalized PLY artifact for review/history parity with MeshInspector workflows.",
        ],
    },
    {
        "command_id": "gcode-load-source",
        "label": "Load G-code Source",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "gcode_load_source_endpoint_url",
        "sdk_operations": ["load_gcode_source"],
        "notes": ["Rust G-code source loader follows MeshLib GcodeLoad::fromAnySupportedFormat for .gcode, .nc, and .txt files through the /gcode/load-source endpoint, preserving non-empty source frames including CRLF carriage returns."],
    },
    {
        "command_id": "gcode-write-source",
        "label": "Write G-code Source",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "gcode_write_source_endpoint_url",
        "sdk_operations": ["write_gcode_source"],
        "notes": ["Rust G-code source writer persists MeshLib ObjectGcode-style source frames through the /gcode/write-source endpoint for reloadable .gcode, .nc, and .txt workflows."],
    },
    {
        "command_id": "gcode-parse-file-paths",
        "label": "Parse G-code File Paths",
        "group": "inspect",
        "rust_backed": True,
        "endpoint_url_key": "gcode_parse_file_paths_endpoint_url",
        "sdk_operations": ["parse_gcode_file_paths"],
        "notes": ["Rust G-code file parser composes MeshLib-style source-file loading, including CRLF carriage-return frame preservation, with the Rust GcodeProcessor path conversion pipeline through the /gcode/parse-file-paths endpoint."],
    },
    {
        "command_id": "version-history",
        "label": "Version History",
        "group": "review",
        "endpoint_url_key": "model_versions_endpoint_url",
        "notes": ["Lists existing version records for review; no geometry mutation is performed."],
    },
    {
        "command_id": "restore-branch",
        "label": "Branch In History",
        "group": "review",
        "endpoint_url_key": "branch_endpoint_url",
        "notes": ["Branches a ready version by duplicating generated SDK artifacts."],
    },
    {
        "command_id": "job-activity",
        "label": "Job Activity",
        "group": "review",
        "endpoint_url_key": "jobs_endpoint_url",
        "notes": ["Streams backend job status and event records."],
    },
    {
        "command_id": "runtime-select-mark-region",
        "label": "Select / Mark Region",
        "group": "runtime",
        "endpoint_url_key": "selection_endpoint_url",
        "runtime_tool_id": "select_mark_region",
        "rust_backed": True,
        "sdk_operations": [
            "apply_meshlib_selection_modifier",
            "closest_points_on_mesh",
            "detect_ring_regions",
            "extract_selected_faces_as_mesh",
            "expand_face_selection_to_components",
            "graph_cut_select_region",
            "graph_cut_select_region_auto_not_region",
            "meshlib_select_scene_objects",
            "select_boundary_faces",
            "select_boundary_edges",
            "select_camera_facing_faces",
            "select_crease_edges",
            "select_degenerate_faces",
            "select_face_by_ray",
            "select_faces_by_area",
            "select_faces_by_screen_brush",
            "select_faces_by_screen_polygon",
            "select_faces_by_screen_rect",
            "select_inside_part_faces",
            "select_largest_component_faces",
            "select_not_smooth_faces",
            "select_outer_layer_faces",
            "select_overlapping_faces",
            "select_overhang_faces",
            "select_short_edges",
            "self_intersecting_faces",
        ],
    },
    {
        "command_id": "runtime-selection-to-object",
        "label": "Selection to Object",
        "group": "runtime",
        "endpoint_url_key": "selection_endpoint_url",
        "runtime_tool_id": "select_mark_region",
        "create_object": True,
        "rust_backed": True,
        "sdk_operations": [
            "apply_meshlib_selection_modifier",
            "extract_selected_faces_as_mesh",
        ],
    },
    {
        "command_id": "runtime-thicken-brush",
        "label": "Thicken Brush",
        "group": "runtime",
        "endpoint_url_key": "brush_endpoint_url",
        "runtime_tool_id": "thicken_brush",
        "rust_backed": True,
        "sdk_operations": ["apply_brush_strokes"],
    },
    {
        "command_id": "runtime-scoop-brush",
        "label": "Scoop Brush",
        "group": "runtime",
        "endpoint_url_key": "brush_endpoint_url",
        "runtime_tool_id": "scoop_brush",
        "rust_backed": True,
        "sdk_operations": ["apply_brush_strokes"],
    },
    {
        "command_id": "runtime-smooth-brush",
        "label": "Smooth Brush",
        "group": "runtime",
        "endpoint_url_key": "brush_endpoint_url",
        "runtime_tool_id": "smooth_brush",
        "rust_backed": True,
        "sdk_operations": ["apply_brush_strokes"],
    },
    {
        "command_id": "runtime-measure-inspect",
        "label": "Measure / Inspect",
        "group": "runtime",
        "endpoint_url_key": "measurement_endpoint_url",
        "runtime_tool_id": "measure_inspect",
        "rust_backed": True,
        "sdk_operations": ["closest_points_on_mesh", "feature_pair_measurements", "mesh_geodesic_path", "mesh_geodesic_polyline_path", "mesh_cut_measure_contours", "mesh_geodesic_quadrangle_path", "mesh_fast_marching_surface_path", "mesh_fast_marching_surface_path_tri_points", "mesh_surface_path_tri_points", "object_lines_from_contours", "mesh_geodesic_distance_field", "mesh_closest_surface_path_targets", "mesh_surface_distance_seed_vertices", "mesh_geodesic_iso_region", "mesh_geodesic_extreme_edges", "thickness_overlay_payload"],
    },
]

OFFICIAL_PARITY_INVENTORY = [
    {
        "official_feature_id": "file-scene-viewer",
        "label": "File, scene tree, viewer, history, and viewport tools",
        "group": "file",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshinspector.com/knowledge-base/object-types/meshes/",
            "https://meshinspector.com/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRCommonPlugins/MRRibbonCommonMenuStructure.ui.json",
            "MeshLib/source/MRCommonPlugins/ViewerButtons/MRRibbonSceneButtons.*",
            "MeshLib/source/MRViewer/MRSceneObjectsListDrawer.*",
            "MeshLib/source/MRMesh/MRObjectLoad.*",
            "MeshLib/source/MRMesh/MRMeshLoadObj.*",
            "MeshLib/source/MRMesh/MRPly.*",
            "MeshLib/source/MRMesh/MRObject.*",
            "MeshLib/source/MRMesh/MRChangeSceneObjectsOrder.*",
            "MeshLib/source/MRMesh/MRObjectMeshHolder.*",
            "MeshLib/source/MRMesh/MRObjectLines.*",
            "MeshLib/source/MRMesh/MRObjectLinesHolder.*",
            "MeshLib/source/MRMesh/MRObjectDistanceMap.*",
            "MeshLib/source/MRVoxels/MRObjectVoxels.*",
            "MeshLib/source/MRVoxels/MRVoxelsLoad.*",
            "MeshLib/source/MRVoxels/MRVoxelsSave.*",
            "MeshLib/source/MRMesh/MRFeatureObject.*",
            "MeshLib/source/MRMesh/MRPointObject.*",
            "MeshLib/source/MRMesh/MRLineObject.*",
            "MeshLib/source/MRMesh/MRPlaneObject.*",
            "MeshLib/source/MRMesh/MRSphereObject.*",
            "MeshLib/source/MRMesh/MRCircleObject.*",
            "MeshLib/source/MRMesh/MRCylinderObject.*",
            "MeshLib/source/MRMesh/MRConeObject.*",
            "MeshLib/source/MRViewer/MRRenderFeatureObjects.*",
            "MeshLib/source/MRMesh/MRDistanceMapLoad.*",
            "MeshLib/source/MRMesh/MRDistanceMapSave.*",
            "MeshLib/source/MRMesh/MRObjectSave.*",
            "MeshLib/source/MRMesh/MRObjectLoad.*",
            "MeshLib/source/MRMesh/MRZip.*",
            "MeshLib/source/MRMesh/miniply.*",
            "MeshLib/source/MRViewer/MRRibbonMenu.cpp",
            "MeshLib/source/MRViewer/MRViewer.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/mesh.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_obj.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_ply.rs",
            "geometry-rs/crates/zennah-geometry-core/src/meshlib_scene.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/scene.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/accelerators/_rust_mesh.py",
            "geometry_sdk/core/mesh.py",
            "geometry_sdk/io/trimesh_adapter.py",
        ],
        "backend_command_ids": [
            "upload-new",
            "download-stl",
            "wireframe",
            "snapshots",
            "version-history",
            "restore-branch",
            "job-activity",
        ],
        "validation_gates": [
            "cargo test -p zennah-geometry-core mesh_ply_import_prefers_meshlib_uv_short_names_over_texture_names",
            "cargo test -p zennah-geometry-core mesh_ply_import_reads_binary",
            "cargo test -p zennah-geometry-core mesh_ply_import_packs_polygon_texcoord_lists_like_meshlib",
            "cargo test -p zennah-geometry-core mesh_ply_import_keeps_polygon_face_colors_per_meshlib_source_face_row",
            "cargo test -p zennah-geometry-core mesh_ply_import_exposes_meshlib_vertex_normals_and_edges",
            "cargo test -p zennah-geometry-core mesh_ply_import_loads_first_existing_texture_like_meshlib_texturefile",
            "cargo test -p zennah-geometry-core mesh_ply_import_trims_meshlib_texturefile_comment_trailing_spaces",
            "cargo test -p zennah-geometry-core mesh_obj_import_triangulates_meshlib_negative_index_quad",
            "cargo test -p zennah-geometry-core mesh_obj_import_loads_meshlib_mtl_diffuse_texture_metadata",
            "cargo test -p zennah-geometry-core mesh_obj_import_loads_meshlib_map_kd_texture_image",
            "cargo test -p zennah-geometry-core meshlib_transform_scene_object_updates_world_vertices_from_object_xf",
            "cargo test -p zennah-geometry-core meshlib_multi_object_mru_scene_preserves_nested_object_children",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_lines_nodes",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_points_nodes",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_distance_map_nodes",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_voxels_nodes",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_voxels_gav_nodes",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_feature_object_nodes",
            "cargo test -p zennah-geometry-core meshlib_reparent_scene_object_updates_hierarchy_paths_like_add_child",
            "cargo test -p zennah-geometry-core meshlib_set_scene_object_state_serializes_visibility_and_lock_flags",
            "cargo test -p zennah-geometry-core meshlib_reorder_scene_children_matches_change_scene_objects_order",
            "cargo test -p zennah-geometry-core meshlib_scene_ribbon",
            "cargo test -p zennah-geometry-core meshlib_scene_tree_ribbon_actions_cover_imported_data_object_types",
            "cargo test -p zennah-geometry-core meshlib_scene_tree_sort_by_name_exports_mixed_data_children_in_meshlib_order",
            "cargo test -p zennah-geometry-core meshlib_scene_tree_group_and_ungroup_match_official_new_object_workflow",
            "cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core mesh_obj_import_preserves_meshlib_vt_uvs_for_textured_faces",
            "cargo check --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-py",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_exposes_meshlib_uv_and_color_metadata -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_exposes_binary_meshlib_metadata -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_packs_polygon_texcoord_lists_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_keeps_polygon_face_colors_per_meshlib_source_face_row -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_ply_uploads_through_rust_meshlib_parser -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_obj_import_triangulates_meshlib_negative_index_quad tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_obj_uploads_through_rust_meshlib_parser -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_obj_import_loads_meshlib_mtl_diffuse_texture_metadata tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_obj_mtl_metadata_through_rust_parser -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_loads_meshlib_obj_map_kd_texture_image -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_obj_vt_uvs_into_glb_preview -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_exposes_meshlib_ply_normals_and_edges -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_loads_first_existing_texture_like_meshlib_texturefile -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_trims_meshlib_texturefile_comment_trailing_spaces -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_and_workbench_manifests_expose_meshlib_texture_artifact tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_first_rust_loaded_meshlib_texture_artifact -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_and_workbench_manifests_expose_ordered_meshlib_texture_artifacts tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_all_obj_map_kd_textures_with_meshlib_texture_per_face -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_obj_map_kd_texture_artifact_with_meshlib_obj_source -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_engine_applies_meshlib_texture_artifact_to_mesh_materials -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_engine_applies_meshlib_texture_per_face_artifacts_to_material_groups -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_engine_uses_meshlib_texture_array_shader_before_material_group_fallback -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_meshlib_object_mesh_scene_payload_matches_object_mesh_holder_fields tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_meshlib_object_mesh_scene_json_artifact tests/test_geometry_sdk_operation_contracts.py::test_meshlib_object_mesh_mru_scene_matches_serialize_object_tree_layout tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_meshlib_mru_scene_artifact tests/test_geometry_sdk_operation_contracts.py::test_load_mesh_routes_mru_scene_through_rust_deserialize_object_tree tests/test_geometry_sdk_operation_contracts.py::test_load_mesh_routes_multi_object_mru_scene_hierarchy_through_rust tests/test_geometry_sdk_operation_contracts.py::test_load_mesh_preserves_mru_shared_model_links_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_round_trips_multi_object_hierarchy_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_lines_type_management_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_round_trips_shared_model_links_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_nested_object_tree_through_rust tests/test_geometry_sdk_operation_contracts.py::test_reparent_mru_scene_object_updates_tree_metadata_and_round_trips_through_rust tests/test_geometry_sdk_operation_contracts.py::test_set_mru_scene_object_state_updates_visibility_and_lock_flags_through_rust tests/test_geometry_sdk_operation_contracts.py::test_reorder_mru_scene_children_updates_export_order_through_rust tests/test_geometry_sdk_operation_contracts.py::test_transform_mru_scene_object_updates_xf_and_round_trips_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_apply_mru_scene_ribbon_actions_and_rename_route_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_mru_scene_tree_ribbon_actions_cover_imported_data_collections_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_group_and_ungroup_mru_scene_objects_route_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_points_type_management_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_distance_map_type_management_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_voxels_type_management_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_voxels_gav_payloads_through_rust -q",
            "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_voxels_vdb_payloads",
            "cargo test -p zennah-geometry-core object_voxels_vdb",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_voxels_vdb_payloads_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_load_meshlib_mru_scene_imports_half_float_active_mask_vdb_values_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_load_meshlib_mru_scene_imports_zip_compressed_vdb_values_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_load_meshlib_mru_scene_imports_blosc_compressed_vdb_values_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_feature_object_type_management_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_save_mesh_preserves_meshlib_vertex_uvs_through_ply_and_glb_preview tests/test_geometry_sdk_core.py::test_save_mesh_preserves_meshlib_tri_corner_uvs_in_ply_and_flattens_preview_uvs -q",
            "uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_architecture.py::test_meshlib_workbench_manifest_exposes_command_level_rust_capabilities -q",
        ],
        "non_geometry_reason": "Scene, upload/download, history, snapshot, and job-activity controls are host workflows; geometry mutation remains Rust-backed when a command runs a kernel.",
        "notes": [
            "mesh_from_ply is Rust-backed and default_sdk.load_mesh routes .ply uploads through it for MeshLib MRPly/miniply-style ASCII and binary little-/big-endian mesh PLY vertices, vertex normals, edge elements, triangle/polygon faces, r/g/b-over-red/green/blue color discovery including polygon face colors per source face row, u/v-over-s/t-over-texture_u/texture_v-over-texture_s/texture_t UV discovery, tri-corner texcoord lists including MeshLib-style polygon texcoord list packing, TextureFile comments with miniply-style leading/trailing comment whitespace trimming, first-existing PNG/JPEG/TIFF TextureFile image loading exposed through SDK metadata with MeshLib Linear/Clamp texture settings, ingest-time texture artifact URL/metadata handoff into viewer/workbench manifests, ordered multi-texture artifact manifests, normalized PLY UV/TextureFile export, preview GLB TEXCOORD_0 export for vertex and tri-corner UV sampling, viewer material texture application with Three.js Linear/Clamp settings, MeshLib texturePerFace viewer material groups, native MeshLib texture-array shader sampling, and Rust-backed MeshLib ObjectMeshHolder/ObjectLinesHolder/ObjectPointsHolder-style scene JSON serialization plus serializeObjectTree-style .mru package export/import, ObjectMesh multi-object hierarchy import/export round-trip with object XF transforms, nested object-tree export preservation, Link shared-model reuse, ObjectLines scene object import/export with Polyline.Points and flat Polyline.Lines preservation, ObjectPoints scene object import/export with MeshLib PointsSave/PointsLoad-style point PLY, normals, vertex colors, PointSize, MaxRenderingPoints, and state preservation, Rust-backed scene-object transform, reparent, state, reorder editing, RibbonMenu group/ungroup new-object workflows, and scene-tree ribbon Select all/Unselect all/Show all/Hide all/Show only previous/Show only next/Sort by name/Rename/Remove selected objects controls across ObjectMesh, ObjectLines, ObjectPoints, ObjectDistanceMap, ObjectVoxels, and FeatureObject collections, and artifact registration in viewer/workbench manifests. mesh_from_obj is Rust-backed and default_sdk.load_mesh routes .obj uploads through it for MeshLib MRMeshLoadObj-style vertex parsing, negative index resolution, object-name metadata, polygon fan triangulation, mtllib/usemtl material scopes, Kd diffuse color conversion, OBJ vt UV import into preview-ready tri-corner UVs, map_Kd texture-per-face metadata, map_Kd PNG/JPEG/TIFF texture image loading exposed through SDK metadata with MeshLib Linear/Clamp texture settings, OBJ texture artifact URL/metadata handoff that preserves MRMeshLoadObj provenance, ordered multi-texture artifact manifests, viewer material texture application with Three.js Linear/Clamp settings, MeshLib texturePerFace viewer material groups, native MeshLib texture-array shader sampling, and Rust-backed MeshLib ObjectMeshHolder/ObjectLinesHolder/ObjectPointsHolder-style scene JSON serialization plus serializeObjectTree-style .mru package export/import, ObjectMesh multi-object hierarchy import/export round-trip with object XF transforms, nested object-tree export preservation, Link shared-model reuse, ObjectLines scene object import/export with Polyline.Points and flat Polyline.Lines preservation, ObjectPoints scene object import/export with MeshLib PointsSave/PointsLoad-style point PLY, normals, vertex colors, PointSize, MaxRenderingPoints, and state preservation, Rust-backed scene-object transform, reparent, state, reorder editing, RibbonMenu group/ungroup new-object workflows, and scene-tree ribbon Select all/Unselect all/Show all/Hide all/Show only previous/Show only next/Sort by name/Rename/Remove selected objects controls across ObjectMesh, ObjectLines, ObjectPoints, ObjectDistanceMap, ObjectVoxels, and FeatureObject collections, and artifact registration in viewer/workbench manifests.",
            "ObjectDistanceMap scene object import/export is Rust-backed through geometry-rs meshlib_scene.rs with MeshLib .raw/.mrdistancemap parsing, .raw export, PixelXVec, PixelYVec, DepthVec, OriginWorld, valid-value stats, visibility, selection, lock, and parent-lock preservation.",
            "ObjectVoxels scene object import/export is Rust-backed through geometry-rs meshlib_scene.rs with MeshLib ObjectVoxels::serializeFields_ fields, raw .raw voxel payload import/export, filename-auto dimensions/voxelSize/gridLevelSet parsing, MeshLib VoxelsLoad::fromGav/MRVoxelsSave::toGav-style Micro CT .gav payload import/export, OpenVDB .vdb FloatGrid metadata import (active bbox dimensions, transform voxel size, and level-set class) plus uncompressed Tree_float_5_4_3, ZIP-compressed Tree_float_5_4_3, Blosc-compressed Tree_float_5_4_3, and active-mask Tree_float_5_4_3_HalfFloat dense value import (MeshLib x-fastest ordering, zlib and Blosc/LZ4 chunk decompression, half-float promotion, inactive background reconstruction, and min/max stats) plus model payload preservation/import/export, VoxelSize, Dimensions, MinCorner, MaxCorner, IsoValue, DualMarchingCubes, compact SelectionVoxels bitset import/export, raw/GAV value stats, visibility, selection, lock, and parent-lock preservation.",
            "FeatureObject scene object import/export is Rust-backed through geometry-rs meshlib_scene.rs with MeshLib FeatureObject::serializeFields_ fields, concrete PointObject/LineObject/PlaneObject/SphereObject/CircleObject/CylinderObject/ConeObject type preservation, XF-driven geometry, visualization masks, decoration colors, point/line size, alpha, and dimension-visibility preservation.",
            "FeatureObject render payload generation is Rust-backed through geometry-rs meshlib_scene.rs with MeshLib MRRenderFeatureObjects-style PointObject, LineObject, PlaneObject, CircleObject, MR::makeSphere/subdivideMesh edge-flip SphereObject topology, CylinderObject, and ConeObject primary primitives, DetailsOnNameTag labels, Diameter/Angle/Length dimension payloads, and visual Subfeatures payloads exposed through the Python SDK as a facade-only bridge.",
            "The official product exposes a broader object/data-type matrix than the current jewelry workspace.",
            "Current runtime embeds the official-style workbench contract but still stores app metadata in the FastAPI domain model; direct OpenVDB .vdb FloatGrid dense-payload dual meshing with OpenVDB active bbox origin preservation, distinct OpenVDB topology and value-buffer masks, tight sparse active-bbox, active-window boundary, and full-leaf-span sparse active-mask background halo padding, dense planar adaptivity coalescing, and MeshLib relaxDisorientedTriangles-style closed-surface ray-count face relaxation are Rust-backed, while exact sparse OpenVDB VolumeToMesh topology and curved/sparse adaptivity remain future parity work.",
        ],
    },
    {
        "official_feature_id": "selection-tools",
        "label": "Object, primitive, region, and brush selection",
        "group": "selection",
        "status": "partial",
        "official_sources": [
            "https://meshinspector.com/knowledge-base/selection/how-to-use-meshinspectors-select-tools/",
            "https://meshlib.io/feature/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRCommonPlugins/Selectors/MRSelectObjectByClick.cpp",
            "MeshLib/source/MRViewer/ImGuiMenu.cpp",
            "MeshLib/source/MRViewer/MRSelectScreenLasso.*",
            "MeshLib/source/MRMesh/MRFixSelfIntersections.*",
            "MeshLib/source/MRMesh/MRMeshFixer.*",
            "MeshLib/source/MRMesh/MRMeshMath.*",
            "MeshLib/source/MRMesh/MRFilterCreaseEdges.*",
            "MeshLib/source/MRMesh/MRMeshOverhangs.*",
            "MeshLib/source/MRMesh/MRMeshDoubleLayer.*",
            "MeshLib/source/MRMesh/MRFillContourByGraphCut.*",
            "MeshLib/source/MRMesh/MREdgeMetric.*",
            "MeshLib/source/MRViewer/MRSelectCurvaturePreference.*",
            "MeshLib/source/MRMesh/MROverlappingTris.*",
            "MeshLib/source/MRMesh/MRTriMath.h",
            "MeshLib/source/MRMesh/MRMeshCollide.*",
            "MeshLib/source/MRMesh/MRMeshComponents.*",
            "MeshLib/source/MRMesh/MRRegionBoundary.*",
            "MeshLib/source/MRMesh/MRMeshSegmentation.*",
            "MeshLib/source/MRViewer/MRRibbonMenu.cpp",
            "MeshLib/source/MRMesh/MRObjectMesh.*",
            "MeshLib/source/MRMesh/MRMesh.cpp",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/jewelry.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/selection_modifier.rs",
            "geometry-rs/crates/zennah-geometry-core/src/meshlib_scene.rs",
            "geometry-rs/crates/zennah-geometry-core/src/meshlib_scene/edit.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_degeneracy.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_smoothness.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/closest.rs",
            "geometry-rs/crates/zennah-geometry-py/src/jewelry.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/scene.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/scene/api_export.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/selection.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/selection_object.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_degeneracy.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_smoothness.rs",
            "geometry-rs/crates/zennah-geometry-py/src/spatial.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/accelerators/_rust_mesh.py",
            "geometry_sdk/accelerators/_rust_mesh_scene.py",
            "geometry_sdk/accelerators/_rust_mesh_selection.py",
            "geometry_sdk/accelerators/_rust_repair.py",
            "geometry_sdk/accelerators/_rust_smoothness.py",
            "geometry_sdk/core/mesh.py",
        ],
        "backend_command_ids": [
            "regions",
            "runtime-select-mark-region",
            "runtime-selection-to-object",
        ],
        "hosted_tool_ids": [
            "select_mark_region",
        ],
        "validation_gates": [
            "cargo test -p zennah-geometry-core expand_face_selection_to_components",
            "cargo test -p zennah-geometry-core select_largest_component_faces",
            "cargo test -p zennah-geometry-core select_boundary",
            "cargo test -p zennah-geometry-core select_faces_by_screen_polygon",
            "cargo test -p zennah-geometry-core select_faces_by_screen_rect",
            "cargo test -p zennah-geometry-core select_faces_by_screen_brush",
            "cargo test -p zennah-geometry-core ray_hits_cube_front_face",
            "cargo test -p zennah-geometry-core select_camera_facing_faces",
            "cargo test -p zennah-geometry-core select_degenerate_faces",
            "cargo test -p zennah-geometry-core select_short_edges",
            "cargo test -p zennah-geometry-core select_faces_by_area",
            "cargo test -p zennah-geometry-core select_overhang_faces",
            "cargo test -p zennah-geometry-core select_outer_layer_faces",
            "cargo test -p zennah-geometry-core select_not_smooth_faces",
            "cargo test -p zennah-geometry-core graph_cut_select_region",
            "cargo test -p zennah-geometry-core graph_cut_select_region_auto_not_region",
            "cargo test -p zennah-geometry-core graph_cut_select_region_uses_meshlib_curvature_preference_metric",
            "cargo test -p zennah-geometry-core select_overlapping_faces",
            "cargo test -p zennah-geometry-core select_inside_part_faces",
            "cargo test -p zennah-geometry-core extract_selected_faces_as_mesh",
            "cargo test -p zennah-geometry-core apply_meshlib_selection_modifier_matches_primary_ctrl_toggle_contract",
            "cargo test -p zennah-geometry-core meshlib_scene_selection_modifier_matches_name_tag_select_one_and_toggle",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_expand_face_selection_to_components_matches_meshlib_component_selection -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_largest_component_faces_matches_meshlib_surface_area_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_boundary_faces_and_edges_match_meshlib_boundary_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_degenerate_faces_matches_meshlib_aspect_ratio_and_boundary_filter -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_short_edges_matches_meshlib_critical_length_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_area_matches_meshlib_area_threshold_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_overhang_faces_matches_meshlib_layer_basement_and_normal_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_outer_layer_faces_matches_meshlib_double_layer_seed_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_not_smooth_faces_matches_meshlib_neighbor_angle_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_graph_cut_select_region_matches_meshlib_source_sink_edge_length_cut_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_graph_cut_select_region_auto_not_region_matches_meshinspector_uncertainty_workflow -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_graph_cut_select_region_matches_meshinspector_curvature_preference -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_overlapping_faces_matches_meshlib_opposite_close_triangle_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_inside_part_faces_matches_meshlib_winding_self_intersection_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_crease_edges_matches_meshlib_find_crease_edges_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_screen_polygon_matches_meshlib_lasso_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_screen_rect_matches_meshlib_rect_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_screen_brush_matches_meshlib_near_polygon_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_face_by_ray_matches_meshlib_pick_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_camera_facing_faces_matches_meshinspector_view_direction_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_spatial.py::test_triangle_intersection_detects_crossing_faces -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_expands_selected_faces_to_meshlib_components -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_largest_component_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_boundary_selectors -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_lasso_mask -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_rect_mask -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_brush_mask -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_pick_mask -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_camera_facing_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_degenerate_face_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_short_edge_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_area_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_crease_edge_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_overhang_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_outer_layer_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_not_smooth_faces_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_graph_cut_region_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_graph_cut_auto_not_region_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_graph_cut_curvature_preference -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_overlaps_mode -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_overlapping_faces_selector -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_inside_part_mode -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_extract_selected_faces_as_mesh_matches_meshlib_clone_region_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_extract_selected_faces_as_mesh_remaps_meshlib_clone_region_visual_attributes -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_apply_meshlib_selection_modifier_matches_primary_ctrl_toggle_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_applies_meshinspector_primary_ctrl_toggle_modifier -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_mesh_vertex_selection_applies_meshinspector_primary_ctrl_toggle_modifier -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_point_cloud_selection_applies_meshinspector_primary_ctrl_toggle_modifier -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_select_mru_scene_objects_applies_meshinspector_name_tag_modifier_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_can_create_meshlib_selection_to_object_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_parity.py -q",
            "uv run --extra dev pytest tests/test_versions_interactive_commit.py -q",
        ],
        "notes": [
            "Current support is ring-semantic region selection, closest-point brush resolution, MeshLib MeshComponents::getComponents-style shared-edge face-component expansion via selection.metadata.expand_to_components, MeshComponents::getLargestComponent-style largest-area component selector metadata via selection.metadata.selector=largest_component, MeshTopology::findBdFaces/findLeftBdEdges-style boundary face/edge selector metadata, MRSelectScreenLasso-style screen polygon selection via selection.metadata.selector=screen_lasso_faces, MRSelectScreenLasso-style screen rectangle selection via selection.metadata.selector=screen_rect_faces, MRSelectScreenLasso-style screen brush selection via selection.metadata.selector=screen_brush_faces, first_ray_hit-style primitive Pick selector via selection.metadata.selector=pick_face, MeshInspector Select Camera-Facing selector metadata via selection.metadata.selector=camera_facing_faces, SelfIntersections::getFaces strict face selector metadata via selection.metadata.selector=self_intersections, FastWindingNumber::calcSelfIntersections-style Inside Part selector via selection.metadata.selector=inside_part_faces or selection.metadata.selector=self_intersections with mode=inside_part, findDegenerateFaces-style aspect-ratio selector metadata via selection.metadata.selector=degenerate_faces, findShortEdges-style edge-length selector metadata via selection.metadata.selector=short_edges, Mesh::area-style Select by Area selector metadata via selection.metadata.selector=area_faces, findCreaseEdges-style Select Creases by Angle selector metadata via selection.metadata.selector=crease_edges, findOverhangs-style Select Overhangs selector metadata via selection.metadata.selector=overhang_faces, findOuterLayer-style Select Outer Layer selector metadata via selection.metadata.selector=outer_layer_faces, findNotSmoothFaces-style Select Not Smooth Triangles selector metadata via selection.metadata.selector=not_smooth_faces, segmentByGraphCut-style seeded Select Region selector metadata via selection.metadata.selector=graph_cut_region, automatic not-region workflow via uncertainty-distance sink seeding, edgeCurvMetric-style Curvature Preference metadata via selection.metadata.curvature_preference, findOverlappingTris-style Select Self-Intersections Overlaps mode via selection.metadata.selector=overlapping_faces or selection.metadata.selector=self_intersections with mode=overlaps, MeshInspector primary-control face-selection, mesh vertex-selection, point-cloud point-selection, and scene-tree object selection toggle semantics via selection.metadata.modifier_primary_ctrl / ctrlKey / metaKey / selection_modifier=toggle and meshlib_select_scene_objects, and MeshLib RibbonMenu::cloneSelectedPart/ObjectMesh::cloneRegion/Mesh::cloneRegion-style mesh face Selection to Object through create_object selection commits.",
            "Uncertainty distance beyond current automatic sink seeding and broader curvature segmentation beyond Select Region Curvature Preference remain future parity items.",
        ],
    },
    {
        "official_feature_id": "mesh-healer",
        "label": "Mesh healing, hole filling, and production repair",
        "group": "repair",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshlib.io/feature/mesh-healing/",
            "https://meshlib.io/documentation/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRMeshFillHole.*",
            "MeshLib/source/MRMesh/MRMeshFixer.*",
            "MeshLib/source/MRMesh/MRFilterCreaseEdges.*",
            "MeshLib/source/MRMesh/MRMeshComponents.*",
            "MeshLib/source/MRMesh/MRMeshCollide.*",
            "MeshLib/source/MRMesh/MRTunnelDetector.*",
            "MeshLib/source/MRMesh/MRMeshTopology.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/repair.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair/fill.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_components.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_degeneracy.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_diagnostics.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_holes.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_nonmanifold.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_smoothness.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair_tunnels.rs",
            "geometry-rs/crates/zennah-geometry-core/src/health_service.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_components.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_degeneracy.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_diagnostics.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_holes.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_nonmanifold.rs",
            "geometry-rs/crates/zennah-geometry-py/src/repair_smoothness.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/accelerators/_rust_nonmanifold.py",
            "geometry_sdk/accelerators/_rust_repair.py",
            "geometry_sdk/accelerators/_rust_tunnels.py",
            "geometry_sdk/repair/basic.py",
            "geometry_sdk/repair/holes.py",
            "geometry_sdk/repair/self_intersections.py",
        ],
        "backend_command_ids": [
            "repair",
            "make-manufacturable",
        ],
        "validation_gates": [
            "uv run --extra dev pytest tests/test_geometry_sdk_repair.py -q",
            "cargo test -p zennah-geometry-core find_disoriented_faces_matches_meshlib_ray_count_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_find_disoriented_faces_matches_meshlib_ray_count_contract -q",
            "cargo test -p zennah-geometry-core flip_normals_matches_meshlib_full_orientation_flip_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_flip_normals_matches_meshlib_orientation_flip_contract -q",
            "cargo test -p zennah-geometry-core fix_self_intersections_relax_matches_meshlib_relax_region_without_subdivision",
            "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_fix_self_intersections_relax_exposes_meshlib_relax_subset -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_architecture.py -q",
        ],
        "notes": [
            "Basic repair, MeshLib-style hole-fill plan diagnostics, service hole filling with MultipleEdgesResolveMode None/Simple/Strong dispatch, Simple-mode duplicate-edge avoidance, Strong-mode reused generated chord repair, outNewFaces new-face index reporting, maxPolygonSubdivisions split sampling, makeDegenerateBand duplicate-boundary band creation, stopBeforeBadTriangulation bad-patch guarding, smoothBd boundary-edge metric control, getMinAreaMetric double-area triangulation, getEdgeLengthFillMetric edge-length triangulation, getUniversalMetric universal smooth triangulation, getMaxDihedralAngleMetric max-dihedral-angle triangulation, getParallelPlaneFillMetric parallel-plane projection triangulation, getComplexFillMetric aspect-area edge-penalty triangulation, getMinTriAngleMetric minimum-angle triangulation, getPlaneFillMetric plane-normal triangulation, getPlaneNormalizedFillMetric plane-normalized aspect triangulation, getComplexStitchMetric aspect-ratio/dihedral stitch triangulation, getEdgeLengthStitchMetric edge-length stitch triangulation, getVerticalStitchMetric caller-supplied upDir vertical stitch triangulation, and getVerticalStitchMetricEdgeBased caller-supplied upDir vertical edge-projection stitch triangulation, short-edge, degenerate-face, multiple-edge, not-smooth-face, MeshLib findDisorientedFaces ray-count disorientation selection, MeshTopology::flipOrientation full-face normal flipping, crease-edge diagnostics, crease repair plan diagnostics, crease repair execution, boundary-only close-vertex uniting, crease component-length filtering, crease branch-length filtering, multiple-edge repair, MeshBuilder-style non-manifold edge face-pruning repair, MeshBuilder-style duplicateNonManifoldVertices disconnected, repeated-neighbor, face-region scoped, partial-triangulation lastValidVert duplicate-id allocation, and single-pass path-orientation behavior, multi-hole vertex duplication, area-based component pruning, SelfIntersections::getFaces strict non-touching face detection, SelfIntersections::fix Relax topology-preserving repair with subdivision disabled, Rust topological tunnel diagnostics, MeshLib-oracle 24x8/24x10/24x12 torus detectTunnelFaces face-band selection, MeshLib-oracle torus eliminateTunnels delete-and-fill repair, SDF rebuild self-intersection repair, and Mesh Healer diagnostics are wired through Rust-backed SDK operations.",
            "Full official healing parity still needs MeshLib min-area fill-hole parity for every crease-repair topology case, broader MRTunnelDetector arbitrary co-loop face-band selection and eliminateTunnels repair, and SelfIntersections::fix CutAndFill, degeneracy preprocessing, and subdivision/remesh parity.",
        ],
    },
    {
        "official_feature_id": "mesh-edit-simplify",
        "label": "Mesh edit, smoothing, simplification, and deformation",
        "group": "edit",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshlib.io/feature/mesh-simplification/",
            "https://meshinspector.com/knowledge-base/mesh-editing/mesh-cut-and-measure-in-meshinspectors-mesh-edit/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRMeshDecimate.*",
            "MeshLib/source/MRMesh/MRMeshDecimateCallbacks.*",
            "MeshLib/source/MRMesh/MRMeshDelone.*",
            "MeshLib/source/MRMesh/MRMeshRelax.*",
            "MeshLib/source/MRMesh/MRMeshSubdivide.*",
            "MeshLib/source/MRMesh/MRLaplacian.*",
            "MeshLib/source/MRViewer/MRSurfaceManipulationWidget.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/deform.rs",
            "geometry-rs/crates/zennah-geometry-core/src/deform_smooth.rs",
            "geometry-rs/crates/zennah-geometry-core/src/deform_target.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_edit.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_edit/decimate.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_edit/decimate/helpers.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_edit/smooth.rs",
            "geometry-rs/crates/zennah-geometry-core/src/resize.rs",
            "geometry-rs/crates/zennah-geometry-py/src/deform.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh_edit.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/accelerators/_rust_mesh_edit.py",
            "geometry_sdk/mesh_edit/__init__.py",
        ],
        "backend_command_ids": [
            "resize",
            "fit-size",
            "smooth",
            "batch-smooth",
            "decimate-mesh",
            "subdivide-mesh",
            "make-delone",
            "scoop",
            "runtime-scoop-brush",
            "runtime-smooth-brush",
        ],
        "hosted_tool_ids": [
            "scoop_brush",
            "smooth_brush",
        ],
        "validation_gates": [
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_interpolates_vertex_uvs_with_meshlib_pre_collapse_callback",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_interpolates_vertex_colors_with_meshlib_pre_collapse_truncation",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_edges_to_collapse_subset_and_remaps_it",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_empty_meshlib_edges_to_collapse_subset",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_critical_triangle_aspect_ratio_relaxation",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_tiny_edge_length_aspect_bypass",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_max_angle_change_delone_flip",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_flips_meshlib_twin_edge_with_max_angle_change",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_remaps_meshlib_twin_map_after_collapse",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_collapses_meshlib_twin_edge_with_same_position",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core subdivide_mesh",
            "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core make_delone_edge_flips",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_interpolates_vertex_uvs_with_meshlib_pre_collapse_callback -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_interpolates_vertex_colors_with_meshlib_pre_collapse_truncation -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_edges_to_collapse_subset_and_remaps_it -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_empty_meshlib_edges_to_collapse_subset -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_critical_triangle_aspect_ratio_relaxation -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_tiny_edge_length_aspect_bypass -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_max_angle_change_delone_flip -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_flips_meshlib_twin_edge_with_max_angle_change -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_remaps_meshlib_twin_map_after_collapse -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_collapses_meshlib_twin_edge_with_same_position -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_make_delone_edge_flips_matches_meshlib_quadrangle_diagonal_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_parity.py -q",
            "uv run --extra dev pytest tests/test_versions_interactive_commit.py -q",
        ],
        "notes": [
            "Current edit parity covers jewelry resize, local scoop, smooth, interactive replay, Rust-backed MR::decimateMesh DecimateStrategy::MinimizeError QEM with target triangle count/percentage stop controls through maxDeletedFaces, stabilizer and angleWeightedDistToPlane face-plane weighting and ShortestEdgeFirst subset for maxError stop behavior, FaceBitSet region masks, maxEdgeLen/deletion limits including MeshLib's unbounded-default half-face guard, maxBdShift boundary-shift guards, maxTriangleAspectRatio collapse guards, criticalTriAspectRatio aspect-relaxation guard, tinyEdgeLength endpoint aspect-bypass guard, maxAngleChange local Delone flip guard, touchNearBdEdges boundary filtering, touchBdVerts boundary-vertex preservation, notFlippable adjacent-collapse guards with crease-form QEM weighting, optimized collapse positions, notFlippable dynamic remapping with remapped_not_flippable_edges metadata, edgesToCollapse collapse subset and remapping metadata, twinMap symmetric validation plus paired same-position collapse, paired maxAngleChange Delone flips, and collapse/flip/pack remapping metadata, MeshLib preCollapseVertAttribute-style vertex_uvs and vertex_colors interpolation, packMesh output, subdivideParts part partitioning, decimateBetweenParts final pass, Rust-backed MeshLib-oracle subdivision for SubdivideSettings maxEdgeLen/curvaturePriority/maxEdgeSplits plus FaceBitSet region masks, notFlippable protected Delone-ring edge guards with split-edge remapping, maxDeviationAfterFlip, maxAngleChangeAfterFlip, criticalAspectRatioFlip, aspect-ratio stop/splittable gates, projectOnOriginalMesh projection, smoothMode cotan positioning with minSharpDihedralAngle sharp-vertex fixing, and standalone MR::makeDeloneEdgeFlips local Delone edge flips with region masks, iteration control, maxDeviationAfterFlip diagonal-deviation guard, maxAngleChange dihedral-delta guard, criticalTriAspectRatio angle-guard override, notFlippable edge constraints, and vertRegion vertex constraints.",
            "Arbitrary preCollapse callbacks and true threaded execution, remeshing composition, broader smoothMode crease-topology oracles, broader Delone topology cases, and full Mesh Cut & Measure spline/control-point/Fast-Marching/cut-export workflows remain open.",
        ],
    },
    {
        "official_feature_id": "surface-manipulation-brushes",
        "label": "Rust-backed interactive surface manipulation brushes",
        "group": "edit",
        "status": "implemented",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshlib.io/documentation/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRViewer/MRSurfaceManipulationWidget.*",
            "MeshLib/source/MRMesh/MROffsetVerts.*",
            "MeshLib/source/MRMesh/MRMeshRelax.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/deform.rs",
            "geometry-rs/crates/zennah-geometry-core/src/deform_smooth.rs",
            "geometry-rs/crates/zennah-geometry-core/src/deform_target.rs",
            "geometry-rs/crates/zennah-geometry-py/src/deform.rs",
        ],
        "backend_command_ids": [
            "runtime-thicken-brush",
            "runtime-scoop-brush",
            "runtime-smooth-brush",
        ],
        "hosted_tool_ids": [
            "thicken_brush",
            "scoop_brush",
            "smooth_brush",
        ],
        "validation_gates": [
            "uv run --extra dev pytest tests/test_versions_interactive_commit.py -q",
            "cargo test -p zennah-geometry-core",
        ],
        "notes": [
            "Brush replay is committed through backend endpoints and does not rely on frontend-only mesh state.",
        ],
    },
    {
        "official_feature_id": "boolean-collision",
        "label": "Exact and voxel boolean operations plus collision detection",
        "group": "boolean",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshlib.io/feature/mesh-boolean/",
            "https://meshlib.io/feature/collision-detection/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRMeshBoolean.*",
            "MeshLib/source/MRMesh/MRMeshBooleanFacade.*",
            "MeshLib/source/MRMesh/MRMeshCollide.*",
            "MeshLib/source/MRVoxels/MRBoolean.*",
            "MeshLib/source/MRVoxels/MRVoxelsBooleanOperation.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/spatial/exact_intersections.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/exact_kernel.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/exact_boolean.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/exact_boolean_candidate.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/exact_boolean_diagnostics.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_mesh_ops.rs",
            "geometry-rs/crates/zennah-geometry-py/src/spatial.rs",
            "geometry-rs/crates/zennah-geometry-py/src/boolean.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/spatial/intersections.py",
            "geometry_sdk/voxel/mesh_ops.py",
        ],
        "backend_command_ids": ["exact-boolean", "voxel-boolean", "collision-detect"],
        "validation_gates": [
            "cargo test -p zennah-geometry-core exact_mesh_intersections",
            "uv run --extra dev pytest tests/test_geometry_sdk_spatial.py::test_exact_mesh_intersections_exposes_meshlib_style_collision_face_pairs -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_collision_detect_endpoint_returns_meshlib_style_rust_payload -q",
            "cargo test -p zennah-geometry-core spatial::exact_boolean",
            "uv run --extra dev pytest tests/test_geometry_sdk_parity.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_exact_boolean_endpoint_creates_rust_backed_child_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_boolean_endpoint_creates_rust_backed_child_version -q",
        ],
        "notes": [
            "exact-boolean is exposed as a Rust-backed MeshLib MR::boolean-style command that creates a child version from two normalized mesh versions.",
            "voxel-boolean is exposed as a Rust-backed MeshLib MRVoxels MeshVoxelsConverter-style command that creates a child version from two normalized mesh versions.",
            "collision-detect is exposed as a Rust-backed MeshLib findCollidingTriangles-style exact face-pair collision command.",
            "The combined aggregate Boolean / Collision product workflow remains partial until broader official boolean UX parity is complete.",
        ],
    },
    {
        "official_feature_id": "offset-shell",
        "label": "Offset, shell, hollow, drain, and local thickening",
        "group": "offset",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshlib.io/feature/mesh-offsetting/",
            "https://meshinspector.com/knowledge-base/getting-started/using-the-offset-tool-in-meshinspector/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRVoxels/MROffset.*",
            "MeshLib/source/MRVoxels/MRPartialOffset.*",
            "MeshLib/source/MRVoxels/MRWeightedPointsShell.*",
            "MeshLib/source/MRMesh/MROffsetVerts.*",
            "MeshLib/source/MRMesh/MROffsetContours.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/hollow.rs",
            "geometry-rs/crates/zennah-geometry-core/src/hollow_service.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_mesh_ops.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_partial_offset.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh_edit/offset_verts.rs",
            "geometry-rs/crates/zennah-geometry-core/src/deform/offset.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh_edit.rs",
            "geometry-rs/crates/zennah-geometry-py/src/hollow.rs",
        ],
        "backend_command_ids": [
            "offset-mesh",
            "shell-mesh",
            "thicken-mesh",
            "weighted-shell",
            "partial-offset",
            "offset-verts",
            "expand-shrink",
            "shrink-expand",
            "protected-hollow",
            "hollow-drains",
            "reduce-weight",
            "prepare-casting",
            "thicken-violations",
            "thicken-region",
            "batch-thicken",
            "runtime-thicken-brush",
        ],
        "hosted_tool_ids": [
            "thicken_brush",
        ],
        "validation_gates": [
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_offset_mesh_endpoint_creates_rust_backed_child_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_shell_mesh_endpoint_creates_rust_backed_child_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_offset_smoothing_endpoint_sequences_official_signed_offsets -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_partial_offset_endpoint_creates_rust_backed_child_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_offset_verts_endpoint_creates_rust_backed_child_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_hollow.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_parity.py -q",
            "cargo test -p zennah-geometry-core hollow",
            "cargo test -p zennah-geometry-core voxel_partial_offset_mesh",
            "cargo test -p zennah-geometry-core offset_verts_mesh",
        ],
        "notes": [
            "offset-mesh, shell-mesh, thicken-mesh, weighted-shell, partial-offset, offset-verts, expand-shrink, and shrink-expand expose official MeshInspector Offset tool modes through Rust child-version commands.",
            "Jewelry hollowing, drain planning, and local thickening are backed by Rust paths.",
            "Broader lower-level offset-contours intersection index maps remain incomplete; the closed clockwise signed round-corner fixed-offset, positive sharp-corner fixed-offset with maxSharpAngle limiting, MeshLib default 3D signed fixed/variable Type::Offset, sharp max-angle, fixed/variable shell Z restore/one-pass default relaxation, explicit relaxIterations, constant/custom source-Z restore plus callable zCallback output/index/origin context, positive closed fixed/variable non-intersection, closed fixed zero-offset identity indicesMap/origin output, plus negative and shell-inner closed fixed/variable intersection indicesMap/origin output, signed variable-offset Type::Offset round/sharp-corner with maxSharpAngle limiting, positive fixed/variable including unequal-variable and mixed-signed Type::Offset final-outline self-overlap remap with indicesMap intersections, signed variable-offset shell round/sharp-corner with maxSharpAngle limiting including empty negative-shell output, signed fixed-offset shell mode, open round/cut end fixed-offset, open fixed bent/zig and variable bent/zig round-end indicesMap/origin output, open fixed cut-end connected collinear seam-preserving axis/non-axis plus axis/non-axis shifted parallel global-outline composition, axis-aligned perpendicular crossing, horizontal/vertical/non-axis touching-chain including horizontal direction variants, direction-reversed vertical and diagonal origin maps, and first-direction-reversed vertical/diagonal outline ordering, axis/non-axis overlapping-parallel, and axis/non-axis collinear-overlap plus direction-reversed horizontal collinear-overlap including first-source and both-reversed ordering, vertical direction variants, diagonal direction variants, and three-segment horizontal/vertical/diagonal collinear-overlap chains including diagonal chain direction variants global-outline indicesMap/origin output, and open variable-offset cut-end contour slices are Rust-backed under the distance-map/lines feature.",
        ],
    },
    {
        "official_feature_id": "features-measurement",
        "label": "Feature creation, dimensions, sections, heatmaps, and inspection measurement",
        "group": "inspect",
        "status": "partial",
        "official_sources": [
            "https://meshinspector.com/knowledge-base/inspect-measure/how-to-create-and-measure-features-in-meshinspector/",
            "https://meshinspector.com/knowledge-base/inspect-measure/surface-distance/",
            "https://meshinspector.com/knowledge-base/mesh-editing/mesh-cut-and-measure-in-meshinspectors-mesh-edit/",
            "https://meshlib.io/feature/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRFeatures.*",
            "MeshLib/source/MRMesh/MRFeatureObject.*",
            "MeshLib/source/MRMesh/MRFeatureRefine.*",
            "MeshLib/source/MRMesh/MRConeApproximator.*",
            "MeshLib/source/MRMesh/MRConeObject.*",
            "MeshLib/source/MRMesh/MRCylinderApproximator.*",
            "MeshLib/source/MRMesh/MREdgePaths.*",
            "MeshLib/source/MRMesh/MRMeshProject.*",
            "MeshLib/source/MRMesh/MRMeshSection.*",
            "MeshLib/source/MRMesh/MRSurfaceDistance.*",
            "MeshLib/source/MRMesh/MRSurfaceDistanceBuilder.*",
            "MeshLib/source/MRMesh/MRSurfacePath.*",
            "MeshLib/source/MRMesh/MROneMeshContours.*",
            "MeshLib/source/MRMesh/MRContoursCut.*",
            "MeshLib/source/MRMesh/MRObjectLines.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/analysis.rs",
            "geometry-rs/crates/zennah-geometry-core/src/analysis/section.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/cone_approx.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/intersections.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/measure.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/objects.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/cylinder_approx.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/refine.rs",
            "geometry-rs/crates/zennah-geometry-core/src/features/support.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/fast_marching.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/fast_marching_reduce.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/fast_marching_prune.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_extreme.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_quadrangle.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_descent.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_strip.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/surface_distance.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/surface_path.rs",
            "geometry-rs/crates/zennah-geometry-core/src/mesh/triangle_strip.rs",
            "geometry-rs/crates/zennah-geometry-core/src/lines.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/closest.rs",
            "geometry-rs/crates/zennah-geometry-core/src/distance.rs",
            "geometry-rs/crates/zennah-geometry-py/src/analysis.rs",
            "geometry-rs/crates/zennah-geometry-py/src/features.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/fast_marching.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/core.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic_descent.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic_extreme.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic_strip.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/surface_path.rs",
            "geometry-rs/crates/zennah-geometry-py/src/mesh/triangle_strip.rs",
            "geometry-rs/crates/zennah-geometry-py/src/lines.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/accelerators/_rust_fast_marching.py",
            "geometry_sdk/accelerators/_rust_features.py",
            "geometry_sdk/accelerators/_rust_geodesic.py",
            "geometry_sdk/distance_map/lines.py",
            "geometry_sdk/core/mesh.py",
        ],
        "backend_command_ids": [
            "export-section",
            "section",
            "heatmap",
            "measure-inspect",
            "mesh-cut-measure-path",
            "runtime-measure-inspect",
        ],
        "hosted_tool_ids": [
            "measure_inspect",
        ],
        "validation_gates": [
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_measure_inspect_endpoint_returns_rust_geodesic_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_measure_inspect_endpoint_returns_rust_fast_marching_mesh_tri_point_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_path_matches_meshlib_edge_shortest_path_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_polyline_path_exposes_control_vertex_surface_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_cut_measure_contours_matches_meshlib_onemesh_contour_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_cut_measure_edge_path_topology_cut_splits_shared_edge_seam -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_mesh_cut_measure_topology_endpoint_registers_rust_cut_child_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_quadrangle_path_matches_meshlib_reduce_path_crossing_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_triangle_step_matches_meshlib_triangle_exit_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_edge_step_matches_meshlib_edgepoint_vertex_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_vertex_step_matches_meshlib_vertid_triangle_exit_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_path_matches_meshlib_descent_path_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_fast_marching_surface_path_matches_meshlib_vertex_endpoint_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_fast_marching_surface_path_tri_points_stops_in_end_triangle_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_reduces_single_crossing_like_meshlib_compute_surface_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_reduces_unfolded_triangle_strip_like_meshlib_compute_surface_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_avoids_adjacent_face_vertex_like_meshlib_reduce_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_avoids_non_adjacent_vertex_fan_like_meshlib_reduce_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_removes_repeated_edge_vertex_detour_like_meshlib_reduce_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_removes_duplicate_nonvertex_location_like_meshlib_reduce_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_removes_same_triangle_nonvertex_detour_like_meshlib_reduce_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_collapses_repeated_location_strip_vertex_run_like_meshlib_reduce_path -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_planar_triangle_strip_path_matches_meshlib_funnel_crossing_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_triangle_strip_unfolded_path_matches_meshlib_unfolder_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_edge_point_path_matches_meshlib_surface_path_length_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_edge_point_path_matches_meshlib_geodesic_path_length_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_distance_field_matches_meshlib_surface_distance_seed_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_distance_field_uses_meshlib_triangle_front_update -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_closest_surface_path_targets_match_meshlib_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_distance_seed_vertices_exposes_official_source_modes -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_iso_region_exposes_surface_distance_cut_select_foundation -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_extreme_edges_match_meshlib_ridge_and_gorge_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_feature_pair_measurements_expose_meshlib_center_distance_and_angle -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_feature_pair_measurements_match_meshlib_parallel_cylinder_center_distance_fallback -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_feature_object_descriptors_match_meshlib_primitive_to_object_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_refine_feature_primitives_matches_meshlib_plane_refine_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_refine_feature_primitives_uses_meshlib_cylinder_approximation -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_refine_feature_primitives_uses_meshlib_cone_approximation -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_section.py -q",
            "cargo test -p zennah-geometry-core analysis",
            "cargo test -p zennah-geometry-core cone_",
            "cargo test -p zennah-geometry-core cone_approximation_matches_meshlib_partial_arc_fixture",
            "cargo test -p zennah-geometry-core feature_center_distance_matches_meshlib_parallel_cylinder_fallback",
            "cargo test -p zennah-geometry-core feature_",
            "cargo test -p zennah-geometry-core cylinder_approximation_matches_meshlib_partial_arc_fixture",
            "cargo test -p zennah-geometry-core mesh_geodesic_path",
            "cargo test -p zennah-geometry-core mesh_geodesic_polyline_path",
            "cargo test -p zennah-geometry-core mesh_cut_measure_contours",
            "cargo test -p zennah-geometry-core mesh_cut_measure_edge_path_topology_cut",
            "cargo test -p zennah-geometry-core mesh_geodesic_quadrangle_path",
            "cargo test -p zennah-geometry-core mesh_steepest_descent_triangle_step",
            "cargo test -p zennah-geometry-core mesh_steepest_descent_edge_step",
            "cargo test -p zennah-geometry-core mesh_steepest_descent_vertex_step",
            "cargo test -p zennah-geometry-core mesh_steepest_descent_path",
            "cargo test -p zennah-geometry-core mesh_fast_marching_surface_path",
            "cargo test -p zennah-geometry-core mesh_fast_marching_surface_path_tri_points",
            "cargo test -p zennah-geometry-core mesh_surface_path_tri_points",
            "cargo test -p zennah-geometry-core mesh_planar_triangle_strip_path",
            "cargo test -p zennah-geometry-core mesh_triangle_strip_unfolded_path",
            "cargo test -p zennah-geometry-core mesh_surface_edge_point_path",
            "cargo test -p zennah-geometry-core mesh_geodesic_edge_point_path",
            "cargo test -p zennah-geometry-core mesh_geodesic_distance_field",
            "cargo test -p zennah-geometry-core mesh_closest_surface_path_targets",
            "cargo test -p zennah-geometry-core mesh_surface_distance_seed_vertices",
            "cargo test -p zennah-geometry-core mesh_geodesic_iso_region",
            "cargo test -p zennah-geometry-core mesh_geodesic_extreme_edges",
        ],
        "notes": [
            "Current measurement covers jewelry dimensions, section contours, heatmap fields, closest-point inspection, Euclidean point-pair distances, MeshLib MR::buildShortestPath-style vertex-edge geodesic path length with line-segment and path-point output, MeshLib buildShortestPath-style open and closed multi-control geodesic polylines for Mesh Cut & Measure path foundations, Rust-backed MeshLib convertSurfacePathsToMeshContours / cutMesh-style OneMeshContour cut-input payloads and result-cut edge-path placeholders for vertex-control paths, Rust-backed edge-aligned MR::cutMesh seam topology mutation with duplicated cut-side vertices and child-version normalized_mesh_ply export, Rust-backed MeshLib shortestPathInQuadrangle/reducePath-style two-triangle surface path refinement payloads, Rust-backed MeshLib findSteepestDescentPoint(MeshTriPoint)-style Fast Marching descent edge-exit payloads, Rust-backed MeshLib findSteepestDescentPoint(MeshEdgePoint)-style shared-edge descent payloads, Rust-backed MeshLib findSteepestDescentPoint(VertId)-style vertex descent payloads, Rust-backed MeshLib computeSteepestDescentPath-style scalar-field edge-crossing descent paths, Rust-backed MeshLib computeFastMarchingPath-style vertex-endpoint and arbitrary MeshTriPoint approximate surface paths, Rust-backed MeshLib computeSurfacePath/reducePath-style single-crossing, unfolded triangle-strip, adjacent-face plus non-adjacent vertex-fan avoidance, repeated-edge vertex-detour simplification, duplicate non-vertex location removal, same-triangle non-vertex detour pruning, repeated-location strip same-vertex run collapse, topology-changing return-count and max-iteration gating semantics, and unfolded-strip vertex-run collapse for MeshTriPoint surface paths, Rust-backed MeshLib PathInPlanarTriangleStrip/reducePath-style unfolded strip funnel crossing payloads, Rust-backed MeshLib TriangleStripUnfolder/reducePath-style mesh triangle-strip unfolding payloads, Rust-backed MeshLib surfacePathLength/surfacePathToContour3f edge-point contour payloads, Rust-backed MeshLib geodesicPathLength/geodesicPathToContour3f endpoint contour payloads, Rust-backed MeshLib ObjectLines path export payloads for measured geodesic paths, Rust-backed MeshLib ObjectPoints/PointCloud-style geodesic path export payloads with area-weighted path point normals, MeshLib Features Point/Sphere/Line/Plane/Circle/Cylinder primitive MeasureResult exact distance, centerDistance including ConeSegment mostly-parallel cylinder fallback, axis/normal angle, supported intersection primitive payloads, MeshLib primitiveToObject-style PointObject/SphereObject/LineObject/PlaneObject/CircleObject/CylinderObject/ConeObject descriptors with shared editable property names/kinds/values, MeshLib ConeObject projectPoint-style cone projection helpers, MeshLib refineFeatureObject-style point/line/plane/sphere/circle mesh-vertex refinement with distance and normal gating plus convergence diagnostics, MeshLib Cylinder3Approximation-style cylinder refinement over partial cylinder arcs, MeshLib Cone3Approximation Levenberg-Marquardt-style cone refinement over partial cone arcs, MeshLib computeClosestSurfacePathTargets-style seeded surface-distance fields and closest-target mapping over mesh vertices, MeshLib findExtremeEdges-style ridge/gorge scalar-field edge extraction, Surface Distance Pick Point, Selected Edges, and Selected Triangles Boundary seed source resolution, and Rust-backed Surface Distance iso-value selected/crossing face, iso-segment extraction, and clipped-inside mesh payloads.",
            "Full official FeatureObject transform editing/viewport visualization/selection semantics, broader cone and cylinder refinement oracle coverage, non-zero-radius cone/cone exact distance cases that MeshLib still leaves unimplemented, Fast Marching reducePath remaining broad repeated-location topology simplification and full iterative geodesic refinement beyond the focused edge-crossing strip, vertex-fan, repeated-edge detour, duplicate non-vertex, same-triangle non-vertex, unfolded-strip vertex-run, and repeated-location strip vertex-run cases, full Surface Distance cut topology rewriting, and arbitrary-contour Mesh Cut & Measure topology mutation, bad-face fill modes, and non-edge path child-version exports beyond the current edge-aligned Rust seam subset are still partial.",
        ],
    },
    {
        "official_feature_id": "compare-report",
        "label": "Deviation comparison, signed distance, and QA reporting",
        "group": "compare",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshinspector.com/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRMeshMeshDistance.*",
            "MeshLib/source/MRMesh/MRPointsToMeshProjector.*",
            "MeshLib/source/MRViewer/MRDistanceMapWidget.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/distance.rs",
            "geometry-rs/crates/zennah-geometry-core/src/spatial/sign.rs",
            "geometry-rs/crates/zennah-geometry-py/src/compare_service.rs",
            "geometry-rs/crates/zennah-geometry-py/src/signed_distance.rs",
        ],
        "backend_command_ids": [
            "compare-versions",
        ],
        "validation_gates": [
            "uv run --extra dev pytest tests/test_geometry_sdk_compare.py -q",
            "uv run --extra dev pytest tests/test_versions_compare.py -q",
            "cargo test -p zennah-geometry-core distance",
        ],
        "notes": [
            "Current compare command is Rust-backed for service compare fields and summaries.",
            "Official side-by-side QA report authoring and export templates are still incomplete.",
        ],
    },
    {
        "official_feature_id": "point-cloud-icp",
        "label": "Point clouds, scan alignment, triangulation, and ICP",
        "group": "point_cloud",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshinspector.com/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRPointsInBall.*",
            "MeshLib/source/MRMesh/MRPointsProject.*",
            "MeshLib/source/MRMesh/MRPointsToMeshProjector.*",
            "MeshLib/source/MRMesh/MRUniformSampling.*",
            "MeshLib/source/MRMesh/MRGridSampling.*",
            "MeshLib/source/MRMesh/MRMeshOrPoints.*",
            "MeshLib/source/MRMesh/MRObjectPoints.*",
            "MeshLib/source/MRMesh/MRPointCloudTriangulation.*",
            "MeshLib/source/MRMesh/MRPointCloudTriangulationHelpers.*",
            "MeshLib/source/MRMesh/MRLocalTriangulations.*",
            "MeshLib/source/MRMesh/MRUnorientedTriangle.h",
            "MeshLib/source/MRMesh/MRICP.*",
            "MeshLib/source/MRMesh/MRMultiwayICP.*",
            "MeshLib/source/MRViewer/MRViewport.*",
            "MeshLib/source/MRViewer/MRSurfacePointPicker.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud.rs",
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan.rs",
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/optimizer.rs",
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/repetitions.rs",
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/topology.rs",
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/fill.rs",
            "geometry-rs/crates/zennah-geometry-core/src/point_cloud/projection.rs",
            "geometry-rs/crates/zennah-geometry-core/src/repair/fill.rs",
            "geometry-rs/crates/zennah-geometry-py/src/point_cloud.rs",
            "geometry-rs/crates/zennah-geometry-py/src/point_cloud_topology.rs",
            "geometry-rs/crates/zennah-geometry-py/src/point_cloud_fill.rs",
            "geometry-rs/crates/zennah-geometry-py/src/point_cloud_projection.rs",
            "geometry-rs/crates/zennah-geometry-core/src/registration.rs",
            "geometry-rs/crates/zennah-geometry-core/src/registration/multiway.rs",
            "geometry-rs/crates/zennah-geometry-core/src/registration/multiway/all_object.rs",
            "geometry-rs/crates/zennah-geometry-core/src/registration/multiway/cascade.rs",
            "geometry-rs/crates/zennah-geometry-py/src/registration.rs",
            "geometry-rs/crates/zennah-geometry-py/src/registration/cascade.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/point_cloud/icp.py",
            "geometry_sdk/point_cloud/multiway.py",
        ],
        "backend_command_ids": [
            "point-cloud-icp",
            "point-cloud-triangulate",
            "point-cloud-multiway-icp",
            "runtime-selection-to-object",
        ],
        "validation_gates": [
            "cargo test -p zennah-geometry-core point_cloud_nearest_projections",
            "cargo test -p zennah-geometry-core point_cloud_project_to_mesh",
            "cargo test -p zennah-geometry-core point_cloud_n_closest_neighbors",
            "cargo test -p zennah-geometry-core point_cloud_two_closest_points",
            "cargo test -p zennah-geometry-core point_cloud_neighbors_in_radius",
            "cargo test -p zennah-geometry-core select_point_cloud_points_by_screen",
            "cargo test -p zennah-geometry-core point_cloud_pick_by_ray",
            "cargo test -p zennah-geometry-core point_cloud_extract_selected_points_as_object",
            "cargo test -p zennah-geometry-core point_cloud_local_neighbor_fan",
            "cargo test -p zennah-geometry-core point_cloud_local_fan_triangles",
            "cargo test -p zennah-geometry-core point_cloud_local_triangulation_repetitions",
            "cargo test -p zennah-geometry-core point_cloud_triangulate_candidate_mesh",
            "cargo test -p zennah-geometry-core point_cloud_triangulate_cleaned_candidate_mesh",
            "cargo test -p zennah-geometry-core point_cloud_triangulate_topology_candidate_mesh",
            "cargo test -p zennah-geometry-core point_cloud_triangulate_filled_candidate_mesh",
            "cargo test -p zennah-geometry-core point_cloud_uniform_sampling",
            "cargo test -p zennah-geometry-core point_cloud_grid_sampling",
            "cargo test -p zennah-geometry-core pairwise_point_to_point_icp",
            "cargo test -p zennah-geometry-core pairwise_point_to_plane_icp",
            "cargo test -p zennah-geometry-core multiway_point_to_point_icp",
            "cargo test -p zennah-geometry-core multiway_point_to_plane_icp",
            "cargo test -p zennah-geometry-core multiway_combined_icp",
            "cargo test -p zennah-geometry-core multiway_all_object",
            "cargo test -p zennah-geometry-core multiway_sequential_cascade",
            "cargo test -p zennah-geometry-core multiway_aabb_cascade",
            "cargo test -p zennah-geometry-core meshlib_stitch_fill_metric_modes_are_selectable_rust_modes",
            "cargo test -p zennah-geometry-core vertical_stitch_metric_uses_meshlib_caller_supplied_up_dir",
            "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py::test_point_cloud_screen_selectors_match_meshlib_viewport_area_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py::test_point_cloud_pick_by_ray_matches_meshlib_frontmost_point_pick_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py::test_point_cloud_extract_selected_points_as_object_matches_meshlib_clone_region_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_point_cloud_triangulation_endpoint_returns_meshlib_style_rust_payload -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_point_cloud_multiway_icp_endpoint_returns_meshlib_style_rust_payload -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_can_create_meshlib_point_cloud_selection_to_object_version -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_service_fill_holes_accepts_meshlib_stitch_metric_modes -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_service_fill_holes_exposes_meshlib_vertical_stitch_up_dir_param -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py -q",
        ],
        "notes": [
            "point_cloud_nearest_projections, point_cloud_project_to_mesh, point_cloud_n_closest_neighbors, point_cloud_two_closest_points, point_cloud_neighbors_in_radius, point_cloud_select_by_screen_polygon, point_cloud_select_by_screen_rect, point_cloud_select_by_screen_brush, point_cloud_pick_by_ray, point_cloud_extract_selected_points_as_object, point_cloud_local_neighbor_fan, point_cloud_local_fan_triangles, point_cloud_local_triangulation_repetitions, point_cloud_triangulate_candidate_mesh, point_cloud_triangulate_cleaned_candidate_mesh, point_cloud_triangulate_topology_candidate_mesh, point_cloud_triangulate_filled_candidate_mesh, point_cloud_uniform_sample, point_cloud_grid_sample, pairwise_point_to_point_icp, pairwise_point_to_plane_icp, multiway_point_to_point_icp, multiway_point_to_plane_icp, multiway_combined_icp, multiway_all_object_point_to_point_icp, multiway_all_object_point_to_plane_icp, multiway_all_object_combined_icp, multiway_sequential_cascade_point_to_point_icp, multiway_sequential_cascade_point_to_plane_icp, multiway_sequential_cascade_combined_icp, multiway_aabb_cascade_point_to_point_icp, multiway_aabb_cascade_point_to_plane_icp, and multiway_aabb_cascade_combined_icp are Rust-backed for MeshLib-style point-cloud projection, single-mesh projection point/distance/face/closest-vertex/normal/boundary payloads with rigid object/reference transforms, face-region masks, and face/edge/vertex pseudonormal normals, MRSelectScreenLasso::findVertsInViewportArea-style point primitive screen selection, MeshLib pickRenderObject/ObjectPointsHolder-style primitive Pick selection, MeshLib ObjectPoints::cloneRegion/PointCloud::addPartByMask-style selected-point extraction, point-cloud Selection to Object child-version creation with normalized_point_cloud_ply artifacts, closest-neighbor primitives, radius-neighbor normal filtering, local fan ordering/boundary detection, fan-triangle emission, local-triangulation repetition accounting, repeated-triangle candidate mesh assembly, two-phase topology edge filtering with MeshBuilder-style half-edge origin-ring insertion guards, MeshLib-style hole-complicating bad-triangle removal, MeshLib-thresholded small-hole fill composition with MultipleEdgesResolveMode None/Simple/Strong dispatch, Simple-mode duplicate-edge avoidance, Strong-mode reused generated chord repair, outNewFaces new-face index reporting, maxPolygonSubdivisions split sampling, makeDegenerateBand duplicate-boundary band creation, stopBeforeBadTriangulation bad-patch guarding, smoothBd boundary-edge metric control, getMinAreaMetric double-area triangulation, getEdgeLengthFillMetric edge-length triangulation, getUniversalMetric universal smooth triangulation, getMaxDihedralAngleMetric max-dihedral-angle triangulation, getParallelPlaneFillMetric parallel-plane projection triangulation, getComplexFillMetric aspect-area edge-penalty triangulation, getMinTriAngleMetric minimum-angle triangulation, getPlaneFillMetric plane-normal triangulation, getPlaneNormalizedFillMetric plane-normalized aspect triangulation, getComplexStitchMetric aspect-ratio/dihedral stitch triangulation, getEdgeLengthStitchMetric edge-length stitch triangulation, getVerticalStitchMetric caller-supplied upDir vertical stitch triangulation, and getVerticalStitchMetricEdgeBased caller-supplied upDir vertical edge-projection stitch triangulation, max-removes optimization, uniform/grid sampling, rigid or translation-only pairwise alignment, point-to-plane distance, normal-cosine, reciprocal closest pair filtering, MeshLib maxGroupSize=1-style independent multiway point-to-point/point-to-plane/combined ICP, MeshLib maxGroupSize=0-style all-object multiway point-to-point/point-to-plane/combined ICP, MeshLib maxGroupSize>1 sequential cascade multiway point-to-point/point-to-plane/combined ICP, and MeshLib AABBTreeBased cascade multiway point-to-point/point-to-plane/combined ICP.",
            "official full MeshLib mesh-topology materialization, arbitrary callback FillHoleMetric parameterization, and non-rigid tree-accelerated/multi-object mesh projection workflows remain open.",
        ],
    },
    {
        "official_feature_id": "voxels-ct-sdf",
        "label": "Voxel volumes, CT reconstruction, SDF, and marching extraction",
        "group": "voxels",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshinspector.com/knowledge-base/mesh-editing/mesh-to-voxels/",
            "https://meshinspector.com/knowledge-base/voxel-ct/voxels-segmentation/",
            "https://meshinspector.com/3d-viewers/dicom-viewer/",
            "https://meshinspector.com/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRCommonPlugins/Voxels/MROpenRawVoxelsPlugin.*",
            "MeshLib/source/MRCommonPlugins/Voxels/MROpenVoxelsFromTiffPlugin.*",
            "MeshLib/source/MRVoxels/MRVoxelsLoad.*",
            "MeshLib/source/MRVoxels/MRObjectVoxels.*",
            "MeshLib/source/MRVoxels/MRVoxelsVolumeAccess.*",
            "MeshLib/source/MRMesh/MRHistogram.*",
            "MeshLib/source/MRVoxels/MRScalarConvert.*",
            "MeshLib/source/MRMesh/MRTiffIO.*",
            "MeshLib/source/MRCommonPlugins/Voxels/MRBinaryOperationsPlugin.*",
            "MeshLib/source/MRViewer/MRMarkedVoxelSlice.*",
            "MeshLib/source/MRVoxels/MRVoxelsSave.*",
            "MeshLib/source/MRVoxels/MRVoxelPath.*",
            "MeshLib/source/MRVoxels/MRVoxelGraphCut.*",
            "MeshLib/source/MRVoxels/MRVolumeSegment.*",
            "MeshLib/source/MRVoxels/MRVDBConversions.*",
            "MeshLib/source/MRViewer/MRRenderVolumeObject.*",
            "MeshLib/source/MRViewer/MRVolumeShader.*",
            "MeshLib/source/MRVoxels/MRMarchingCubes.*",
            "MeshLib/source/MRVoxels/MRMoveMeshToVoxelMaxDeriv.*",
            "MeshLib/source/MRVoxels/MROffset.*",
            "MeshLib/source/MRMesh/MRDistanceMap.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/spatial.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_active_box.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_line_graph.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_path.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_raw.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_rendering.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_slice.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_segmentation.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_tiff.rs",
            "geometry-rs/crates/zennah-geometry-core/src/voxel_mesh_ops.rs",
            "geometry-rs/crates/zennah-geometry-py/src/spatial.rs",
            "geometry-rs/crates/zennah-geometry-py/src/voxel.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/voxel/sdf.py",
            "geometry_sdk/voxel/marching.py",
            "geometry_sdk/voxel/ops.py",
            "geometry_sdk/voxel/active_box.py",
            "geometry_sdk/voxel/conversion.py",
            "geometry_sdk/voxel/line_graph.py",
            "geometry_sdk/voxel/path.py",
            "geometry_sdk/voxel/raw.py",
            "geometry_sdk/voxel/rendering.py",
            "geometry_sdk/voxel/segmentation.py",
            "geometry_sdk/voxel/slice.py",
        ],
        "backend_command_ids": ["mesh-to-voxels-sdf", "voxel-binary-operations", "open-raw-voxels", "open-voxels-from-tiff", "voxel-slice", "voxel-line-graph", "voxel-active-box", "voxel-volume-render-data", "voxel-volume-render-lut", "voxel-volume-render-ray", "voxel-segmentation", "voxel-mask-to-mesh", "voxel-to-mesh-simple", "voxel-to-mesh-dual", "voxel-to-mesh-smart", "voxel-path", "voxel-path-build-four"],
        "validation_gates": [
            "cargo test -p zennah-geometry-core raw_voxels",
            "cargo test -p zennah-geometry-core tiff_voxels",
            "cargo test -p zennah-geometry-core voxel_binary",
            "cargo test -p zennah-geometry-core voxel_path",
            "cargo test -p zennah-geometry-core voxel_segmentation",
            "cargo test -p zennah-geometry-core sdf",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_load_raw_voxels_matches_meshlib_uint16_normalization_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_load_tiff_voxels_dir_matches_meshlib_sorted_slice_stack_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_default_iso_value_matches_meshlib_object_voxels_histogram_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_open_raw_voxels_capability_exposes_meshlib_common_plugin_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_open_voxels_from_tiff_capability_exposes_meshlib_common_plugin_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_slice_matches_meshlib_save_slice_texture_order_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_slice_capability_exposes_meshlib_ct_tool_command -q",
            "cargo test -p zennah-geometry-core voxel_line_graph",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_line_graph_matches_meshinspector_axis_probe_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_line_graph_capability_exposes_meshinspector_ct_tool_command -q",
            "cargo test -p zennah-geometry-core voxel_active_box",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_active_box_matches_meshlib_max_excluded_bounds_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_active_box_capability_exposes_meshinspector_ct_tool_command -q",
            "cargo test -p zennah-geometry-core voxel_volume_render_data",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_volume_render_data_matches_meshlib_normalized_active_box_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_volume_render_data_capability_exposes_meshinspector_ct_tool_command -q",
            "cargo test -p zennah-geometry-core voxel_volume_render_lut",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_volume_render_lut_matches_meshlib_dense_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_volume_render_lut_capability_exposes_meshinspector_ct_tool_command -q",
            "cargo test -p zennah-geometry-core voxel_volume_render_ray",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_volume_render_ray_matches_meshlib_fixed_step_compositing_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_volume_render_ray_matches_meshlib_alpha_gradient_no_zero_normal_discard_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_volume_render_ray_matches_meshlib_shade_color_lighting_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_volume_render_ray_endpoint_returns_rust_meshlib_shader_payload -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_volume_render_ray_capability_exposes_meshinspector_ct_tool_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_segmentation_matches_meshlib_graph_cut_and_boundary_seed_contracts -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_segmentation_mesh_matches_meshlib_simple_mask_iso_shift_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_segmentation_capability_exposes_meshinspector_ct_tool_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_mask_to_mesh_matches_meshlib_smooth_mask_meshing_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_mask_to_mesh_capability_exposes_meshinspector_ct_tool_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_simple_matches_meshlib_dense_volume_iso_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_simple_capability_exposes_meshinspector_ct_tool_command -q",
            "cargo test -p zennah-geometry-core voxel_to_mesh_dual",
            "cargo test -p zennah-geometry-core voxel_to_mesh_dual_values_with_settings_enforces_meshlib_limits",
            "cargo test -p zennah-geometry-core voxel_to_mesh_dual_values_with_settings_applies_meshlib_planar_adaptivity",
            "cargo test -p zennah-geometry-core relax_disoriented_mesh_triangles_flips_meshlib_ray_invalid_faces",
            "cargo test -p zennah-geometry-core meshlib_vdb_payload_to_dual_mesh",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_extracts_meshlib_dense_dual_plane_slice -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_exposes_meshlib_face_and_vertex_limits -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_exposes_meshlib_adaptivity_setting -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_exposes_meshlib_relax_disoriented_triangles_setting -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_meshes_openvdb_dense_leaf_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_preserves_openvdb_active_bbox_origin_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_accepts_distinct_openvdb_topology_and_buffer_masks_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_pads_tight_openvdb_active_bbox_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_pads_sparse_openvdb_active_window_boundary_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_pads_full_leaf_span_sparse_openvdb_mask_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_returns_rust_meshlib_mesh_payload -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_enforces_meshlib_limits_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_exposes_meshlib_adaptivity_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_exposes_meshlib_relax_disoriented_triangles_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_accepts_openvdb_payload_through_rust -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_enforces_openvdb_payload_limits_through_rust -q",
            "cargo test -p zennah-geometry-core voxel_move_mesh_to_max_deriv",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_move_mesh_to_max_deriv_matches_meshlib_cubic_shift_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_smart_capability_exposes_meshinspector_ct_tool_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_binary_operations_match_meshlib_binary_operations_plugin_scalar_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_binary_operations_capability_exposes_meshlib_common_plugin_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_path_matches_meshlib_difference_and_exponent_metric_contracts -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_path_capability_exposes_meshlib_ct_tool_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_path_build_four_matches_meshlib_quarter_seed_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_path_build_four_capability_exposes_meshlib_ct_tool_command -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_mesh_to_voxels_sdf_endpoint_returns_meshlib_style_rust_payload -q",
        ],
        "notes": [
            "mesh-to-voxels-sdf is exposed as a Rust-backed MeshLib meshToLevelSet-style signed closed-mesh conversion and meshToDistanceField-style unsigned conversion with voxel-size, positive surface-offset, occupancy, volume, and optional iso-surface extraction.",
            "voxel-binary-operations exposes MeshLib CommonPlugins BinaryOperations behavior for level-set Union/Intersection/Difference and scalar-grid Max/Min/Sum/Multiply/Divide operations, including MeshLib result iso-value rules.",
            "open-raw-voxels exposes MeshLib CommonPlugins Open RAW Voxels explicit-parameter and filename-auto VoxelsLoad::fromRaw behavior, including integer ScalarType normalization, current MeshLib Float64/Float32_4 zero-range conversion semantics, and ObjectVoxels histogram one-third-bin default iso-value selection.",
            "open-voxels-from-tiff exposes MeshLib CommonPlugins Open Voxels From TIFF directory loading, including TIFF filtering, scan-name numeric sorting, per-slice parameter consistency, scalar/RGB/RGBA float conversion, voxel size, DenseGrid/LevelSet selection, and ObjectVoxels histogram one-third-bin default iso-value selection.",
            "voxel-slice exposes MeshLib MRMarkedVoxelSlice and MRVoxelsSave::saveSliceToImage-style YZ/ZX/XY slice extraction, texture dimensions, coordinate order, and min/max normalization.",
            "voxel-line-graph exposes the official MeshInspector Voxels Line Graph CT tool as MeshLib x-fastest axis-probe sampling over ObjectVoxels dense values.",
            "voxel-active-box exposes MeshLib ObjectVoxels::setActiveBounds max-excluded active-box semantics and the official Create New Object crop payload.",
            "voxel-volume-render-data exposes MeshLib ObjectVoxels::prepareDataForVolumeRendering / vdbVolumeToSimpleVolumeNorm-style active-box prepared data with source-scale normalization to [0, 1].",
            "voxel-volume-render-lut exposes MeshLib RenderVolumeObject::bindVolume_ denseMap behavior for ObjectVoxels::VolumeRenderingParams::LutType and VolumeRenderingParams::AlphaType transfer-function color/alpha bytes.",
            "voxel-volume-render-ray exposes MRVolumeShader samplingStep > 0 fixed-step traversal and step <= 0 rayVoxelIntersection voxel-boundary traversal with clipping-plane discard, active-mask filtering, density gating, denseMap lookup, front-to-back alpha compositing, shadingMode == 1 value-gradient zero-normal sample rejection, shadingMode == 2 alpha-gradient normal sampling, and optional MeshLib shadeColor lighting modulation.",
            "voxel-segmentation exposes the official MeshInspector Voxels Segmentation CT tool as Rust-backed MeshLib MRVoxelGraphCut/MRVolumeSegment-style seed-based volume segmentation with inside/outside seeds, crop expansion, boundary outside seeds, directed density edge capacities, and createMeshFromSegmentation-style simple mask meshing at iso-value 0.5 with minVoxel*voxelSize mesh shift.",
            "voxel-mask-to-mesh exposes MeshLib MRVolumeSegment::meshFromVoxelsMask-style smooth mask meshing with whole-volume mask crop expansion, prepareVolumePart VolumeMaskMeshingMode::Smooth density averaging, expand/shrink smoothing bands, iso-value 0.5, and minVoxel*voxelSize mesh shift.",
            "voxel-to-mesh-simple exposes the official MeshInspector Voxels to Mesh Simple Conversion contract around MeshLib ObjectVoxels::recalculateIsoSurface: iso-value extraction, anisotropic voxelSize scaling, lessInside=false high-density interior convention for dense volumes, lessInside=true signed-distance convention for LevelSet grids, and MeshLib x-fastest voxel indexing.",
            "voxel-to-mesh-dual exposes a Rust-backed dense dual-contouring slice of MeshLib ObjectVoxels::recalculateIsoSurface / openvdb::tools::VolumeToMesh with MeshLib x-fastest dense values, anisotropic voxelSize scaling, dense high-value interiors, LevelSet lessInside semantics, MeshLib maxVertices/maxFaces limit errors, dense planar adaptivity coalescing, direct .vdb FloatGrid dense-payload decoding with OpenVDB active bbox origin preservation, distinct OpenVDB topology and value-buffer masks, tight sparse active-bbox, active-window boundary, and full-leaf-span sparse active-mask background halo padding, and MeshLib relaxDisorientedTriangles-style closed-surface ray-count face relaxation; exact sparse OpenVDB VolumeToMesh topology and curved/sparse adaptivity remain open.",
            "voxel-to-mesh-smart exposes the official MeshInspector Smart Conversion refinement via MeshLib MR::moveMeshToVoxelMaxDeriv: samplePoints=6 default, degree=3 default with MeshLib degree=3..6 support, polynomial density fitting along vertex normals, derivative-minimum shift, outlier threshold, clamped 0.1 voxel shift, intermediate smoothing, and final relax.",
            "voxel-path exposes MeshLib MRVoxelPath buildSmallestMetricPath behavior for CT voxel-path inspection, including Difference and Exponent path metrics.",
            "voxel-path-build-four exposes the official Voxels Path Build four mode by running MRVolumeSegment-style QuarterBit masks 1, 2, 4, and 8 as separate Rust-backed paths.",
            "Broader official voxel-object management, CT reconstruction workflow, exact sparse Dual Marching Cubes/OpenVDB VolumeToMesh topology, curved/sparse OpenVDB VolumeToMesh adaptivity, and full interactive GL ray-march volume-rendering viewport controls remain open.",
        ],
    },
    {
        "official_feature_id": "distance-maps-lines-gcode",
        "label": "Distance maps, lines, contours, iso-lines, and G-code data types",
        "group": "distance_map",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/feature/",
            "https://meshlib.io/documentation/namespaceMR.html",
            "https://meshinspector.com/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRMesh/MRDistanceMap.*",
            "MeshLib/source/MRMesh/MRExtractIsolines.*",
            "MeshLib/source/MRMesh/MROffsetContours.*",
            "MeshLib/source/MRMesh/MRObjectLines.*",
            "MeshLib/source/MRMesh/MRObjectLinesHolder.*",
            "MeshLib/source/MRMesh/MRPolyline.*",
            "MeshLib/source/MRMesh/MRPolylineTopology.*",
            "MeshLib/source/MRMesh/MRLinesLoad.*",
            "MeshLib/source/MRMesh/MRLinesSave.*",
            "MeshLib/source/MRMesh/MRGcodeLoad.*",
            "MeshLib/source/MRMesh/MRGcodeProcessor.*",
            "MeshLib/source/MRMesh/MRCNCMachineSettings.*",
            "MeshLib/source/MRMesh/MRObjectGcode.*",
            "MeshLib/source/MRIOExtras/MRSvg.*",
            "MeshLib/source/MRIOExtras/",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-core/src/distance.rs",
            "geometry-rs/crates/zennah-geometry-core/src/distance_tiff.rs",
            "geometry-rs/crates/zennah-geometry-core/src/lines.rs",
            "geometry-rs/crates/zennah-geometry-core/src/lines/offset_contours.rs",
            "geometry-rs/crates/zennah-geometry-core/src/lines/svg.rs",
            "geometry-rs/crates/zennah-geometry-core/src/gcode.rs",
            "geometry-rs/crates/zennah-geometry-py/src/distance.rs",
            "geometry-rs/crates/zennah-geometry-py/src/lines.rs",
            "geometry-rs/crates/zennah-geometry-py/src/gcode.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/distance_map/contours.py",
            "geometry_sdk/distance_map/lines.py",
            "geometry_sdk/gcode/paths.py",
        ],
        "backend_command_ids": ["distance-map-from-mesh", "distance-map-contours", "object-lines-from-contours", "object-lines-to-contours", "offset-contours", "object-lines-load-mrlines", "object-lines-save-mrlines", "object-lines-load-ply", "object-lines-save-ply", "object-lines-load-pts", "object-lines-load-svg", "object-lines-save-pts", "object-lines-save-dxf", "distance-map-iso-lines", "distance-map-merge", "distance-map-contour-boolean", "distance-map-from-tiff", "distance-map-to-tiff", "gcode-parse-paths", "gcode-load-source", "gcode-write-source", "gcode-parse-file-paths"],
        "validation_gates": [
            "cargo test -p zennah-geometry-core distance_map_from_mesh",
            "cargo test -p zennah-geometry-core distance_map_from_contours",
            "cargo test -p zennah-geometry-core distance_map_to_iso_segments",
            "cargo test -p zennah-geometry-core distance_map_merge",
            "cargo test -p zennah-geometry-core distance_map_contour_boolean",
            "cargo test -p zennah-geometry-core distance_map_from_tiff",
            "cargo test -p zennah-geometry-core distance_map_to_tiff",
            "cargo test -p zennah-geometry-core object_lines",
            "cargo test -p zennah-geometry-core offset_contours",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_default_3d_z_restore_relaxation_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_variable_shell_3d_z_restore_relaxation_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_variable_negative_offset_3d_z_restore_relaxation_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_variable_sharp_max_angle_3d_z_restore_relaxation_contract -q",
            "cargo test -p zennah-geometry-core tests::offset_contours_matches_meshlib_closed_variable_mixed_signed_offset_contract -- --exact --nocapture --test-threads=1",
            "cargo test -p zennah-geometry-core tests::offset_contours_exposes_meshlib_mixed_signed_variable_index_map_contract -- --exact --nocapture --test-threads=1",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_closed_variable_mixed_signed_offset_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_mixed_signed_variable_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_restore_z_relax_iterations",
            "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_constant_z_callback_mode",
            "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_custom_z_callback_mode",
            "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_callable_z_callback_context",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_restore_z_relax_iterations -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_constant_z_callback_mode -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_custom_z_callback_mode -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_callable_z_callback_context -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_positive_round_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_negative_intersection_index_map_contract -q",
            "cargo test -p zennah-geometry-core tests::offset_contours_exposes_meshlib_zero_offset_identity_index_map_contract -- --exact --nocapture --test-threads=1",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_zero_offset_identity_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_positive_variable_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_negative_variable_intersection_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_fixed_shell_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_variable_shell_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_round_end_index_map_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_round_end_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_open_variable_zig_round_end_index_map_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_variable_zig_round_end_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_index_map_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_index_map_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_perpendicular_segments_global_outline_index_map_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_perpendicular_segments_global_outline_index_map_contract -q",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_touching_horizontal_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_touching_horizontal_direction_variants_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_vertical_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_vertical_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_diagonal_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_diagonal_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract",
            "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_touching_horizontal_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_horizontal_direction_variants_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_vertical_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_vertical_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_diagonal_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_diagonal_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract -q",
            "cargo test -p zennah-geometry-core object_lines_mrlines",
            "cargo test -p zennah-geometry-core object_lines_ply",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_format_version_tuple -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_format_minor_prefix_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_format_minor_alpha_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_format_minor_underscore_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_magic -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_format_line_tokens -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_element_line_tokens -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_element_count_alpha_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_element_count_underscore_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_property_line_tokens -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_leading_header_keyword_whitespace_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_spaced_format_version_tuple -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_end_header -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_unknown_header_directives_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_casts_coordinates_to_vector3f_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_wraps_narrow_vertex_coordinates_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_casts_float_edge_indices_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_wraps_narrow_edge_indices_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_unneeded_vertex_property -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_skipped_element -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_last_integer_prefix_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_skips_meshlib_unsigned_negative_edge_endpoint -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ply_import_prefers_meshlib_rgb_short_names_over_long_color_names -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_casts_float_vertex_colors_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_wraps_integer_vertex_colors_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_ignores_unneeded_list_properties_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_property_name_prefix_suffix -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_non_identifier_property_names_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_float64_type_alias_like_meshlib -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_ignores_invalid_edges_like_meshlib -q",
            "cargo test -p zennah-geometry-core object_lines_ascii_ply_import_skips_edge_elements_without_meshlib_vertex_properties",
            "cargo test -p zennah-geometry-core object_lines_binary_ply_import_skips_edge_elements_without_meshlib_vertex_properties",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_skips_edge_elements_without_meshlib_vertex_properties -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_binary_ply_import_skips_edge_elements_without_meshlib_vertex_properties -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_vertex_only_files_like_meshlib -q",
            "cargo test -p zennah-geometry-core object_lines_ascii_ply_import_trims_meshlib_texturefile_comment_trailing_spaces",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_preserves_meshlib_uv_and_texture_comment -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_trims_meshlib_texturefile_comment_trailing_spaces -q",
            "cargo test -p zennah-geometry-core object_lines_pts",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_pts_import_accepts_meshlib_trailing_point_fields -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_pts_import_accepts_meshlib_last_coordinate_prefix_suffix -q",
            "cargo test -p zennah-geometry-core object_lines_dxf",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_line_polyline_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_line_and_polyline_y_flip -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_accepts_meshlib_compact_signed_points_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_polygon_rect_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_polygon_and_rect_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_circle_ellipse_sampling_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_circle_and_ellipse_sampling_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_rounded_rect_sampling_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_rounded_rect_sampling_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_linear_path_commands_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_linear_path_commands_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_curve_path_commands_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_curve_path_commands_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_arc_path_commands_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_arc_path_commands_y_flip -q",
            "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_transform_attributes_y_flip",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_transform_attributes_y_flip -q",
            "cargo test -p zennah-geometry-core gcode_linear_paths_match_meshlib_processor_modal_motion",
            "cargo test -p zennah-geometry-core gcode_command_values_match_meshlib_strtof_narrowing",
            "cargo test -p zennah-geometry-core gcode_command_values_accept_meshlib_strtof_special_float_tokens",
            "cargo test -p zennah-geometry-core gcode_command_values_accept_meshlib_strtof_hex_float_tokens",
            "cargo test -p zennah-geometry-core gcode_command_values_accept_meshlib_strtof_leading_whitespace",
            "cargo test -p zennah-geometry-core gcode_arc_radius_mismatch_warning_matches_meshlib_to_string_float_format",
            "cargo test -p zennah-geometry-core gcode_radius_only_arc_matches_meshlib_no_motion_feedrate_contract",
            "cargo test -p zennah-geometry-core gcode_feedrate_only_frame_updates_meshlib_feedrate_max_without_segments",
            "cargo test -p zennah-geometry-core gcode_zero_idle_feedrate_is_rewritten_to_meshlib_final_feedrate_max",
            "cargo test -p zennah-geometry-core gcode_arc_paths_match_meshlib_center_offset_sampling",
            "cargo test -p zennah-geometry-core gcode_arc_paths_match_meshlib_g18_g19_work_plane_mapping",
            "cargo test -p zennah-geometry-core gcode_scaling_matches_meshlib_g51_g50_contract",
            "cargo test -p zennah-geometry-core gcode_rotary_axis_matches_meshlib_default_c_axis_sampling",
            "cargo test -p zennah-geometry-core gcode_tool_directions_match_meshlib_default_rotated_plus_z",
            "cargo test -p zennah-geometry-core gcode_custom_cnc_home_and_idle_feedrate_match_meshlib_settings",
            "cargo test -p zennah-geometry-core gcode_g28_at_home_emits_meshlib_zero_length_idle_action",
            "cargo test -p zennah-geometry-core gcode_custom_cnc_rotation_axes_and_order_match_meshlib_settings",
            "cargo test -p zennah-geometry-core gcode_custom_cnc_rotation_limits_match_meshlib_warning_contract",
            "cargo test -p zennah-geometry-core gcode_custom_cnc_rotation_limits_are_clamped_like_meshlib_settings",
            "cargo test -p zennah-geometry-core gcode_source_file",
            "cargo test -p zennah-geometry-core gcode_source_file_preserves_meshlib_crlf_frame_carriage_returns",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_command_narrowing -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_special_float_tokens -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_hex_float_tokens -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_leading_whitespace -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_arc_radius_mismatch_warning_format -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_radius_only_arc_no_motion_feedrate_contract tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_feedrate_only_frame_without_segments -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_zero_idle_feedrate_post_pass -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_g28_at_home_zero_length_idle_action -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_gcode_machine_settings_exports_meshlib_cnc_json_contract tests/test_geometry_sdk_gcode.py::test_gcode_machine_settings_imports_meshlib_cnc_json_contract -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_gcode_source_file_preserves_meshlib_crlf_frame_carriage_returns -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py -q",
            "uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py -q",
            "Broaden PLY line fixture coverage beyond MeshLib-style magic-line whitespace, format-version whitespace, minor punctuation-suffix tolerance and alpha-suffix rejection, format-line, element-line, and property-line trailing-token tolerance plus element-count alpha or underscore suffix rejection and property-name prefix suffix tolerance, end_header trailing-whitespace handling, strict header directive, leading keyword whitespace, and identifier plus scalar type alias validation, Vector3f coordinate narrowing and source scalar conversion, scalar-to-int edge endpoint conversion plus ASCII row integer-prefix suffix, narrow integer wrapping, and unsigned scalar sign-cast handling, invalid edge skipping, edge elements without vertex1/vertex2 skipping, r/g/b color-name precedence, scalar color conversion including integer wrapping, unneeded list-property skipping, MeshLib-style binary list-count scalar conversion, vertex-only point payloads, and per-vertex UV/TextureFile metadata import into texture image loading/rendering/export, tri-corner UV, and broader third-party variants.",
        ],
        "notes": [
            "distance_map_from_mesh is Rust-backed for MeshLib-style orthographic pixel-center ray distance maps from triangle meshes.",
            "distance_map_from_contours is Rust-backed for MeshLib-style pixel-center contour distance maps with signed inside values.",
            "object_lines_from_contours and object_lines_to_contours are Rust-backed for MeshLib ObjectLinesHolder Polyline.Points/Polyline.Lines scene-object persistence and open/closed contour restoration.",
            "offset_contours is Rust-backed for the MeshLib MROffsetContours closed clockwise signed Type::Offset round-corner fixed-offset slices, positive CornerType::Sharp fixed-offset slices with maxSharpAngle limiting, MeshLib default 3D signed fixed/variable Type::Offset, sharp max-angle, fixed/variable shell Z restore/one-pass default relaxation, explicit relaxIterations, constant/custom source-Z restore plus callable zCallback output/index/origin context, positive closed fixed/variable non-intersection, closed fixed zero-offset identity indicesMap/origin output, plus negative and shell-inner closed fixed/variable intersection indicesMap/origin output, closed clockwise signed variable-offset Type::Offset round/sharp-corner slices with maxSharpAngle limiting, positive fixed/variable including unequal-variable and mixed-signed Type::Offset final-outline self-overlap remap with indicesMap intersections, signed variable-offset Type::Shell round/sharp-corner slices with maxSharpAngle limiting including empty negative-shell output, closed signed fixed-offset Type::Shell slices, open EndType::Round/Cut fixed-offset slices, open fixed bent/zig and variable bent/zig round-end indicesMap/origin output, open fixed cut-end connected collinear seam-preserving axis/non-axis plus axis/non-axis shifted parallel global-outline composition, axis-aligned perpendicular crossing, horizontal/vertical/non-axis touching-chain including horizontal direction variants, direction-reversed vertical and diagonal origin maps, and first-direction-reversed vertical/diagonal outline ordering, axis/non-axis overlapping-parallel, and axis/non-axis collinear-overlap plus direction-reversed horizontal collinear-overlap including first-source and both-reversed ordering, vertical direction variants, diagonal direction variants, and three-segment horizontal/vertical/diagonal collinear-overlap chains including diagonal chain direction variants global-outline indicesMap/origin output, and open variable-offset EndType::Cut slices; broader intersection index maps remain future parity items.",
            "object_lines_from_mrlines and object_lines_to_mrlines are Rust-backed for MeshLib binary .mrlines PolylineTopology and Vector3f point payloads.",
            "object_lines_from_ply and object_lines_to_ply are Rust-backed for ASCII PLY and binary little-/big-endian PLY line vertex/edge/color payloads, including MeshLib-style magic-line whitespace, format-version whitespace, minor punctuation-suffix tolerance and alpha-suffix rejection, format-line, element-line, and property-line trailing-token tolerance plus element-count alpha or underscore suffix rejection and property-name prefix suffix tolerance, end_header trailing-whitespace handling, strict header directive, leading keyword whitespace, and identifier validation, strict scalar type alias validation, MeshLib-style Vector3f coordinate narrowing and source scalar conversion, MeshLib-style scalar-to-int edge endpoint conversion plus ASCII row integer-prefix suffix, narrow integer wrapping, and unsigned scalar sign-cast handling, PolylineTopology-style invalid edge skipping, MeshLib-style edge elements without vertex1/vertex2 skipping, ASCII mesh face/list elements skipped before edge extraction, MeshLib-style r/g/b short-name color precedence over red/green/blue, MeshLib-style scalar-to-uchar vertex color conversion with integer wrapping, unneeded list-property skipping in vertex/edge elements, MeshLib-style binary list-count scalar conversion, MeshLib-style vertex-only point payloads with empty topology, and MeshLib per-vertex PLY UV import aliases (u/v, s/t, texture_u/texture_v, texture_s/texture_t) plus TextureFile comment metadata with miniply-style leading/trailing comment whitespace trimming; export remains MeshLib-style binary little-endian.",
            "object_lines_from_pts, object_lines_to_pts, object_lines_to_dxf, and object_lines_from_svg are Rust-backed for MeshLib PTS line block import/export with trailing point-field tolerance and last-coordinate numeric-prefix suffix tolerance, DXF POLYLINE export, and MRIOExtras LinesLoad::fromSvg line/polyline/polygon/circle/ellipse/simple-rounded-rect/path-command/transform import with compact signed polyline/polygon points and MeshLib's post-parse Y-axis flip.",
            "distance_map_to_iso_segments is Rust-backed for MeshLib-style distance-map iso-line extraction.",
            "distance_map_merge is Rust-backed for MeshLib-style min, max, and subtraction distance-map composition.",
            "distance_map_contour_boolean is Rust-backed for MeshLib-style contour union, intersection, and subtraction.",
            "distance_map_from_tiff is Rust-backed for MeshLib-style TIFF distance-map import with scalar/RGB/RGBA sample conversion into float grids.",
            "distance_map_to_tiff is Rust-backed for MeshLib-style TIFF distance-map export with float scalar samples, WhiteIsZero photometric metadata, and GDAL NoData sentinel metadata.",
            "parse_gcode_paths is Rust-backed for MeshLib-style source-frame parsing, comments, strtof command-value narrowing including leading command-value whitespace, special, and hexadecimal float tokens, no-motion feedrateMax updates, zero-idle feedrate post-pass rewriting, radius-only G2/G3 no-op handling, G28 home zero-length idle actions, MeshLib-style arc radius-mismatch warning formatting, modal G0/G1 moves, G17/G18/G19 G2/G3 center/radius arcs, G50/G51 scaling, default/custom CNC home/feedrate/axis/order/limit settings, rotary-axis sampled toolpaths, tool-direction export, absolute/relative coordinates, inch/mm units, feedrates, and rotation-limit warnings.",
            "GcodeMachineSettings MeshLib JSON import/export is Rust-backed for CNCMachineSettings::saveToJson/loadFromJson-style Axes Order, Axis A/B/C Direction and Limits, Feedrate Idle, Home Position, MRSerializer vector objects, decimal/hex-float/numeric-prefix whitespace-string vector fallback with MeshLib stream-default partial assignment, duplicate-axis rejection, axis normalization, limit clamping, and inactive-axis omission.",
            "load_gcode_source, write_gcode_source, and parse_gcode_file_paths are Rust-backed for MeshLib-style .gcode/.nc/.txt source file workflows, including non-empty CRLF frame carriage-return preservation from GcodeLoad::fromGcode.",
            "External .mrlines, ASCII and binary little-/big-endian .ply vertex-edge-color, UV/TextureFile metadata, vertex-only line files, .pts, .dxf, and SVG line/polyline/polygon/circle/ellipse/simple-rounded-rect/path-command/transform workflows are Rust-backed with MeshLib-style magic-line whitespace, format-version whitespace, minor punctuation-suffix tolerance and alpha-suffix rejection, format-line, element-line, and property-line trailing-token tolerance plus element-count alpha or underscore suffix rejection and property-name prefix suffix tolerance, end_header trailing-whitespace handling, strict header directive, leading keyword whitespace, and identifier plus scalar type alias validation, Vector3f coordinate narrowing and source scalar conversion, scalar-to-int edge endpoint conversion plus ASCII row integer-prefix suffix, narrow integer wrapping, and unsigned scalar sign-cast handling, invalid edge skipping, edge elements without vertex1/vertex2 skipping, r/g/b short-name color precedence over red/green/blue, scalar-to-uchar color conversion including integer wrapping, unneeded list-property skipping, and MeshLib-style binary list-count scalar conversion; ObjectLines texture image loading/rendering/export, tri-corner UV, and broader third-party variants remain future hardening.",
        ],
    },
    {
        "official_feature_id": "automation-plugin-api",
        "label": "MeshLib/MeshInspector plugin and automation integration",
        "group": "automation",
        "status": "partial",
        "official_sources": [
            "https://meshlib.io/documentation/HowtoAddPluginOverview.html",
            "https://meshlib.io/",
        ],
        "meshlib_source_paths": [
            "MeshLib/source/MRCommonPlugins/",
            "MeshLib/source/MRViewer/MRStatePlugin.*",
            "MeshLib/source/MRViewer/MRRibbonMenu.*",
        ],
        "rust_owner_modules": [
            "geometry-rs/crates/zennah-geometry-py/src/lib.rs",
        ],
        "bridge_modules": [
            "geometry_sdk/accelerators/rust.py",
            "api/routers/versions.py",
        ],
        "hosted_tool_ids": [
            "select_mark_region",
            "thicken_brush",
            "scoop_brush",
            "smooth_brush",
            "measure_inspect",
        ],
        "validation_gates": [
            "uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_architecture.py::test_meshlib_workbench_manifest_exposes_command_level_rust_capabilities -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_architecture.py::test_enabled_rust_owned_workbench_commands_are_advertised_as_rust_backed -q",
            "uv run --extra dev pytest tests/test_geometry_sdk_architecture.py::test_official_workbench_plugin_assets_expose_parity_inventory_tools -q",
        ],
        "non_geometry_reason": "Plugin loading and command registration are host/runtime concerns; backend-executed geometry commands are tracked separately by command capabilities.",
        "notes": [
            "The app exposes a hosted workbench manifest, backend command bridge, and Rust-backed command advertising for enabled Rust-owned workbench commands.",
            "Native MeshInspector plugin binary loading and full scripting parity are not implemented.",
        ],
    },
]


def _load_json_artifact(artifact: ModelArtifactRecord | None) -> dict | None:
    if artifact is None:
        return None
    if object_store.driver == "local":
        path = object_store.get_local_path(artifact.storage_key)
    else:
        path = object_store.download_to_path(artifact.storage_key, _download_temp_path(artifact))
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None


def _download_temp_path(artifact: ModelArtifactRecord) -> Path:
    filename = f"{artifact.id}_{Path(artifact.storage_key).name}"
    return settings.TEMP_DIR / "downloads" / artifact.version_id / filename


def _materialize_artifact_to_path(artifact: ModelArtifactRecord) -> Path:
    if object_store.driver == "local":
        return object_store.get_local_path(artifact.storage_key)
    return object_store.download_to_path(artifact.storage_key, _download_temp_path(artifact))


def _selected_region_vertex_indices(region_payload: dict | None, selected_region_ids: str | None) -> list[int]:
    region_ids = [value.strip() for value in (selected_region_ids or "").split(",") if value.strip()]
    if not region_ids:
        return []
    if not region_payload:
        raise HTTPException(status_code=404, detail="Region artifact not found")
    region_map = {str(region.get("region_id")): region for region in region_payload.get("regions", [])}
    missing = [region_id for region_id in region_ids if region_id not in region_map]
    if missing:
        raise HTTPException(status_code=400, detail=f"Unknown selected region id(s): {', '.join(missing)}")
    indices: list[int] = []
    for region_id in region_ids:
        indices.extend(int(index) for index in region_map[region_id].get("vertex_indices", []))
    return indices


def _compute_and_store_snapshot(db: Session, version: ModelVersionRecord) -> ManufacturabilitySnapshot | None:
    """Compute a manufacturability snapshot from a version's own mesh and cache it.

    Versions created by the synchronous offset/shell/thicken/boolean endpoints do
    not run the async finalizer that builds a snapshot, so without this they fall
    back to the *parent's* numbers — showing the pre-operation weight/dimensions.
    Computing from the version's own normalized artifact yields correct numbers,
    and persisting it means the cost is paid once.
    """
    normalized = get_artifact_by_type(db, version.id, "normalized_mesh_ply")
    if normalized is None:
        return None
    try:
        mesh_path = _materialize_artifact_to_path(normalized)
        workdir = settings.TEMP_DIR / "snapshot_backfill" / version.id
        snapshot, _ = compute_manufacturability_snapshot(mesh_path, workdir)
        payload = snapshot.model_dump(mode="json")
        payload["version_id"] = version.id
        upsert_snapshot(db, version.id, "manufacturability", payload)
        db.commit()
        return serialize_snapshot(get_snapshot(db, version.id))
    except Exception:  # noqa: BLE001 - snapshot backfill is best-effort
        db.rollback()
        return None


def _snapshot_for_version_or_parent(db: Session, version: ModelVersionRecord) -> ManufacturabilitySnapshot | None:
    snapshot = serialize_snapshot(get_snapshot(db, version.id))
    if snapshot is not None:
        return snapshot
    # Prefer the version's own (correct) numbers over the parent's stale ones.
    computed = _compute_and_store_snapshot(db, version)
    if computed is not None:
        return computed
    if version.parent_version_id:
        parent_snapshot = serialize_snapshot(get_snapshot(db, version.parent_version_id))
        if parent_snapshot is not None:
            return parent_snapshot.model_copy(update={"version_id": version.id})
    return None


def _artifact_by_type_or_parent(db: Session, version: ModelVersionRecord, artifact_type: str) -> ModelArtifactRecord | None:
    artifact = get_artifact_by_type(db, version.id, artifact_type)
    if artifact is not None:
        return artifact
    topology_preserving_ops = {
        "offset_verts",
        "interactive_brush_replay",
        "scoop",
        "thicken",
        "smooth",
    }
    if version.parent_version_id and version.operation_type in topology_preserving_ops:
        return get_artifact_by_type(db, version.parent_version_id, artifact_type)
    return None


def _texture_artifact_index(artifact: ModelArtifactRecord) -> int:
    value = artifact.metadata_json.get("texture_index")
    if isinstance(value, int):
        return value
    try:
        return int(value)
    except (TypeError, ValueError):
        return 0


def _texture_artifacts_for_version(db: Session, version_id: str) -> list[TextureArtifactManifest]:
    texture_artifacts = [
        artifact
        for artifact in get_version_artifacts(db, version_id)
        if artifact.artifact_type == "texture_image"
    ]
    texture_artifacts.sort(key=lambda artifact: (_texture_artifact_index(artifact), artifact.id))
    return [
        TextureArtifactManifest(
            texture_index=_texture_artifact_index(artifact),
            artifact_url=f"/api/artifacts/{artifact.id}",
            metadata=artifact.metadata_json,
        )
        for artifact in texture_artifacts
    ]


def _texture_per_face_from_artifacts(texture_artifacts: list[TextureArtifactManifest]) -> list[int]:
    for artifact in texture_artifacts:
        texture_per_face = artifact.metadata.get("texture_per_face")
        if isinstance(texture_per_face, list):
            return [int(texture_id) for texture_id in texture_per_face]
    return []


def _point_tuple(point) -> tuple[float, float, float]:
    return (float(point[0]), float(point[1]), float(point[2]))


def _distance_mm(start: tuple[float, float, float], end: tuple[float, float, float]) -> float:
    return math.sqrt(sum((start[index] - end[index]) ** 2 for index in range(3)))


def _nearest_mesh_vertex(vertices, point: tuple[float, float, float]) -> int:  # noqa: ANN001
    best_index = -1
    best_distance_sq = math.inf
    for index, vertex in enumerate(vertices):
        distance_sq = sum((float(vertex[axis]) - point[axis]) ** 2 for axis in range(3))
        if distance_sq < best_distance_sq:
            best_index = int(index)
            best_distance_sq = distance_sq
    if best_index < 0:
        raise HTTPException(status_code=400, detail="Mesh has no vertices for geodesic measurement")
    return best_index


def _face_average_scalar(mesh, face_index: int, values: list[object] | None) -> float | None:
    if values is None or face_index < 0 or face_index >= int(mesh.faces.shape[0]):
        return None
    local_values: list[float] = []
    for vertex_index in mesh.faces[face_index]:
        index = int(vertex_index)
        if index < 0 or index >= len(values):
            continue
        try:
            value = float(values[index])
        except (TypeError, ValueError):
            continue
        # The overlay payload sanitizes deferred/NaN thickness samples to 0.0
        # for rendering; a non-positive thickness is "unknown", not a value.
        if math.isfinite(value) and value > 0.0:
            local_values.append(value)
    if not local_values:
        return None
    return sum(local_values) / len(local_values)


def _selection_counts(request: SelectionCommitRequest) -> dict[str, int]:
    selection = request.selection
    return {
        "vertex_ids": len(selection.vertex_ids),
        "face_ids": len(selection.face_ids),
        "region_ids": len(selection.region_ids),
        "brush_points_world": len(selection.brush_points_world),
    }


def _selection_targets_point_cloud(request: SelectionCommitRequest) -> bool:
    metadata = {**request.metadata, **request.selection.metadata}
    object_type = str(
        metadata.get("object_type")
        or metadata.get("source_object_type")
        or metadata.get("target_object_type")
        or metadata.get("meshlib_object_type")
        or ""
    ).lower()
    return object_type in {"point_cloud", "pointcloud", "object_points", "objectpoints"}


def _point_cloud_metadata_int_list(metadata: dict, *keys: str) -> list[int]:
    for key in keys:
        value = metadata.get(key)
        if value is None:
            continue
        if not isinstance(value, (list, tuple)):
            raise HTTPException(status_code=400, detail=f"{key} must be a list of point indices")
        return [int(index) for index in value]
    return []


_POINT_CLOUD_CURRENT_ID_KEYS = (
    "previous_point_ids",
    "previousPointIds",
    "current_point_ids",
    "currentPointIds",
    "existing_point_ids",
    "existingPointIds",
    "previous_vertex_ids",
    "previousVertexIds",
    "current_vertex_ids",
    "currentVertexIds",
)


def _point_cloud_current_point_ids(metadata: dict) -> list[int]:
    return _point_cloud_metadata_int_list(metadata, *_POINT_CLOUD_CURRENT_ID_KEYS)


def _resolve_point_cloud_selection_ids(cloud: PointCloudDocument, selection) -> list[int]:
    point_ids = [int(index) for index in selection.vertex_ids]
    point_ids.extend(
        _point_cloud_metadata_int_list(selection.metadata, "point_ids", "selected_point_ids", "selectedPointIds")
    )
    resolved: list[int] = []
    for point_id in point_ids:
        if point_id < 0 or point_id >= cloud.point_count:
            raise HTTPException(
                status_code=400,
                detail=f"Selection point {point_id} is outside the point cloud point range",
            )
        resolved.append(point_id)
    incoming_point_ids = sorted(set(resolved))
    modifier_mode = _selection_modifier_mode(selection.metadata)
    current_point_ids = _point_cloud_current_point_ids(selection.metadata)
    if current_point_ids or modifier_mode != "replace":
        try:
            incoming_point_ids = apply_meshlib_selection_modifier(
                current_point_ids,
                incoming_point_ids,
                modifier_mode,
                item_count=cloud.point_count,
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    return incoming_point_ids


def _commit_point_cloud_selection(
    version_id: str,
    version: ModelVersionRecord,
    request: SelectionCommitRequest,
    db: Session,
) -> SelectionCommitResponse:
    normalized = get_artifact_by_type(db, version_id, "normalized_point_cloud_ply")
    if normalized is None:
        raise HTTPException(status_code=404, detail="Normalized point cloud artifact not found")
    try:
        cloud = default_sdk.load_point_cloud_ply(_materialize_artifact_to_path(normalized))
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    resolved_point_ids = _resolve_point_cloud_selection_ids(cloud, request.selection)
    if request.create_object and not resolved_point_ids:
        raise HTTPException(status_code=400, detail="Point-cloud Selection to Object requires selected points")
    counts = _selection_counts(request)
    resolved_counts = {
        "vertex_ids": len(resolved_point_ids),
        "point_ids": len(resolved_point_ids),
    }
    payload = {
        "version_id": version_id,
        "tool_id": request.tool_id,
        "operation_label": request.operation_label,
        "label": request.label,
        "create_object": request.create_object,
        "object_type": "point_cloud",
        "selection": request.selection.model_dump(mode="json"),
        "selection_counts": counts,
        "resolved_vertex_ids": resolved_point_ids,
        "resolved_point_ids": resolved_point_ids,
        "resolved_face_ids": [],
        "resolved_counts": resolved_counts,
        "metadata": request.metadata,
    }
    selection_dir = settings.TEMP_DIR / "selection_commits" / version_id
    selection_dir.mkdir(parents=True, exist_ok=True)
    selection_path = selection_dir / "meshlib_selection.json"
    selection_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
    artifact = register_file_artifact(
        db,
        version_id,
        selection_path,
        "meshlib_selection_json",
        "application/json",
        metadata_json={
            "source": "meshlib_workbench",
            "object_type": "point_cloud",
            "tool_id": request.tool_id,
            "operation_label": request.operation_label,
            "label": request.label,
            "selection_counts": counts,
            "resolved_counts": resolved_counts,
            **request.metadata,
        },
    )
    selected_object_version = None
    selected_object_artifact = None
    selected_object_counts = None
    if request.create_object:
        try:
            selected_cloud = default_sdk.point_cloud_extract_selected_points_as_object(cloud, resolved_point_ids)
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
        selected_object_version = create_version(
            db,
            model_id=version.model_id,
            parent_version_id=version.id,
            operation_type="selection_to_object",
            operation_label=request.operation_label,
            status="ready",
        )
        selected_object_path = selection_dir / "selection_object_points.ply"
        default_sdk.save_point_cloud_ply(selected_cloud, selected_object_path)
        selected_object_counts = {
            "point_ids": int(selected_cloud.point_count),
        }
        selected_object_metadata = {
            **selected_cloud.metadata,
            **request.metadata,
            "source": "meshlib_point_cloud_selection_to_object",
            "object_type": "point_cloud",
            "tool_id": request.tool_id,
            "operation_label": request.operation_label,
            "label": request.label,
            "source_version_id": version.id,
            "source_selection_artifact_id": artifact.id,
            "selection_counts": counts,
            "resolved_counts": resolved_counts,
            "point_count": int(selected_cloud.point_count),
            "meshlib_reference": "MR::ObjectPoints::cloneRegion",
            "meshlib_source": "MeshLib/source/MRMesh/MRObjectPoints.cpp",
        }
        selected_object_artifact = register_file_artifact(
            db,
            selected_object_version.id,
            selected_object_path,
            "normalized_point_cloud_ply",
            "model/ply",
            metadata_json=selected_object_metadata,
        )
    db.commit()
    db.refresh(artifact)
    if selected_object_artifact is not None:
        db.refresh(selected_object_artifact)
    return SelectionCommitResponse(
        version_id=version_id,
        artifact_id=artifact.id,
        artifact_url=f"/api/artifacts/{artifact.id}",
        selection_counts=counts,
        resolved_counts=resolved_counts,
        selected_object_version_id=selected_object_version.id if selected_object_version is not None else None,
        selected_object_artifact_id=selected_object_artifact.id if selected_object_artifact is not None else None,
        selected_object_artifact_url=(
            f"/api/artifacts/{selected_object_artifact.id}" if selected_object_artifact is not None else None
        ),
        selected_object_artifact_type=(
            selected_object_artifact.artifact_type if selected_object_artifact is not None else None
        ),
        selected_object_counts=selected_object_counts,
    )


def _selection_has_content(request: SelectionCommitRequest) -> bool:
    if any(count > 0 for count in _selection_counts(request).values()):
        return True
    return str(request.selection.metadata.get("selector") or "") in {
        "boundary_faces",
        "boundary_edges",
        "camera_facing_faces",
        "crease_edges",
        "degenerate_faces",
        "area_faces",
        "graph_cut_region",
        "inside_part_faces",
        "largest_component",
        "not_smooth_faces",
        "outer_layer_faces",
        "overlapping_faces",
        "overhang_faces",
        "pick_face",
        "screen_brush_faces",
        "screen_lasso_faces",
        "screen_rect_faces",
        "short_edges",
        "self_intersections",
    }


def _region_payload_from_entries(entries) -> dict:
    return {
        "regions": [
            {
                "region_id": str(entry.region_id),
                "label": str(entry.label),
                "vertex_indices": [int(index) for index in entry.vertex_indices],
                "coverage_pct": float(entry.coverage_pct),
                "protected_by_default": bool(entry.protected_by_default),
                "allowed_operations": [str(operation) for operation in entry.allowed_operations],
                "min_thickness_mm": entry.min_thickness_mm,
                "avg_thickness_mm": entry.avg_thickness_mm,
                "violation_count": int(entry.violation_count),
                "centroid_mm": entry.centroid_mm,
            }
            for entry in entries
        ]
    }


def _region_entries_from_payload(payload: dict | None) -> list[RegionEntry]:
    if not payload:
        return []
    entries: list[RegionEntry] = []
    for item in payload.get("regions", []):
        centroid = item.get("centroid_mm")
        entries.append(
            RegionEntry(
                region_id=str(item.get("region_id", "")),
                label=str(item.get("label") or item.get("region_id") or ""),
                vertex_indices=[int(index) for index in item.get("vertex_indices", [])],
                coverage_pct=float(item.get("coverage_pct", 0.0)),
                protected_by_default=bool(item.get("protected_by_default", False)),
                allowed_operations=[str(operation) for operation in item.get("allowed_operations", [])],
                min_thickness_mm=item.get("min_thickness_mm"),
                avg_thickness_mm=item.get("avg_thickness_mm"),
                violation_count=int(item.get("violation_count", 0)),
                centroid_mm=tuple(float(value) for value in centroid) if centroid is not None else None,
            )
        )
    return entries


def _region_payload_has_ids(region_payload: dict | None, region_ids: list[str]) -> bool:
    if not region_ids:
        return True
    if not region_payload:
        return False
    available_ids = {str(region.get("region_id")) for region in region_payload.get("regions", [])}
    return all(region_id in available_ids for region_id in region_ids)


def _detect_region_payload(mesh) -> dict:
    measurement = default_sdk.measure_ring(mesh)
    return _region_payload_from_entries(default_sdk.detect_ring_regions(mesh, measurement))


def _metadata_bool(metadata: dict, key: str, default: bool) -> bool:
    value = metadata.get(key)
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized in {"1", "true", "yes", "on"}:
            return True
        if normalized in {"0", "false", "no", "off"}:
            return False
    return bool(value)


def _metadata_float(metadata: dict, keys: tuple[str, ...], default: float) -> float:
    for key in keys:
        if key not in metadata:
            continue
        try:
            value = float(metadata[key])
        except (TypeError, ValueError) as exc:
            raise HTTPException(status_code=400, detail=f"{key} must be numeric") from exc
        if not math.isfinite(value):
            raise HTTPException(status_code=400, detail=f"{key} must be finite")
        return value
    return default


def _metadata_string(metadata: dict, keys: tuple[str, ...], default: str) -> str:
    for key in keys:
        if key not in metadata:
            continue
        value = metadata[key]
        if value is None:
            return default
        if not isinstance(value, str):
            raise HTTPException(status_code=400, detail=f"{key} must be a string")
        return value
    return default


def _metadata_int(metadata: dict, keys: tuple[str, ...], default: int) -> int:
    for key in keys:
        if key not in metadata:
            continue
        try:
            value = int(metadata[key])
        except (TypeError, ValueError) as exc:
            raise HTTPException(status_code=400, detail=f"{key} must be an integer") from exc
        return value
    return default


def _metadata_int_list(metadata: dict, keys: tuple[str, ...]) -> list[int]:
    for key in keys:
        if key not in metadata:
            continue
        value = metadata[key]
        if not isinstance(value, (list, tuple)):
            raise HTTPException(status_code=400, detail=f"{key} must be a list of face ids")
        output: list[int] = []
        for item in value:
            if isinstance(item, bool):
                raise HTTPException(status_code=400, detail=f"{key} values must be integers")
            try:
                face_id = int(item)
            except (TypeError, ValueError) as exc:
                raise HTTPException(status_code=400, detail=f"{key} values must be integers") from exc
            if face_id < 0:
                raise HTTPException(status_code=400, detail=f"{key} values must be non-negative")
            output.append(face_id)
        return output
    return []


_SELECTION_CURRENT_FACE_ID_KEYS = (
    "previous_face_ids",
    "previousFaceIds",
    "current_face_ids",
    "currentFaceIds",
    "existing_face_ids",
    "existingFaceIds",
)
_SELECTION_CURRENT_VERTEX_ID_KEYS = (
    "previous_vertex_ids",
    "previousVertexIds",
    "current_vertex_ids",
    "currentVertexIds",
    "existing_vertex_ids",
    "existingVertexIds",
)
_SELECTION_MODIFIER_KEYS = (
    "selection_modifier",
    "selectionModifier",
    "selection_operation",
    "selectionOperation",
    "modifier_operation",
    "modifierOperation",
)


def _selection_current_face_ids(metadata: dict) -> list[int]:
    return _metadata_int_list(metadata, _SELECTION_CURRENT_FACE_ID_KEYS)


def _selection_current_vertex_ids(metadata: dict) -> list[int]:
    return _metadata_int_list(metadata, _SELECTION_CURRENT_VERTEX_ID_KEYS)


def _selection_modifier_mode(metadata: dict) -> str:
    for key in _SELECTION_MODIFIER_KEYS:
        if key not in metadata:
            continue
        value = metadata[key]
        if value is None:
            return "replace"
        if not isinstance(value, str):
            raise HTTPException(status_code=400, detail=f"{key} must be a string")
        return value
    primary_ctrl = (
        _metadata_bool(metadata, "modifier_primary_ctrl", False)
        or _metadata_bool(metadata, "primary_ctrl", False)
        or _metadata_bool(metadata, "primaryCtrl", False)
        or _metadata_bool(metadata, "ctrlKey", False)
        or _metadata_bool(metadata, "metaKey", False)
        or _metadata_bool(metadata, "commandKey", False)
    )
    return "toggle" if primary_ctrl else "replace"


def _metadata_optional_float(metadata: dict, keys: tuple[str, ...]) -> float | None:
    if not any(key in metadata for key in keys):
        return None
    return _metadata_float(metadata, keys, 0.0)


def _overhang_axis_metadata(metadata: dict) -> list[float]:
    axis = (
        metadata.get("axis")
        or metadata.get("axis_vector")
        or metadata.get("axisVector")
        or metadata.get("direction")
    )
    if axis is None and any(key in metadata for key in ("axis_x", "axis_y", "axis_z", "axisX", "axisY", "axisZ")):
        axis = [
            metadata.get("axis_x", metadata.get("axisX", 0.0)),
            metadata.get("axis_y", metadata.get("axisY", 0.0)),
            metadata.get("axis_z", metadata.get("axisZ", 1.0)),
        ]
    if axis is None:
        return [0.0, 0.0, 1.0]
    if isinstance(axis, str):
        normalized = axis.strip().lower().replace(" ", "_").replace("-", "_")
        axis_map = {
            "x": [1.0, 0.0, 0.0],
            "to_x": [1.0, 0.0, 0.0],
            "positive_x": [1.0, 0.0, 0.0],
            "_x": [-1.0, 0.0, 0.0],
            "y": [0.0, 1.0, 0.0],
            "to_y": [0.0, 1.0, 0.0],
            "positive_y": [0.0, 1.0, 0.0],
            "_y": [0.0, -1.0, 0.0],
            "z": [0.0, 0.0, 1.0],
            "to_z": [0.0, 0.0, 1.0],
            "positive_z": [0.0, 0.0, 1.0],
            "_z": [0.0, 0.0, -1.0],
            "negative_x": [-1.0, 0.0, 0.0],
            "negative_y": [0.0, -1.0, 0.0],
            "negative_z": [0.0, 0.0, -1.0],
        }
        if normalized in axis_map:
            return axis_map[normalized]
        camera_axis = metadata.get("camera_axis") or metadata.get("cameraAxis") or metadata.get("camera_direction")
        if normalized in {"camera", "to_camera"} and camera_axis is not None:
            axis = camera_axis
        else:
            raise HTTPException(status_code=400, detail="overhang_faces axis must be a 3-vector or x/y/z direction")
    if not isinstance(axis, (list, tuple)) or len(axis) != 3:
        raise HTTPException(status_code=400, detail="overhang_faces requires axis with 3 values")
    try:
        values = [float(value) for value in axis]
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="overhang_faces axis values must be numeric") from exc
    if not all(math.isfinite(value) for value in values):
        raise HTTPException(status_code=400, detail="overhang_faces axis values must be finite")
    return values


def _screen_lasso_metadata(selection) -> tuple[list[float], list[list[float]], bool, bool]:
    metadata = selection.metadata
    view_projection = metadata.get("view_projection_4x4") or metadata.get("viewProjection4x4")
    if not isinstance(view_projection, list) or len(view_projection) != 16:
        raise HTTPException(status_code=400, detail="screen_lasso_faces requires view_projection_4x4 with 16 values")
    try:
        view_projection_values = [float(value) for value in view_projection]
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="view_projection_4x4 values must be numeric") from exc

    polygon = metadata.get("polygon_xy") or metadata.get("polygon") or metadata.get("screen_polygon_xy")
    if not isinstance(polygon, list) or len(polygon) < 3:
        raise HTTPException(status_code=400, detail="screen_lasso_faces requires polygon_xy with at least 3 points")
    polygon_values: list[list[float]] = []
    try:
        for point in polygon:
            if not isinstance(point, (list, tuple)) or len(point) != 2:
                raise ValueError
            polygon_values.append([float(point[0]), float(point[1])])
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="polygon_xy points must be numeric [x, y] pairs") from exc

    return (
        view_projection_values,
        polygon_values,
        _metadata_bool(metadata, "include_backfaces", True),
        _metadata_bool(metadata, "visible_only", False),
    )


def _screen_rect_metadata(selection) -> tuple[list[float], list[float], list[float], bool, bool]:
    metadata = selection.metadata
    view_projection = metadata.get("view_projection_4x4") or metadata.get("viewProjection4x4")
    if not isinstance(view_projection, list) or len(view_projection) != 16:
        raise HTTPException(status_code=400, detail="screen_rect_faces requires view_projection_4x4 with 16 values")
    try:
        view_projection_values = [float(value) for value in view_projection]
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="view_projection_4x4 values must be numeric") from exc

    rect_min = (
        metadata.get("rect_min_xy")
        or metadata.get("rectMinXy")
        or metadata.get("min_xy")
        or metadata.get("minXy")
    )
    rect_max = (
        metadata.get("rect_max_xy")
        or metadata.get("rectMaxXy")
        or metadata.get("max_xy")
        or metadata.get("maxXy")
    )
    rect_xyxy = metadata.get("rect_xyxy") or metadata.get("rectXyxy") or metadata.get("rectangle_xyxy")
    if (rect_min is None or rect_max is None) and isinstance(rect_xyxy, (list, tuple)) and len(rect_xyxy) == 4:
        rect_min = [rect_xyxy[0], rect_xyxy[1]]
        rect_max = [rect_xyxy[2], rect_xyxy[3]]
    if not isinstance(rect_min, (list, tuple)) or len(rect_min) != 2:
        raise HTTPException(status_code=400, detail="screen_rect_faces requires rect_min_xy with 2 values")
    if not isinstance(rect_max, (list, tuple)) or len(rect_max) != 2:
        raise HTTPException(status_code=400, detail="screen_rect_faces requires rect_max_xy with 2 values")
    try:
        rect_min_values = [float(rect_min[0]), float(rect_min[1])]
        rect_max_values = [float(rect_max[0]), float(rect_max[1])]
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="screen_rect_faces rectangle values must be numeric") from exc
    if not all(math.isfinite(value) for value in rect_min_values + rect_max_values):
        raise HTTPException(status_code=400, detail="screen_rect_faces rectangle values must be finite")

    return (
        view_projection_values,
        rect_min_values,
        rect_max_values,
        _metadata_bool(metadata, "include_backfaces", True),
        _metadata_bool(metadata, "visible_only", False),
    )


def _screen_brush_metadata(selection) -> tuple[list[float], list[list[float]], float, bool, bool]:
    metadata = selection.metadata
    view_projection = metadata.get("view_projection_4x4") or metadata.get("viewProjection4x4")
    if not isinstance(view_projection, list) or len(view_projection) != 16:
        raise HTTPException(status_code=400, detail="screen_brush_faces requires view_projection_4x4 with 16 values")
    try:
        view_projection_values = [float(value) for value in view_projection]
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="view_projection_4x4 values must be numeric") from exc

    brush_path = (
        metadata.get("brush_path_xy")
        or metadata.get("brushPathXy")
        or metadata.get("brush_path")
        or metadata.get("brushPath")
        or metadata.get("stroke_xy")
        or metadata.get("strokeXy")
    )
    if not isinstance(brush_path, list) or not brush_path:
        raise HTTPException(status_code=400, detail="screen_brush_faces requires brush_path_xy with at least one point")
    brush_path_values: list[list[float]] = []
    try:
        for point in brush_path:
            if not isinstance(point, (list, tuple)) or len(point) != 2:
                raise ValueError
            brush_path_values.append([float(point[0]), float(point[1])])
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail="brush_path_xy points must be numeric [x, y] pairs") from exc

    return (
        view_projection_values,
        brush_path_values,
        _metadata_float(metadata, ("radius_px", "radiusPx", "brush_radius_px", "brushRadiusPx"), 0.0),
        _metadata_bool(metadata, "include_backfaces", True),
        _metadata_bool(metadata, "visible_only", False),
    )


def _metadata_vec3(metadata: dict, keys: tuple[str, ...], label: str) -> list[float]:
    value = None
    for key in keys:
        if key in metadata:
            value = metadata[key]
            break
    if not isinstance(value, (list, tuple)) or len(value) != 3:
        raise HTTPException(status_code=400, detail=f"{label} requires 3 values")
    try:
        values = [float(value[0]), float(value[1]), float(value[2])]
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=f"{label} values must be numeric") from exc
    if not all(math.isfinite(item) for item in values):
        raise HTTPException(status_code=400, detail=f"{label} values must be finite")
    return values


def _ray_pick_metadata(selection) -> tuple[list[float], list[float], float, list[int]]:
    metadata = selection.metadata
    ray_origin = _metadata_vec3(
        metadata,
        ("ray_origin", "rayOrigin", "origin", "camera_origin", "cameraOrigin"),
        "pick_face ray_origin",
    )
    ray_direction = _metadata_vec3(
        metadata,
        ("ray_direction", "rayDirection", "direction", "camera_direction", "cameraDirection"),
        "pick_face ray_direction",
    )
    if sum(value * value for value in ray_direction) <= 1e-24:
        raise HTTPException(status_code=400, detail="pick_face ray_direction magnitude is too small")
    epsilon = _metadata_float(metadata, ("epsilon", "pick_epsilon", "pickEpsilon"), 1e-8)
    ignored_faces = _metadata_int_list(
        metadata,
        ("ignore_faces", "ignoreFaces", "ignore_face_ids", "ignoreFaceIds", "ignored_faces", "ignoredFaceIds"),
    )
    return ray_origin, ray_direction, epsilon, ignored_faces


def _camera_facing_metadata(selection) -> tuple[list[float], float]:
    camera_direction = _metadata_vec3(
        selection.metadata,
        (
            "camera_direction",
            "cameraDirection",
            "view_direction",
            "viewDirection",
            "camera_forward",
            "cameraForward",
        ),
        "camera_facing_faces camera_direction",
    )
    min_dot = _metadata_float(selection.metadata, ("min_dot", "minDot", "threshold"), 0.0)
    return camera_direction, min_dot


def _not_smooth_min_angle_metadata(selection) -> float:
    metadata = selection.metadata
    degree_keys = ("min_angle_degrees", "minAngleDegrees", "min_angle_deg", "minAngleDeg")
    if any(key in metadata for key in degree_keys):
        return math.radians(_metadata_float(metadata, degree_keys, 0.0))
    return _metadata_float(metadata, ("min_angle_radians", "minAngleRadians", "min_angle", "minAngle"), 0.3)


def _resolve_selection_faces_and_seed_vertices(mesh, selection, region_payload: dict | None) -> tuple[list[int], list[int]]:
    vertex_count = int(mesh.vertex_count)
    face_count = int(mesh.faces.shape[0])
    indices: list[int] = []

    for vertex_id in selection.vertex_ids:
        index = int(vertex_id)
        if index < 0 or index >= vertex_count:
            raise HTTPException(status_code=400, detail=f"Selection vertex {index} is outside the mesh vertex range")
        indices.append(index)

    selected_face_ids: list[int] = []
    selector = str(selection.metadata.get("selector") or "")
    if selector == "boundary_faces":
        try:
            selected_face_ids.extend(select_boundary_faces(mesh))
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "boundary_edges":
        try:
            for edge in select_boundary_edges(mesh):
                indices.extend(int(vertex_index) for vertex_index in edge)
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "screen_lasso_faces":
        view_projection, polygon, include_backfaces, visible_only = _screen_lasso_metadata(selection)
        try:
            selected_face_ids.extend(
                select_faces_by_screen_polygon(
                    mesh,
                    view_projection,
                    polygon,
                    include_backfaces=include_backfaces,
                    visible_only=visible_only,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "screen_rect_faces":
        view_projection, rect_min, rect_max, include_backfaces, visible_only = _screen_rect_metadata(selection)
        try:
            selected_face_ids.extend(
                select_faces_by_screen_rect(
                    mesh,
                    view_projection,
                    rect_min,
                    rect_max,
                    include_backfaces=include_backfaces,
                    visible_only=visible_only,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "screen_brush_faces":
        view_projection, brush_path, radius_px, include_backfaces, visible_only = _screen_brush_metadata(selection)
        try:
            selected_face_ids.extend(
                select_faces_by_screen_brush(
                    mesh,
                    view_projection,
                    brush_path,
                    radius_px=radius_px,
                    include_backfaces=include_backfaces,
                    visible_only=visible_only,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "camera_facing_faces":
        camera_direction, min_dot = _camera_facing_metadata(selection)
        try:
            selected_face_ids.extend(
                select_camera_facing_faces(
                    mesh,
                    camera_direction=camera_direction,
                    min_dot=min_dot,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "pick_face":
        ray_origin, ray_direction, epsilon, ignored_faces = _ray_pick_metadata(selection)
        try:
            selected_face_ids.extend(
                select_face_by_ray(
                    mesh,
                    ray_origin,
                    ray_direction,
                    epsilon=epsilon,
                    ignore_faces=ignored_faces,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "largest_component":
        min_area_mm2 = _metadata_float(
            selection.metadata,
            ("min_area_mm2", "minAreaMm2", "min_area", "minArea"),
            0.0,
        )
        try:
            selected_face_ids.extend(
                select_largest_component_faces(
                    mesh,
                    min_area_mm2=min_area_mm2,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "self_intersections":
        try:
            mode = str(
                selection.metadata.get("mode")
                or selection.metadata.get("self_intersection_mode")
                or selection.metadata.get("selfIntersectionMode")
                or "self_intersections"
            ).strip().lower()
            if mode in {"overlaps", "overlap", "overlapping", "overlapping_faces"}:
                selected_face_ids.extend(
                    select_overlapping_faces(
                        mesh,
                        max_dist_sq=_metadata_float(
                            selection.metadata,
                            ("max_dist_sq", "maxDistSq", "max_distance_sq", "maxDistanceSq"),
                            1e-10,
                        ),
                        max_normal_dot=_metadata_float(
                            selection.metadata,
                            ("max_normal_dot", "maxNormalDot"),
                            -0.99,
                        ),
                        min_area_fraction=_metadata_float(
                            selection.metadata,
                            ("min_area_fraction", "minAreaFraction"),
                            1e-5,
                        ),
                    )
                )
            elif mode in {"inside", "inside_part", "inside_part_faces", "winding", "winding_number"}:
                selected_face_ids.extend(select_inside_part_faces(mesh))
            else:
                selected_face_ids.extend(
                    sorted(
                        default_sdk.self_intersecting_faces(
                            mesh,
                            touch_is_intersection=_metadata_bool(selection.metadata, "touch_is_intersection", True),
                        )
                    )
                )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "inside_part_faces":
        try:
            selected_face_ids.extend(select_inside_part_faces(mesh))
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "overlapping_faces":
        try:
            selected_face_ids.extend(
                select_overlapping_faces(
                    mesh,
                    max_dist_sq=_metadata_float(
                        selection.metadata,
                        ("max_dist_sq", "maxDistSq", "max_distance_sq", "maxDistanceSq"),
                        1e-10,
                    ),
                    max_normal_dot=_metadata_float(
                        selection.metadata,
                        ("max_normal_dot", "maxNormalDot"),
                        -0.99,
                    ),
                    min_area_fraction=_metadata_float(
                        selection.metadata,
                        ("min_area_fraction", "minAreaFraction"),
                        1e-5,
                    ),
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "degenerate_faces":
        min_aspect_ratio = _metadata_float(
            selection.metadata,
            ("min_aspect_ratio", "minAspectRatio", "critical_aspect_ratio", "criticalTriAspectRatio"),
            1e4,
        )
        try:
            boundary_only = _metadata_bool(
                selection.metadata,
                "boundary_only",
                _metadata_bool(selection.metadata, "boundaryOnly", False),
            )
            selected_face_ids.extend(
                select_degenerate_faces(
                    mesh,
                    min_aspect_ratio=min_aspect_ratio,
                    boundary_only=boundary_only,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "short_edges":
        max_edge_length_mm = _metadata_float(
            selection.metadata,
            ("max_edge_length_mm", "maxEdgeLength", "critical_length_mm", "criticalLength"),
            0.0,
        )
        try:
            for edge in select_short_edges(mesh, max_edge_length_mm=max_edge_length_mm):
                indices.extend(int(vertex_index) for vertex_index in edge)
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "crease_edges":
        angle_radian_keys = (
            "angle_from_planar_radians",
            "angleFromPlanarRadians",
            "angle_radians",
            "angleRadians",
        )
        angle_degree_keys = (
            "angle_from_planar_degrees",
            "angleFromPlanarDegrees",
            "deviation_degrees",
            "deviationFromPlaneDegrees",
        )
        if any(key in selection.metadata for key in angle_radian_keys):
            angle_from_planar_radians = _metadata_float(
                selection.metadata,
                angle_radian_keys,
                math.radians(175.0),
            )
        else:
            angle_from_planar_radians = math.radians(
                _metadata_float(
                    selection.metadata,
                    angle_degree_keys,
                    175.0,
                )
            )
        critical_length_mm = _metadata_optional_float(
            selection.metadata,
            ("critical_length_mm", "criticalLength", "criticalLengthMm"),
        )
        min_component_length_mm = _metadata_optional_float(
            selection.metadata,
            ("min_component_length_mm", "minComponentLengthMm", "filter_component_length_mm", "filterComponentLengthMm"),
        )
        min_branch_length_mm = _metadata_optional_float(
            selection.metadata,
            ("min_branch_length_mm", "minBranchLengthMm", "filter_branch_length_mm", "filterBranchLengthMm"),
        )
        if min_component_length_mm is None and _metadata_bool(selection.metadata, "filter_components", False):
            min_component_length_mm = critical_length_mm
        if min_branch_length_mm is None and _metadata_bool(selection.metadata, "filter_branches", False):
            min_branch_length_mm = critical_length_mm
        try:
            for edge in select_crease_edges(
                mesh,
                angle_from_planar_radians=angle_from_planar_radians,
                min_component_length_mm=min_component_length_mm,
                min_branch_length_mm=min_branch_length_mm,
            ):
                indices.extend(int(vertex_index) for vertex_index in edge)
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "area_faces":
        area = _metadata_float(
            selection.metadata,
            ("area", "area_threshold", "areaThreshold", "area_mm2", "areaMm2", "threshold"),
            0.0,
        )
        scalar_type = str(
            selection.metadata.get("scalar_type")
            or selection.metadata.get("scalarType")
            or selection.metadata.get("area_scalar_type")
            or selection.metadata.get("areaScalarType")
            or "absolute"
        )
        compare_type = str(
            selection.metadata.get("compare_type")
            or selection.metadata.get("compareType")
            or selection.metadata.get("area_compare_type")
            or selection.metadata.get("areaCompareType")
            or "less"
        )
        try:
            selected_face_ids.extend(
                select_faces_by_area(
                    mesh,
                    area=area,
                    scalar_type=scalar_type,
                    compare_type=compare_type,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "overhang_faces":
        layer_height_mm = _metadata_float(
            selection.metadata,
            ("layer_height_mm", "layerHeight", "layerHeightMm", "layer_step_mm", "layerStep", "layerStepMm"),
            1.0,
        )
        max_overhang_distance_mm = _metadata_float(
            selection.metadata,
            (
                "max_overhang_distance_mm",
                "maxOverhangDistance",
                "maxOverhangDistanceMm",
                "overhang_distance_mm",
                "overhangDistanceMm",
            ),
            1.0,
        )
        hops = _metadata_int(selection.metadata, ("hops", "smooth_hops", "smoothHops"), 0)
        smooth_out_overhangs = _metadata_bool(
            selection.metadata,
            "smooth_out_overhangs",
            _metadata_bool(selection.metadata, "smoothOutOverhangs", False),
        )
        if smooth_out_overhangs and hops == 0:
            hops = 1
        try:
            selected_face_ids.extend(
                select_overhang_faces(
                    mesh,
                    axis=_overhang_axis_metadata(selection.metadata),
                    layer_height_mm=layer_height_mm,
                    max_overhang_distance_mm=max_overhang_distance_mm,
                    hops=hops,
                )
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "outer_layer_faces":
        epsilon = _metadata_float(selection.metadata, ("epsilon", "ray_epsilon", "rayEpsilon"), 1e-8)
        try:
            selected_face_ids.extend(select_outer_layer_faces(mesh, epsilon=epsilon))
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "not_smooth_faces":
        min_angle_radians = _not_smooth_min_angle_metadata(selection)
        try:
            selected_face_ids.extend(select_not_smooth_faces(mesh, min_angle_radians=min_angle_radians))
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    elif selector == "graph_cut_region":
        source_face_ids = _metadata_int_list(
            selection.metadata,
            ("source_face_ids", "sourceFaceIds", "region_face_ids", "regionFaceIds"),
        )
        sink_face_ids = _metadata_int_list(
            selection.metadata,
            ("sink_face_ids", "sinkFaceIds", "not_region_face_ids", "notRegionFaceIds"),
        )
        boundary_weight = _metadata_float(
            selection.metadata,
            ("boundary_weight", "boundaryWeight"),
            1.0,
        )
        curvature_preference = _metadata_string(
            selection.metadata,
            ("curvature_preference", "curvaturePreference", "path_preference", "pathPreference"),
            "geodesic",
        )
        try:
            if sink_face_ids:
                selected_face_ids.extend(
                    graph_cut_select_region(
                        mesh,
                        source_face_ids=source_face_ids,
                        sink_face_ids=sink_face_ids,
                        boundary_weight=boundary_weight,
                        curvature_preference=curvature_preference,
                    )
                )
            else:
                selected_face_ids.extend(
                    graph_cut_select_region_auto_not_region(
                        mesh,
                        source_face_ids=source_face_ids,
                        uncertainty_distance_mm=_metadata_float(
                            selection.metadata,
                            ("uncertainty_distance_mm", "uncertaintyDistanceMm", "uncertainty_distance", "uncertaintyDistance"),
                            0.0,
                        ),
                        boundary_weight=boundary_weight,
                        curvature_preference=curvature_preference,
                    )
                )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc

    for face_id in selection.face_ids:
        index = int(face_id)
        if index < 0 or index >= face_count:
            raise HTTPException(status_code=400, detail=f"Selection face {index} is outside the mesh face range")
        selected_face_ids.append(index)

    if selected_face_ids and bool(selection.metadata.get("expand_to_components")):
        try:
            selected_face_ids = expand_face_selection_to_components(mesh, selected_face_ids)
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc

    if selection.region_ids:
        if not region_payload:
            raise HTTPException(status_code=404, detail="Region artifact not found")
        region_map = {str(region.get("region_id")): region for region in region_payload.get("regions", [])}
        for region_id in selection.region_ids:
            region = region_map.get(str(region_id))
            if region is None:
                raise HTTPException(status_code=400, detail=f"Unknown selected region id: {region_id}")
            for vertex_id in region.get("vertex_indices", []):
                index = int(vertex_id)
                if index < 0 or index >= vertex_count:
                    raise HTTPException(status_code=400, detail=f"Region {region_id} contains invalid mesh vertex {index}")
                indices.append(index)

    if selection.brush_points_world:
        _closest_points, _distances, face_indices = default_sdk.closest_points_on_mesh(selection.brush_points_world, mesh)
        for face_index in face_indices:
            index = int(face_index)
            if index < 0 or index >= face_count:
                raise HTTPException(status_code=400, detail=f"Brush point resolved outside the mesh face range: {index}")
            selected_face_ids.append(index)

    incoming_face_ids = sorted(set(selected_face_ids))
    modifier_mode = _selection_modifier_mode(selection.metadata)
    current_face_ids = _selection_current_face_ids(selection.metadata)
    if current_face_ids or modifier_mode != "replace":
        try:
            incoming_face_ids = apply_meshlib_selection_modifier(
                current_face_ids,
                incoming_face_ids,
                modifier_mode,
                item_count=face_count,
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc

    return incoming_face_ids, indices


def _resolve_selection_face_ids(mesh, selection, region_payload: dict | None) -> list[int]:
    selected_face_ids, _indices = _resolve_selection_faces_and_seed_vertices(mesh, selection, region_payload)
    return selected_face_ids


def _resolve_selection_vertex_ids(mesh, selection, region_payload: dict | None) -> list[int]:
    selected_face_ids, indices = _resolve_selection_faces_and_seed_vertices(mesh, selection, region_payload)
    for index in selected_face_ids:
        indices.extend(int(vertex_index) for vertex_index in mesh.faces[index])
    incoming_vertex_ids = sorted(set(indices))
    modifier_mode = _selection_modifier_mode(selection.metadata)
    current_vertex_ids = _selection_current_vertex_ids(selection.metadata)
    if current_vertex_ids or modifier_mode != "replace":
        try:
            incoming_vertex_ids = apply_meshlib_selection_modifier(
                current_vertex_ids,
                incoming_vertex_ids,
                modifier_mode,
                item_count=int(mesh.vertex_count),
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
    return incoming_vertex_ids


def _serialize_section_contour(contour) -> SectionContourPayload:
    return SectionContourPayload(
        section_constant=contour.section_constant,
        plane_axis=contour.plane_axis,
        plane_u_axis=contour.plane_u_axis,
        plane_v_axis=contour.plane_v_axis,
        plane_origin=contour.plane_origin,
        contour_count=contour.contour_count,
        segment_count=contour.segment_count,
        selected_region_segment_count=contour.selected_region_segment_count,
        perimeter_mm=contour.perimeter_mm,
        width_mm=contour.width_mm,
        depth_mm=contour.depth_mm,
        projected_bounds_min=contour.projected_bounds_min,
        projected_bounds_max=contour.projected_bounds_max,
        bounds_min=contour.bounds_min,
        bounds_max=contour.bounds_max,
        segments=[
            SectionContourSegment(
                start=segment.start,
                end=segment.end,
                selected_region_hit=segment.selected_region_hit,
            )
            for segment in contour.segments
        ],
    )


def _workbench_endpoint_urls(version: ModelVersionRecord, db: Session | None = None) -> dict[str, str | None]:
    version_id = version.id
    manufacturing_stl = get_artifact_by_type(db, version_id, "manufacturing_stl") if db is not None else None
    return {
        "artifact_endpoint_url": f"/api/artifacts/{manufacturing_stl.id}" if manufacturing_stl else None,
        "branch_endpoint_url": f"/api/versions/{version_id}/branch",
        "brush_endpoint_url": f"/api/versions/{version_id}/brush-replay",
        "collision_endpoint_url": f"/api/versions/{version_id}/collision/detect",
        "commit_endpoint_url": f"/api/versions/{version_id}/interactive-commit",
        "compare_endpoint_url": f"/api/versions/{version_id}/compare",
        "decimate_endpoint_url": f"/api/versions/{version_id}/decimate",
        "distance_map_contours_endpoint_url": f"/api/versions/{version_id}/distance-map/contours",
        "distance_map_from_mesh_endpoint_url": f"/api/versions/{version_id}/distance-map/mesh",
        "distance_map_iso_lines_endpoint_url": f"/api/versions/{version_id}/distance-map/iso-lines",
        "distance_map_merge_endpoint_url": f"/api/versions/{version_id}/distance-map/merge",
        "distance_map_contour_boolean_endpoint_url": f"/api/versions/{version_id}/distance-map/contour-boolean",
        "distance_map_from_tiff_endpoint_url": f"/api/versions/{version_id}/distance-map/from-tiff",
        "distance_map_to_tiff_endpoint_url": f"/api/versions/{version_id}/distance-map/to-tiff",
        "expand_shrink_endpoint_url": f"/api/versions/{version_id}/offset/expand-shrink",
        "exact_boolean_endpoint_url": f"/api/versions/{version_id}/boolean/exact",
        "gcode_load_source_endpoint_url": f"/api/versions/{version_id}/gcode/load-source",
        "gcode_parse_file_paths_endpoint_url": f"/api/versions/{version_id}/gcode/parse-file-paths",
        "gcode_parse_paths_endpoint_url": f"/api/versions/{version_id}/gcode/parse-paths",
        "gcode_write_source_endpoint_url": f"/api/versions/{version_id}/gcode/write-source",
        "hollow_endpoint_url": f"/api/versions/{version_id}/hollow",
        "inspection_snapshots_endpoint_url": f"/api/versions/{version_id}/inspection-snapshots",
        "jobs_endpoint_url": f"/api/versions/{version_id}/jobs",
        "make_delone_endpoint_url": f"/api/versions/{version_id}/make-delone",
        "make_manufacturable_endpoint_url": f"/api/versions/{version_id}/make-manufacturable",
        "measurement_endpoint_url": f"/api/versions/{version_id}/measure-inspect",
        "mesh_cut_measure_topology_endpoint_url": f"/api/versions/{version_id}/mesh-cut-measure/topology",
        "model_versions_endpoint_url": f"/api/models/{version.model_id}/versions",
        "object_lines_from_contours_endpoint_url": f"/api/versions/{version_id}/object-lines/from-contours",
        "open_raw_voxels_endpoint_url": f"/api/versions/{version_id}/voxels/open-raw",
        "open_voxels_from_tiff_endpoint_url": f"/api/versions/{version_id}/voxels/open-tiff-dir",
        "object_lines_load_mrlines_endpoint_url": f"/api/versions/{version_id}/object-lines/load-mrlines",
        "object_lines_load_ply_endpoint_url": f"/api/versions/{version_id}/object-lines/load-ply",
        "object_lines_load_pts_endpoint_url": f"/api/versions/{version_id}/object-lines/load-pts",
        "object_lines_load_svg_endpoint_url": f"/api/versions/{version_id}/object-lines/load-svg",
        "object_lines_save_dxf_endpoint_url": f"/api/versions/{version_id}/object-lines/save-dxf",
        "object_lines_save_mrlines_endpoint_url": f"/api/versions/{version_id}/object-lines/save-mrlines",
        "object_lines_save_ply_endpoint_url": f"/api/versions/{version_id}/object-lines/save-ply",
        "object_lines_save_pts_endpoint_url": f"/api/versions/{version_id}/object-lines/save-pts",
        "object_lines_to_contours_endpoint_url": f"/api/versions/{version_id}/object-lines/to-contours",
        "offset_contours_endpoint_url": f"/api/versions/{version_id}/contours/offset",
        "offset_mesh_endpoint_url": f"/api/versions/{version_id}/offset/voxel",
        "offset_verts_endpoint_url": f"/api/versions/{version_id}/offset/verts",
        "partial_offset_endpoint_url": f"/api/versions/{version_id}/offset/partial",
        "point_cloud_icp_endpoint_url": f"/api/versions/{version_id}/point-cloud/icp",
        "point_cloud_multiway_icp_endpoint_url": f"/api/versions/{version_id}/point-cloud/icp/multiway",
        "point_cloud_triangulation_endpoint_url": f"/api/versions/{version_id}/point-cloud/triangulate",
        "repair_endpoint_url": f"/api/versions/{version_id}/repair",
        "resize_endpoint_url": f"/api/versions/{version_id}/resize",
        "scoop_endpoint_url": f"/api/versions/{version_id}/scoop",
        "section_endpoint_url": f"/api/versions/{version_id}/section",
        "selection_endpoint_url": f"/api/versions/{version_id}/selection-commit",
        "shell_mesh_endpoint_url": f"/api/versions/{version_id}/shell/voxel",
        "shrink_expand_endpoint_url": f"/api/versions/{version_id}/offset/shrink-expand",
        "smooth_endpoint_url": f"/api/versions/{version_id}/smooth",
        "subdivide_endpoint_url": f"/api/versions/{version_id}/subdivide",
        "thicken_endpoint_url": f"/api/versions/{version_id}/thicken",
        "thicken_mesh_endpoint_url": f"/api/versions/{version_id}/offset/thicken",
        "thickness_overlay_url": f"/api/versions/{version_id}/overlays/thickness",
        "voxel_active_box_endpoint_url": f"/api/versions/{version_id}/voxels/active-box",
        "voxel_binary_operations_endpoint_url": f"/api/versions/{version_id}/voxels/binary",
        "voxel_boolean_endpoint_url": f"/api/versions/{version_id}/boolean/voxel",
        "voxel_line_graph_endpoint_url": f"/api/versions/{version_id}/voxels/line-graph",
        "voxel_mask_to_mesh_endpoint_url": f"/api/versions/{version_id}/voxels/mask-to-mesh",
        "voxel_path_endpoint_url": f"/api/versions/{version_id}/voxels/path",
        "voxel_path_build_four_endpoint_url": f"/api/versions/{version_id}/voxels/path/build-four",
        "voxel_segmentation_endpoint_url": f"/api/versions/{version_id}/voxels/segmentation",
        "voxel_slice_endpoint_url": f"/api/versions/{version_id}/voxels/slice",
        "voxel_to_mesh_dual_endpoint_url": f"/api/versions/{version_id}/voxels/to-mesh/dual",
        "voxel_to_mesh_simple_endpoint_url": f"/api/versions/{version_id}/voxels/to-mesh/simple",
        "voxel_to_mesh_smart_endpoint_url": f"/api/versions/{version_id}/voxels/to-mesh/smart",
        "voxel_volume_render_data_endpoint_url": f"/api/versions/{version_id}/voxels/volume-render-data",
        "voxel_volume_render_lut_endpoint_url": f"/api/versions/{version_id}/voxels/volume-render-lut",
        "voxel_volume_render_ray_endpoint_url": f"/api/versions/{version_id}/voxels/volume-render-ray",
        "voxelize_mesh_endpoint_url": f"/api/versions/{version_id}/voxels/mesh-to-sdf",
        "weighted_shell_endpoint_url": f"/api/versions/{version_id}/offset/weighted-shell",
    }


def _workbench_command_capabilities(version: ModelVersionRecord, db: Session | None = None) -> list[MeshLibWorkbenchCommandCapability]:
    endpoint_urls = _workbench_endpoint_urls(version, db=db)
    capabilities: list[MeshLibWorkbenchCommandCapability] = []
    for capability in WORKBENCH_COMMAND_CAPABILITIES:
        endpoint_key = capability.get("endpoint_url_key")
        payload = {
            **capability,
            "endpoint_url": endpoint_urls.get(str(endpoint_key)) if endpoint_key else None,
        }
        capabilities.append(MeshLibWorkbenchCommandCapability.model_validate(payload))
    return capabilities


def _official_parity_inventory() -> list[MeshLibOfficialParityFeature]:
    return [
        MeshLibOfficialParityFeature.model_validate(feature)
        for feature in OFFICIAL_PARITY_INVENTORY
    ]


@router.get("/versions/{version_id}", response_model=VersionDetailResponse)
async def get_version(version_id: str, db: Session = Depends(get_db)) -> VersionDetailResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = get_snapshot(db, version_id)
    return VersionDetailResponse(
        version=serialize_version(version),
        artifacts=[serialize_artifact(artifact) for artifact in get_version_artifacts(db, version_id)],
        latest_snapshot=serialize_snapshot(snapshot),
    )


@router.get("/versions/{version_id}/manuf", response_model=ManufacturabilitySnapshot)
async def get_manufacturability_snapshot(version_id: str, db: Session = Depends(get_db)) -> ManufacturabilitySnapshot:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = _snapshot_for_version_or_parent(db, version)
    if snapshot is None:
        raise HTTPException(status_code=404, detail="Manufacturability snapshot not found")
    return snapshot


@router.get("/versions/{version_id}/viewer", response_model=ViewerManifest)
async def get_viewer_manifest(version_id: str, db: Session = Depends(get_db)) -> ViewerManifest:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = _snapshot_for_version_or_parent(db, version)
    high = get_artifact_by_type(db, version_id, "preview_glb_high")
    low = get_artifact_by_type(db, version_id, "preview_glb_low")
    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    meshlib_scene_object = get_artifact_by_type(db, version_id, "meshlib_object_mesh_scene_json")
    meshlib_scene_mru = get_artifact_by_type(db, version_id, "meshlib_scene_mru")
    texture = get_artifact_by_type(db, version_id, "texture_image")
    texture_artifacts = _texture_artifacts_for_version(db, version_id)
    texture_per_face = _texture_per_face_from_artifacts(texture_artifacts)
    thickness = _artifact_by_type_or_parent(db, version, "analysis_thickness_npz")
    regions = _artifact_by_type_or_parent(db, version, "analysis_regions_json")
    if snapshot is None:
        raise HTTPException(status_code=404, detail="Snapshot not available")
    region_payload = _load_json_artifact(regions) or {}
    return ViewerManifest(
        version_id=version_id,
        preview_low_url=f"/api/artifacts/{low.id}" if low else None,
        preview_high_url=f"/api/artifacts/{high.id}" if high else None,
        normalized_mesh_url=f"/api/artifacts/{normalized.id}" if normalized else None,
        meshlib_scene_object_url=f"/api/artifacts/{meshlib_scene_object.id}" if meshlib_scene_object else None,
        meshlib_scene_object_metadata=meshlib_scene_object.metadata_json if meshlib_scene_object else {},
        meshlib_scene_mru_url=f"/api/artifacts/{meshlib_scene_mru.id}" if meshlib_scene_mru else None,
        meshlib_scene_mru_metadata=meshlib_scene_mru.metadata_json if meshlib_scene_mru else {},
        texture_artifact_url=f"/api/artifacts/{texture.id}" if texture else None,
        texture_metadata=texture.metadata_json if texture else {},
        texture_artifacts=texture_artifacts,
        texture_per_face=texture_per_face,
        thickness_artifact_url=f"/api/artifacts/{thickness.id}" if thickness else None,
        region_artifact_url=f"/api/artifacts/{regions.id}" if regions else None,
        bounding_box=snapshot.dimensions.bbox_mm,
        available_overlays=[item for item in ["thickness" if thickness else None, "regions" if regions else None] if item],
        region_manifest=snapshot.regions if snapshot.regions else region_payload.get("regions", []),
        measurements_summary=snapshot.dimensions.model_dump(mode="json"),
        needs_axis_confirmation=snapshot.dimensions.needs_axis_confirmation,
    )


@router.get("/versions/{version_id}/meshlib-workbench", response_model=MeshLibWorkbenchManifest)
async def get_meshlib_workbench_manifest(version_id: str, db: Session = Depends(get_db)) -> MeshLibWorkbenchManifest:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")

    high = get_artifact_by_type(db, version_id, "preview_glb_high")
    low = get_artifact_by_type(db, version_id, "preview_glb_low")
    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    meshlib_scene_object = get_artifact_by_type(db, version_id, "meshlib_object_mesh_scene_json")
    meshlib_scene_mru = get_artifact_by_type(db, version_id, "meshlib_scene_mru")
    texture = get_artifact_by_type(db, version_id, "texture_image")
    texture_artifacts = _texture_artifacts_for_version(db, version_id)
    texture_per_face = _texture_per_face_from_artifacts(texture_artifacts)
    return MeshLibWorkbenchManifest(
        version_id=version_id,
        entry_html_url="/meshlib-workbench/index.html",
        runtime_asset_base_url="/meshlib-workbench/runtime",
        normalized_mesh_url=f"/api/artifacts/{normalized.id}" if normalized else None,
        meshlib_scene_object_url=f"/api/artifacts/{meshlib_scene_object.id}" if meshlib_scene_object else None,
        meshlib_scene_object_metadata=meshlib_scene_object.metadata_json if meshlib_scene_object else {},
        meshlib_scene_mru_url=f"/api/artifacts/{meshlib_scene_mru.id}" if meshlib_scene_mru else None,
        meshlib_scene_mru_metadata=meshlib_scene_mru.metadata_json if meshlib_scene_mru else {},
        texture_artifact_url=f"/api/artifacts/{texture.id}" if texture else None,
        texture_metadata=texture.metadata_json if texture else {},
        texture_artifacts=texture_artifacts,
        texture_per_face=texture_per_face,
        preview_low_url=f"/api/artifacts/{low.id}" if low else None,
        preview_high_url=f"/api/artifacts/{high.id}" if high else None,
        commit_endpoint_url=f"/api/versions/{version_id}/interactive-commit",
        selection_endpoint_url=f"/api/versions/{version_id}/selection-commit",
        brush_endpoint_url=f"/api/versions/{version_id}/brush-replay",
        measurement_endpoint_url=f"/api/versions/{version_id}/measure-inspect",
        mesh_cut_measure_topology_endpoint_url=f"/api/versions/{version_id}/mesh-cut-measure/topology",
        built_in_ui=WORKBENCH_BUILT_IN_UI,
        interactive_tools=WORKBENCH_INTERACTIVE_TOOLS,
        command_capabilities=_workbench_command_capabilities(version, db=db),
        official_parity_inventory=_official_parity_inventory(),
        feature_flags={
            "supports_scene_tree": True,
            "supports_feature_search": True,
            "supports_toolbar": True,
            "supports_view_cube": True,
            "supports_scale_bar": True,
            "supports_workspace_commands": True,
            "supports_interactive_commit": True,
            "supports_selection_commit": True,
            "supports_brush_replay": True,
            "supports_measure_inspect": True,
        },
        notes=[
            "This endpoint describes the MeshLib workbench contract for the active version.",
            "The runtime is served from /public/meshlib-workbench/runtime and the frontend host loads this contract into the embedded MeshLib workbench.",
        ],
    )


@router.post("/versions/{version_id}/selection-commit", response_model=SelectionCommitResponse)
async def commit_selection(
    version_id: str,
    request: SelectionCommitRequest,
    db: Session = Depends(get_db),
) -> SelectionCommitResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for selection commits")
    if not _selection_has_content(request):
        raise HTTPException(status_code=400, detail="Selection commit requires selected vertices, faces, regions, or brush points")
    if _selection_targets_point_cloud(request):
        return _commit_point_cloud_selection(version_id, version, request, db)

    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if normalized is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")
    regions = get_artifact_by_type(db, version_id, "analysis_regions_json")
    region_payload = _load_json_artifact(regions)
    mesh = default_sdk.load_mesh(_materialize_artifact_to_path(normalized))
    if request.selection.region_ids and not _region_payload_has_ids(region_payload, request.selection.region_ids):
        region_payload = _detect_region_payload(mesh)
    resolved_face_ids, resolved_seed_vertex_ids = _resolve_selection_faces_and_seed_vertices(
        mesh, request.selection, region_payload
    )
    resolved_vertex_indices = list(resolved_seed_vertex_ids)
    for index in resolved_face_ids:
        resolved_vertex_indices.extend(int(vertex_index) for vertex_index in mesh.faces[index])
    resolved_vertex_ids = sorted(set(resolved_vertex_indices))
    counts = _selection_counts(request)
    resolved_counts = {"vertex_ids": len(resolved_vertex_ids)}
    if request.create_object:
        resolved_counts["face_ids"] = len(resolved_face_ids)
        if not resolved_face_ids:
            raise HTTPException(status_code=400, detail="Selection to Object requires selected mesh faces")
    payload = {
        "version_id": version_id,
        "tool_id": request.tool_id,
        "operation_label": request.operation_label,
        "label": request.label,
        "create_object": request.create_object,
        "selection": request.selection.model_dump(mode="json"),
        "selection_counts": counts,
        "resolved_vertex_ids": resolved_vertex_ids,
        "resolved_face_ids": resolved_face_ids,
        "resolved_counts": resolved_counts,
        "metadata": request.metadata,
    }
    selection_dir = settings.TEMP_DIR / "selection_commits" / version_id
    selection_dir.mkdir(parents=True, exist_ok=True)
    selection_path = selection_dir / "meshlib_selection.json"
    selection_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
    artifact = register_file_artifact(
        db,
        version_id,
        selection_path,
        "meshlib_selection_json",
        "application/json",
        metadata_json={
            "source": "meshlib_workbench",
            "tool_id": request.tool_id,
            "operation_label": request.operation_label,
            "label": request.label,
            "selection_counts": counts,
            "resolved_counts": resolved_counts,
            **request.metadata,
        },
    )
    selected_object_version = None
    selected_object_artifact = None
    selected_object_counts = None
    if request.create_object:
        selected_mesh = default_sdk.extract_selected_faces_as_mesh(mesh, resolved_face_ids)
        selected_object_version = create_version(
            db,
            model_id=version.model_id,
            parent_version_id=version.id,
            operation_type="selection_to_object",
            operation_label=request.operation_label,
            status="ready",
        )
        selected_object_path = selection_dir / "selection_object.ply"
        default_sdk.save_mesh(selected_mesh, selected_object_path, file_type="ply")
        selected_object_counts = {
            "vertex_ids": int(selected_mesh.vertex_count),
            "face_ids": int(selected_mesh.face_count),
        }
        selected_object_artifact = register_file_artifact(
            db,
            selected_object_version.id,
            selected_object_path,
            "normalized_mesh_ply",
            "model/ply",
            metadata_json={
                "source": "meshlib_selection_to_object",
                "tool_id": request.tool_id,
                "operation_label": request.operation_label,
                "label": request.label,
                "source_version_id": version.id,
                "source_selection_artifact_id": artifact.id,
                "selection_counts": counts,
                "resolved_counts": resolved_counts,
                "vertex_count": int(selected_mesh.vertex_count),
                "face_count": int(selected_mesh.face_count),
                **selected_mesh.metadata,
                **request.metadata,
            },
        )
    db.commit()
    db.refresh(artifact)
    if selected_object_artifact is not None:
        db.refresh(selected_object_artifact)
    return SelectionCommitResponse(
        version_id=version_id,
        artifact_id=artifact.id,
        artifact_url=f"/api/artifacts/{artifact.id}",
        selection_counts=counts,
        resolved_counts=resolved_counts,
        selected_object_version_id=selected_object_version.id if selected_object_version is not None else None,
        selected_object_artifact_id=selected_object_artifact.id if selected_object_artifact is not None else None,
        selected_object_artifact_url=(
            f"/api/artifacts/{selected_object_artifact.id}" if selected_object_artifact is not None else None
        ),
        selected_object_artifact_type=(
            selected_object_artifact.artifact_type if selected_object_artifact is not None else None
        ),
        selected_object_counts=selected_object_counts,
    )


@router.post("/versions/{version_id}/measure-inspect", response_model=MeasureInspectResponse)
async def measure_inspect(
    version_id: str,
    request: MeasureInspectRequest,
    db: Session = Depends(get_db),
) -> MeasureInspectResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if (
        not request.points
        and not request.point_pairs
        and not request.feature_pairs
        and not request.feature_refinements
        and not (request.features and request.include_feature_objects)
        and request.surface_distance is None
    ):
        raise HTTPException(
            status_code=400,
            detail="At least one point, point pair, feature pair, feature object, feature refinement, or surface distance seed is required",
        )

    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if normalized is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    mesh = default_sdk.load_mesh(_materialize_artifact_to_path(normalized))
    thickness_values: list[object] | None = None
    if request.include_local_thickness:
        thickness = get_artifact_by_type(db, version_id, "analysis_thickness_npz")
        if thickness is not None:
            payload = default_sdk.thickness_overlay_payload(_materialize_artifact_to_path(thickness))
            values = payload.get("values")
            if isinstance(values, list):
                thickness_values = values

    point_results: list[MeasureInspectPointResult] = []
    if request.points:
        closest_points, distances, face_indices = default_sdk.closest_points_on_mesh(request.points, mesh)
        for query_point, closest_point, distance, face_index in zip(request.points, closest_points, distances, face_indices, strict=True):
            face_index_int = int(face_index)
            point_results.append(
                MeasureInspectPointResult(
                    query_point=_point_tuple(query_point),
                    closest_point=_point_tuple(closest_point),
                    face_index=face_index_int,
                    distance_to_surface_mm=float(distance),
                    local_thickness_mm=_face_average_scalar(mesh, face_index_int, thickness_values),
                )
            )

    pair_results: list[MeasureInspectPairResult] = []
    for pair in request.point_pairs:
        start = _point_tuple(pair.start)
        end = _point_tuple(pair.end)
        midpoint = (
            (start[0] + end[0]) / 2.0,
            (start[1] + end[1]) / 2.0,
            (start[2] + end[2]) / 2.0,
        )
        if pair.metric == "geodesic":
            control_vertices = [int(index) for index in pair.control_vertices]
            if control_vertices and len(control_vertices) < 2:
                raise HTTPException(status_code=400, detail="Geodesic control vertex path requires at least two controls")
            surface_path_refinement: dict[str, object] | None = None
            cut_contours: object | None = None
            tri_point_fields = {
                "start_face_index": pair.start_face_index,
                "start_barycentric": pair.start_barycentric,
                "end_face_index": pair.end_face_index,
                "end_barycentric": pair.end_barycentric,
            }
            has_tri_point_path = any(value is not None for value in tri_point_fields.values())
            if has_tri_point_path:
                missing = [name for name, value in tri_point_fields.items() if value is None]
                if missing:
                    raise HTTPException(
                        status_code=400,
                        detail=f"MeshTriPoint geodesic path requires {', '.join(missing)}",
                    )
                if control_vertices:
                    raise HTTPException(
                        status_code=400,
                        detail="MeshTriPoint geodesic path cannot also specify control vertices",
                    )
                if pair.close_path:
                    raise HTTPException(
                        status_code=400,
                        detail="MeshTriPoint geodesic path does not support close_path",
                    )
                fast_path = default_sdk.mesh_fast_marching_surface_path_tri_points(
                    mesh,
                    start_face_index=int(pair.start_face_index),
                    start_barycentric=tuple(float(value) for value in pair.start_barycentric),
                    end_face_index=int(pair.end_face_index),
                    end_barycentric=tuple(float(value) for value in pair.end_barycentric),
                )
                fast_points = [_point_tuple(fast_path["start_point"])]
                fast_points.extend(_point_tuple(point) for point in fast_path.get("points", []))
                if fast_path.get("reached_face_index") == fast_path.get("end_face_index"):
                    fast_points.append(_point_tuple(fast_path["end_point"]))
                segment_lengths = [float(length) for length in fast_path.get("segment_lengths", [])]
                path = {
                    "length_mm": float(fast_path["length_mm"]),
                    "vertex_indices": [],
                    "points": fast_points,
                    "point_normals": [],
                    "edge_lengths": segment_lengths,
                    "leg_lengths": [],
                    "leg_vertex_offsets": [],
                    "line_segments": len(segment_lengths),
                    "closed_path": False,
                    "meshlib_reference": str(fast_path["meshlib_reference"]),
                }
                surface_path_refinement = {
                    "start_face_index": int(fast_path["start_face_index"]),
                    "start_barycentric": [
                        float(value)
                        for value in fast_path["start_barycentric"]
                    ],
                    "start_point": _point_tuple(fast_path["start_point"]),
                    "end_face_index": int(fast_path["end_face_index"]),
                    "end_barycentric": [
                        float(value)
                        for value in fast_path["end_barycentric"]
                    ],
                    "end_point": _point_tuple(fast_path["end_point"]),
                    "edges": [
                        [int(index) for index in edge]
                        for edge in fast_path.get("edges", [])
                    ],
                    "positions": [
                        float(position)
                        for position in fast_path.get("positions", [])
                    ],
                    "points": [
                        _point_tuple(point)
                        for point in fast_path.get("points", [])
                    ],
                    "segment_lengths_mm": segment_lengths,
                    "length_mm": float(fast_path["length_mm"]),
                    "reached_face_index": (
                        None
                        if fast_path.get("reached_face_index") is None
                        else int(fast_path["reached_face_index"])
                    ),
                    "stopped_reason": str(fast_path["stopped_reason"]),
                    "steps": int(fast_path["steps"]),
                    "meshlib_reference": str(fast_path["meshlib_reference"]),
                }
            elif control_vertices:
                cut_control_vertices = control_vertices
                path = default_sdk.mesh_geodesic_polyline_path(
                    mesh,
                    control_vertices=control_vertices,
                    close_path=pair.close_path,
                    max_path_len_mm=pair.geodesic_max_path_len_mm,
                )
                cut_contours = default_sdk.mesh_cut_measure_contours(
                    mesh,
                    control_vertices=cut_control_vertices,
                    close_path=pair.close_path,
                    max_path_len_mm=pair.geodesic_max_path_len_mm,
                )
            else:
                start_vertex = pair.start_vertex if pair.start_vertex is not None else _nearest_mesh_vertex(mesh.vertices, start)
                end_vertex = pair.end_vertex if pair.end_vertex is not None else _nearest_mesh_vertex(mesh.vertices, end)
                cut_control_vertices = [int(start_vertex), int(end_vertex)]
                path = default_sdk.mesh_geodesic_path(
                    mesh,
                    start_vertex=start_vertex,
                    end_vertex=end_vertex,
                    max_path_len_mm=pair.geodesic_max_path_len_mm,
                )
                if pair.include_refined_surface_path:
                    try:
                        refined_path = default_sdk.mesh_geodesic_quadrangle_path(
                            mesh,
                            start_vertex=start_vertex,
                            end_vertex=end_vertex,
                        )
                    except (RuntimeError, ValueError):
                        refined_path = None
                    if refined_path is not None:
                        surface_path_refinement = {
                            "start_vertex": int(refined_path["start_vertex"]),
                            "end_vertex": int(refined_path["end_vertex"]),
                            "start_face_index": int(refined_path["start_face_index"]),
                            "end_face_index": int(refined_path["end_face_index"]),
                            "shared_edge": [int(index) for index in refined_path["shared_edge"]],
                            "crossing_t": float(refined_path["crossing_t"]),
                            "crossing_point": _point_tuple(refined_path["crossing_point"]),
                            "points": [_point_tuple(point) for point in refined_path["points"]],
                            "edge_lengths_mm": [
                                float(length)
                                for length in refined_path["edge_lengths"]
                            ],
                            "length_mm": float(refined_path["length_mm"]),
                            "graph_vertex_indices": [
                                int(index)
                                for index in refined_path["graph_vertex_indices"]
                            ],
                            "graph_length_mm": float(refined_path["graph_length_mm"]),
                            "unfolded_quadrangle_convex": bool(
                                refined_path["unfolded_quadrangle_convex"]
                            ),
                            "meshlib_reference": str(refined_path["meshlib_reference"]),
                        }
                cut_contours = default_sdk.mesh_cut_measure_contours(
                    mesh,
                    control_vertices=cut_control_vertices,
                    close_path=pair.close_path,
                    max_path_len_mm=pair.geodesic_max_path_len_mm,
                )
            pair_results.append(
                MeasureInspectPairResult(
                    start=start,
                    end=end,
                    distance_mm=float(path["length_mm"]),
                    midpoint=midpoint,
                    label=pair.label,
                    metric="geodesic",
                    control_vertex_indices=[int(index) for index in path.get("control_vertex_indices", [])],
                    control_vertex_offsets=[int(index) for index in path.get("control_vertex_offsets", [])],
                    path_vertex_indices=[int(index) for index in path["vertex_indices"]],
                    path_points=[_point_tuple(point) for point in path["points"]],
                    path_point_normals=[_point_tuple(normal) for normal in path.get("point_normals", [])],
                    edge_lengths_mm=[float(length) for length in path.get("edge_lengths", [])],
                    leg_lengths_mm=[float(length) for length in path.get("leg_lengths", [])],
                    leg_vertex_offsets=[int(index) for index in path.get("leg_vertex_offsets", [])],
                    line_segments=int(path["line_segments"]),
                    closed_path=bool(path.get("closed_path", False)),
                    path_object_lines=_path_object_lines_payload(path["points"]),
                    path_object_points=_path_object_points_payload(path["points"], path.get("point_normals", [])),
                    cut_contours=_path_cut_contours_payload(cut_contours),
                    surface_path_refinement=surface_path_refinement,
                    meshlib_reference=str(path["meshlib_reference"]),
                )
            )
        else:
            pair_results.append(
                MeasureInspectPairResult(
                    start=start,
                    end=end,
                    distance_mm=_distance_mm(start, end),
                    midpoint=midpoint,
                    label=pair.label,
                    metric="euclidean",
                    path_points=[start, end],
                    line_segments=0 if start == end else 1,
                )
            )

    feature_index_by_id: dict[str, int] = {}
    sdk_features: list[dict[str, object]] = []
    if request.feature_pairs or request.feature_refinements or (request.features and request.include_feature_objects):
        feature_index_by_id = {feature.feature_id: index for index, feature in enumerate(request.features)}
        if len(feature_index_by_id) != len(request.features):
            raise HTTPException(status_code=400, detail="Feature primitive ids must be unique")
        sdk_features = [_sdk_feature_payload(feature) for feature in request.features]

    feature_object_results: list[MeasureInspectFeatureObjectResult] = []
    if request.features and request.include_feature_objects:
        try:
            feature_object_results = [
                _feature_object_result(descriptor)
                for descriptor in default_sdk.feature_object_descriptors(
                    sdk_features,
                    infinite_extent_mm=request.feature_object_infinite_extent_mm,
                )
            ]
        except (RuntimeError, ValueError) as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc

    feature_pair_results: list[MeasureInspectFeaturePairResult] = []
    if request.feature_pairs:
        pair_indices: list[tuple[int, int]] = []
        for feature_pair in request.feature_pairs:
            if feature_pair.first_feature_id not in feature_index_by_id:
                raise HTTPException(status_code=400, detail=f"Unknown feature id {feature_pair.first_feature_id!r}")
            if feature_pair.second_feature_id not in feature_index_by_id:
                raise HTTPException(status_code=400, detail=f"Unknown feature id {feature_pair.second_feature_id!r}")
            pair_indices.append(
                (
                    feature_index_by_id[feature_pair.first_feature_id],
                    feature_index_by_id[feature_pair.second_feature_id],
                )
            )
        for feature_pair, measurement in zip(
            request.feature_pairs,
            default_sdk.feature_pair_measurements(sdk_features, pair_indices),
            strict=True,
        ):
            feature_pair_results.append(
                MeasureInspectFeaturePairResult(
                    first_feature_id=str(measurement["first_feature_id"]),
                    second_feature_id=str(measurement["second_feature_id"]),
                    first_kind=str(measurement["first_kind"]),
                    second_kind=str(measurement["second_kind"]),
                    label=feature_pair.label,
                    distance=_feature_distance_result(measurement["distance"]),
                    center_distance=_feature_distance_result(measurement["center_distance"]),
                    angle=_feature_angle_result(measurement["angle"]),
                    intersections=[
                        _feature_intersection_result(intersection)
                        for intersection in measurement.get("intersections", [])
                    ],
                    meshlib_reference=str(measurement["meshlib_reference"]),
                )
            )

    feature_refinement_results: list[MeasureInspectFeatureRefinementResult] = []
    if request.feature_refinements:
        for refinement_request in request.feature_refinements:
            if refinement_request.feature_id not in feature_index_by_id:
                raise HTTPException(status_code=400, detail=f"Unknown feature id {refinement_request.feature_id!r}")
            feature_index = feature_index_by_id[refinement_request.feature_id]
            try:
                refinements = default_sdk.refine_feature_primitives(
                    mesh,
                    [sdk_features[feature_index]],
                    distance_limit_mm=refinement_request.distance_limit_mm,
                    normal_tolerance_degrees=refinement_request.normal_tolerance_degrees,
                    max_iterations=refinement_request.max_iterations,
                )
            except (RuntimeError, ValueError) as exc:
                raise HTTPException(status_code=400, detail=str(exc)) from exc
            if not refinements:
                raise HTTPException(status_code=500, detail="Feature refinement did not return a result")
            feature_refinement_results.append(
                _feature_refinement_result(
                    refinements[0],
                    label=refinement_request.label,
                    infinite_extent_mm=request.feature_object_infinite_extent_mm,
                )
            )

    surface_distance_result: MeasureInspectSurfaceDistanceResult | None = None
    if request.surface_distance is not None:
        surface_request = request.surface_distance
        seed_vertices = [int(seed) for seed in surface_request.seed_vertices]
        seed_edges = [(int(edge[0]), int(edge[1])) for edge in surface_request.seed_edges]
        seed_face_ids = [int(face_id) for face_id in surface_request.seed_face_ids]
        seed_point = _point_tuple(surface_request.seed) if surface_request.seed is not None else None
        seed_vertex = surface_request.seed_vertex
        if seed_vertex is None and seed_vertices:
            seed_vertex = seed_vertices[0]
        if seed_vertex is None and seed_point is not None:
            seed_vertex = _nearest_mesh_vertex(mesh.vertices, seed_point)
        if seed_vertex is not None:
            seed_vertices.append(int(seed_vertex))
        if not seed_vertices and not seed_edges and not seed_face_ids:
            raise HTTPException(
                status_code=400,
                detail="Surface distance requires a seed point, seed vertex, selected edge, or selected face boundary",
            )
        surface_sources = default_sdk.mesh_surface_distance_seed_vertices(
            mesh,
            seed_vertices=seed_vertices,
            seed_edges=seed_edges,
            seed_face_ids=seed_face_ids,
        )
        seed_vertices = [int(index) for index in surface_sources["seed_vertices"]]
        if seed_vertex is None:
            seed_vertex = seed_vertices[0]
        field = (
            default_sdk.mesh_geodesic_iso_region(
                mesh,
                seed_vertices=seed_vertices,
                iso_value_mm=surface_request.iso_value_mm,
                max_distance_mm=surface_request.max_distance_mm,
            )
            if surface_request.iso_value_mm is not None
            else default_sdk.mesh_geodesic_distance_field(
                mesh,
                seed_vertices=seed_vertices,
                max_distance_mm=surface_request.max_distance_mm,
            )
        )
        ridge_edges: list[tuple[int, int]] = []
        gorge_edges: list[tuple[int, int]] = []
        if surface_request.include_extreme_edges:
            surface_extreme_scalars = [
                float("inf") if distance is None else float(distance)
                for distance in field["distances_mm"]
            ]
            ridge_edges = [
                (int(edge[0]), int(edge[1]))
                for edge in default_sdk.mesh_geodesic_extreme_edges(
                    mesh,
                    scalars=surface_extreme_scalars,
                    extreme_type="ridge",
                )["edge_indices"]
            ]
            gorge_edges = [
                (int(edge[0]), int(edge[1]))
                for edge in default_sdk.mesh_geodesic_extreme_edges(
                    mesh,
                    scalars=surface_extreme_scalars,
                    extreme_type="gorge",
                )["edge_indices"]
            ]
        surface_distance_result = MeasureInspectSurfaceDistanceResult(
            seed=seed_point,
            seed_vertex=int(seed_vertex),
            seed_vertices=[int(index) for index in field["seed_vertices"]],
            seed_edges=[(int(edge[0]), int(edge[1])) for edge in surface_sources["selected_edges"]],
            seed_face_ids=[int(index) for index in surface_sources["selected_face_indices"]],
            seed_face_boundary_edges=[
                (int(edge[0]), int(edge[1])) for edge in surface_sources["selected_face_boundary_edges"]
            ],
            distances_mm=field["distances_mm"] if surface_request.include_distances else [],
            predecessor_vertices=field["predecessor_vertices"] if surface_request.include_distances else [],
            reachable_vertex_count=int(field["reachable_vertex_count"]),
            max_distance_mm=float(field["max_distance_mm"]),
            iso_value_mm=field.get("iso_value_mm"),
            selected_vertex_indices=[int(index) for index in field.get("selected_vertex_indices", [])],
            selected_face_indices=[int(index) for index in field.get("selected_face_indices", [])],
            crossing_face_indices=[int(index) for index in field.get("crossing_face_indices", [])],
            boundary_edges=[(int(edge[0]), int(edge[1])) for edge in field.get("boundary_edges", [])],
            iso_segments=(
                [
                    (_point_tuple(segment[0]), _point_tuple(segment[1]))
                    for segment in field.get("iso_segments", [])
                ]
                if surface_request.include_iso_segments
                else []
            ),
            ridge_edges=ridge_edges,
            gorge_edges=gorge_edges,
            clipped_vertices=[_point_tuple(point) for point in field.get("clipped_vertices", [])],
            clipped_faces=[
                (int(face[0]), int(face[1]), int(face[2]))
                for face in field.get("clipped_faces", [])
            ],
            clipped_source_face_indices=[
                int(index)
                for index in field.get("clipped_source_face_indices", [])
            ],
            clipped_source_vertex_indices=[
                int(index) if index is not None else None
                for index in field.get("clipped_source_vertex_indices", [])
            ],
            meshlib_reference=str(field["meshlib_reference"]),
        )

    return MeasureInspectResponse(
        version_id=version_id,
        points=point_results,
        point_pairs=pair_results,
        feature_pairs=feature_pair_results,
        feature_objects=feature_object_results,
        feature_refinements=feature_refinement_results,
        surface_distance=surface_distance_result,
    )


@router.post("/versions/{version_id}/mesh-cut-measure/topology", response_model=MeshCutMeasureTopologyResponse)
async def run_mesh_cut_measure_topology_for_version(
    version_id: str,
    request: MeshCutMeasureTopologyRequest,
    db: Session = Depends(get_db),
) -> MeshCutMeasureTopologyResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for Mesh Cut & Measure")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        result = default_sdk.mesh_cut_measure_edge_path_topology_cut(
            source_mesh,
            control_vertices=request.control_vertices,
            close_path=request.close_path,
            max_path_len_mm=request.max_path_len_mm,
        )
        output_mesh = result["mesh"]
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    if int(output_mesh.vertex_count) <= 0 or int(output_mesh.face_count) <= 0:
        raise HTTPException(
            status_code=400,
            detail="Mesh Cut & Measure Topology produced an empty mesh (0 vertices, 0 faces). Choose a valid path on the mesh.",
        )

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="mesh_cut_measure",
        operation_label=request.operation_label or "Mesh Cut & Measure Topology",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "mesh_cut_measure" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_mesh_cut_measure_topology.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    metadata_json = {
        "source": "rust_mesh_cut_measure_edge_path_topology_cut",
        "meshlib_reference": result["meshlib_reference"],
        "meshlib_source": "MeshLib/source/MRMesh/MROneMeshContours.*; MeshLib/source/MRMesh/MRContoursCut.*",
        "meshlib_contract": "MR::convertSurfacePathsToMeshContours -> MR::cutMesh",
        "rust_backed": True,
        "parity_scope": "edge-aligned surface path seam subset",
        "source_version_id": version.id,
        "control_vertices": [int(index) for index in request.control_vertices],
        "closed_path": bool(result["closed_path"]),
        "source_path_vertex_indices": [int(index) for index in result["source_path_vertex_indices"]],
        "result_cut_vertex_indices": [
            [int(index) for index in path] for path in result["result_cut_vertex_indices"]
        ],
        "duplicate_vertex_map": [
            [int(entry[0]), int(entry[1])] for entry in result["duplicate_vertex_map"]
        ],
        "cut_edge_pairs": [[int(entry[0]), int(entry[1])] for entry in result["cut_edge_pairs"]],
        "result_cut_edge_pairs": [
            [int(entry[0]), int(entry[1])] for entry in result["result_cut_edge_pairs"]
        ],
        "bad_face_indices": [int(index) for index in result["bad_face_indices"]],
        "length_mm": float(result["length_mm"]),
        "vertex_count": int(output_mesh.vertex_count),
        "face_count": int(output_mesh.face_count),
    }
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json=metadata_json,
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_mesh_cut_measure_topology_response(
        version.id,
        request,
        result,
        output_version,
        output_artifact,
    )


def _serialize_mesh_cut_measure_topology_response(
    source_version_id: str,
    request: MeshCutMeasureTopologyRequest,
    result: dict[str, object],
    version: ModelVersionRecord,
    artifact: ModelArtifactRecord,
) -> MeshCutMeasureTopologyResponse:
    output_mesh = result["mesh"]
    metadata = {
        "meshlib_reference": result["meshlib_reference"],
        "meshlib_source": "MeshLib/source/MRMesh/MROneMeshContours.*; MeshLib/source/MRMesh/MRContoursCut.*",
        "meshlib_contract": "MR::convertSurfacePathsToMeshContours -> MR::cutMesh",
        "rust_backed": True,
        "source": "rust_mesh_cut_measure_edge_path_topology_cut",
        "parity_scope": "edge-aligned surface path seam subset",
    }
    return MeshCutMeasureTopologyResponse(
        version=serialize_version(version),
        source_version_id=source_version_id,
        artifact_id=artifact.id,
        artifact_url=f"/api/artifacts/{artifact.id}",
        control_vertices=[int(index) for index in request.control_vertices],
        closed_path=bool(result["closed_path"]),
        length_mm=float(result["length_mm"]),
        output_vertex_count=int(output_mesh.vertex_count),
        output_face_count=int(output_mesh.face_count),
        duplicate_vertex_map=[
            [int(entry[0]), int(entry[1])] for entry in result["duplicate_vertex_map"]
        ],
        source_path_vertex_indices=[int(index) for index in result["source_path_vertex_indices"]],
        result_cut_vertex_indices=[
            [int(index) for index in path] for path in result["result_cut_vertex_indices"]
        ],
        cut_edge_pairs=[[int(entry[0]), int(entry[1])] for entry in result["cut_edge_pairs"]],
        result_cut_edge_pairs=[
            [int(entry[0]), int(entry[1])] for entry in result["result_cut_edge_pairs"]
        ],
        bad_face_indices=[int(index) for index in result["bad_face_indices"]],
        metadata=metadata,
    )


def _sdk_feature_payload(feature: object) -> dict[str, object]:
    return {
        "feature_id": feature.feature_id,
        "kind": feature.kind,
        "center": feature.center,
        "direction": feature.direction or feature.normal,
        "radius": feature.radius_mm,
        "length": feature.length_mm,
    }


def _feature_distance_result(payload: object) -> MeasureInspectFeatureDistanceResult:
    if not isinstance(payload, dict):
        raise HTTPException(status_code=500, detail="Feature measurement distance payload is malformed")
    return MeasureInspectFeatureDistanceResult(
        status=str(payload["status"]),
        distance_mm=float(payload["distance_mm"]) if payload.get("distance_mm") is not None else None,
        closest_point_a=_optional_point_tuple(payload.get("closest_point_a")),
        closest_point_b=_optional_point_tuple(payload.get("closest_point_b")),
    )


def _feature_angle_result(payload: object) -> MeasureInspectFeatureAngleResult:
    if not isinstance(payload, dict):
        raise HTTPException(status_code=500, detail="Feature measurement angle payload is malformed")
    return MeasureInspectFeatureAngleResult(
        status=str(payload["status"]),
        angle_radians=float(payload["angle_radians"]) if payload.get("angle_radians") is not None else None,
        angle_degrees=float(payload["angle_degrees"]) if payload.get("angle_degrees") is not None else None,
        point_a=_optional_point_tuple(payload.get("point_a")),
        point_b=_optional_point_tuple(payload.get("point_b")),
        direction_a=_optional_point_tuple(payload.get("direction_a")),
        direction_b=_optional_point_tuple(payload.get("direction_b")),
        is_surface_normal_a=bool(payload.get("is_surface_normal_a", False)),
        is_surface_normal_b=bool(payload.get("is_surface_normal_b", False)),
    )


def _feature_intersection_result(payload: object) -> MeasureInspectFeatureIntersectionResult:
    if not isinstance(payload, dict):
        raise HTTPException(status_code=500, detail="Feature measurement intersection payload is malformed")
    return MeasureInspectFeatureIntersectionResult(
        kind=str(payload["kind"]),
        center=_point_tuple(payload["center"]),
        direction=_optional_point_tuple(payload.get("direction")),
        radius_mm=float(payload["radius_mm"]) if payload.get("radius_mm") is not None else None,
        length_mm=float(payload["length_mm"]) if payload.get("length_mm") is not None else None,
        start_point=_optional_point_tuple(payload.get("start_point")),
        end_point=_optional_point_tuple(payload.get("end_point")),
        meshlib_primitive=str(payload["meshlib_primitive"]),
    )


def _feature_object_result(payload: object) -> MeasureInspectFeatureObjectResult:
    if not isinstance(payload, dict):
        raise HTTPException(status_code=500, detail="Feature object descriptor payload is malformed")
    return MeasureInspectFeatureObjectResult(
        feature_id=str(payload["feature_id"]),
        source_kind=str(payload["source_kind"]),
        object_type=str(payload["object_type"]),
        class_name=str(payload["class_name"]),
        class_name_plural=str(payload["class_name_plural"]),
        shared_properties=[
            _feature_object_property_result(property_payload)
            for property_payload in payload.get("shared_properties", [])
        ],
        meshlib_reference=str(payload["meshlib_reference"]),
    )


def _feature_object_property_result(payload: object) -> MeasureInspectFeatureObjectPropertyResult:
    if not isinstance(payload, dict):
        raise HTTPException(status_code=500, detail="Feature object property payload is malformed")
    return MeasureInspectFeatureObjectPropertyResult(
        name=str(payload["name"]),
        kind=str(payload["kind"]),
        scalar_value=float(payload["scalar_value"]) if payload.get("scalar_value") is not None else None,
        vector_value=_optional_point_tuple(payload.get("vector_value")),
    )


def _feature_refinement_result(
    payload: object,
    *,
    label: str | None = None,
    infinite_extent_mm: float = 1000.0,
) -> MeasureInspectFeatureRefinementResult:
    if not isinstance(payload, dict):
        raise HTTPException(status_code=500, detail="Feature refinement payload is malformed")
    primitive = payload.get("primitive")
    if not isinstance(primitive, dict):
        raise HTTPException(status_code=500, detail="Feature refinement primitive payload is malformed")
    try:
        feature_object_payload = default_sdk.feature_object_descriptors(
            [primitive],
            infinite_extent_mm=infinite_extent_mm,
        )[0]
    except (RuntimeError, ValueError, IndexError) as exc:
        raise HTTPException(status_code=500, detail=f"Feature refinement object descriptor failed: {exc}") from exc
    return MeasureInspectFeatureRefinementResult(
        feature_id=str(payload["feature_id"]),
        kind=str(payload["kind"]),
        label=label,
        center=_point_tuple(primitive["center"]),
        direction=_optional_point_tuple(primitive.get("direction")),
        radius_mm=float(primitive.get("radius_mm", primitive.get("radius", 0.0)) or 0.0),
        length_mm=float(primitive.get("length_mm", primitive.get("length", 0.0)) or 0.0),
        selected_vertex_indices=[int(index) for index in payload.get("selected_vertex_indices", [])],
        selected_count=int(payload["selected_count"]),
        iterations=int(payload["iterations"]),
        converged=bool(payload["converged"]),
        feature_object=_feature_object_result(feature_object_payload),
        meshlib_reference=str(payload["meshlib_reference"]),
    )


def _optional_point_tuple(point: object) -> tuple[float, float, float] | None:
    return _point_tuple(point) if point is not None else None


def _path_object_lines_payload(path_points: object) -> dict[str, object] | None:
    points = [_point_tuple(point) for point in path_points] if isinstance(path_points, list) else []
    if len(points) < 2:
        return None
    lines = default_sdk.object_lines_from_contours(
        [points],
        line_width=1.0,
        show_points=1,
        smooth_connections=0,
    )
    payload = lines.to_meshlib_json()
    payload["Name"] = "Mesh Cut & Measure Path"
    payload["MeshLibReference"] = "MR::ObjectLinesHolder / Polyline export"
    return payload


def _path_object_points_payload(path_points: object, path_normals: object) -> dict[str, object] | None:
    points = [_point_tuple(point) for point in path_points] if isinstance(path_points, list) else []
    normals = [_point_tuple(normal) for normal in path_normals] if isinstance(path_normals, list) else []
    if len(points) < 1 or len(points) != len(normals):
        return None
    return {
        "Type": ["PointsHolder", "ObjectPoints"],
        "Name": "Mesh Cut & Measure Path Points",
        "MeshLibReference": "MR::ObjectPointsHolder / PointCloud export",
        "PointCloud": {
            "Points": [[float(coord) for coord in point] for point in points],
            "Normals": [[float(coord) for coord in normal] for normal in normals],
            "ValidPoints": list(range(len(points))),
        },
    }


def _path_cut_contours_payload(cut_contours: object) -> dict[str, object] | None:
    if not isinstance(cut_contours, dict):
        return None
    contours_payload: list[dict[str, object]] = []
    for contour in cut_contours.get("contours", []):
        if not isinstance(contour, dict):
            continue
        intersections_payload: list[dict[str, object]] = []
        for intersection in contour.get("intersections", []):
            if not isinstance(intersection, dict):
                continue
            intersections_payload.append(
                {
                    "PrimitiveType": str(intersection.get("primitive_type", "")),
                    "PrimitiveId": int(intersection.get("primitive_id", -1)),
                    "Coordinate": [
                        float(coord)
                        for coord in _point_tuple(intersection.get("coordinate", (0.0, 0.0, 0.0)))
                    ],
                }
            )
        contours_payload.append(
            {
                "Closed": bool(contour.get("closed", False)),
                "Intersections": intersections_payload,
            }
        )
    return {
        "Type": ["OneMeshContours", "CutMeshInput"],
        "Name": "Mesh Cut & Measure Cut Contours",
        "MeshLibReference": str(
            cut_contours.get("meshlib_reference", "MR::convertSurfacePathsToMeshContours / MR::cutMesh")
        ),
        "ClosedPath": bool(cut_contours.get("closed_path", False)),
        "ContourCount": int(cut_contours.get("contour_count", len(contours_payload))),
        "CutResultCount": int(cut_contours.get("cut_result_count", 0)),
        "PivotIndices": [int(index) for index in cut_contours.get("pivot_indices", [])],
        "ResultCutVertexIndices": [
            [int(index) for index in path]
            for path in cut_contours.get("result_cut_vertex_indices", [])
        ],
        "BadFaceIndices": [int(index) for index in cut_contours.get("bad_face_indices", [])],
        "Contours": contours_payload,
    }


def _serialize_gcode_paths(version_id: str, document) -> GcodeParsePathsResponse:  # noqa: ANN001
    return GcodeParsePathsResponse(
        version_id=version_id,
        frame_count=document.frame_count,
        command_count=document.command_count,
        segment_count=document.segment_count,
        max_feedrate=document.max_feedrate,
        unit=document.unit,
        segments=document.segments.tolist(),
        tool_directions=document.tool_directions.tolist(),
        source_frame_indices=[int(index) for index in document.source_frame_indices.tolist()],
        idle=[bool(value) for value in document.idle.tolist()],
        feedrates=[float(value) for value in document.feedrates.tolist()],
        warnings=list(document.warnings),
        metadata=dict(document.metadata),
    )


def _gcode_temp_path(version_id: str, file_name: str, purpose: str) -> Path:
    safe_name = Path(file_name or "program.gcode").name
    if not safe_name or safe_name in {".", ".."}:
        safe_name = "program.gcode"
    workdir = settings.TEMP_DIR / "gcode" / version_id / purpose
    workdir.mkdir(parents=True, exist_ok=True)
    return workdir / safe_name


def _materialize_gcode_source(version_id: str, request: GcodeLoadSourceRequest, purpose: str) -> Path:
    path = _gcode_temp_path(version_id, request.file_name, purpose)
    path.write_text(request.source, encoding="utf-8")
    return path


def _distance_map_tiff_temp_path(version_id: str, file_name: str, purpose: str) -> Path:
    safe_name = Path(file_name or "distance-map.tiff").name
    if not safe_name or safe_name in {".", ".."}:
        safe_name = "distance-map.tiff"
    if Path(safe_name).suffix.lower() not in {".tif", ".tiff"}:
        safe_name = f"{safe_name}.tiff"
    workdir = settings.TEMP_DIR / "distance-map-tiff" / version_id / purpose
    workdir.mkdir(parents=True, exist_ok=True)
    return workdir / safe_name


def _materialize_distance_map_tiff_payload(
    version_id: str,
    request: DistanceMapTiffImportRequest,
    purpose: str,
) -> Path:
    try:
        contents = base64.b64decode(request.contents_base64, validate=True)
    except (binascii.Error, ValueError) as exc:
        raise HTTPException(status_code=400, detail="Invalid TIFF base64 payload") from exc
    if not contents:
        raise HTTPException(status_code=400, detail="TIFF payload is empty")
    path = _distance_map_tiff_temp_path(version_id, request.file_name, purpose)
    path.write_bytes(contents)
    return path


def _serialize_gcode_source_response(
    version_id: str,
    file_name: str,
    source_frames: list[str],
    *,
    sdk_operation: str,
    meshlib_reference: str,
) -> GcodeSourceResponse:
    return GcodeSourceResponse(
        version_id=version_id,
        file_name=Path(file_name or "program.gcode").name,
        frame_count=len(source_frames),
        source_frames=[str(frame) for frame in source_frames],
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": meshlib_reference,
            "meshlib_source": "MeshLib/source/MRMesh/MRGcodeLoad.*",
        },
    )


def _serialize_point_cloud_icp_response(
    version_id: str,
    result,
    *,
    sdk_operation: str,
) -> PointCloudIcpResponse:  # noqa: ANN001
    return PointCloudIcpResponse(
        version_id=version_id,
        method=result.method,
        mode=result.mode,
        rotation=[[float(value) for value in row] for row in result.rotation.tolist()],
        translation=tuple(float(value) for value in result.translation.tolist()),
        transform=[[float(value) for value in row] for row in result.transform.tolist()],
        iterations=int(result.iterations),
        mean_square_distance=float(result.mean_square_distance),
        active_pair_count=int(result.active_pair_count),
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::ICP::calculateTransformation",
            "meshlib_source": "MeshLib/source/MRMesh/MRICP.*",
        },
    )


def _serialize_point_cloud_triangulation_response(
    version_id: str,
    stage: str,
    mesh,
    *,
    sdk_operation: str,
) -> PointCloudTriangulationResponse:  # noqa: ANN001
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::triangulatePointCloud",
            "meshlib_source": "MeshLib/source/MRMesh/MRPointCloudTriangulation.*",
        }
    )
    return PointCloudTriangulationResponse(
        version_id=version_id,
        stage=stage,
        vertices=[
            tuple(float(value) for value in vertex)
            for vertex in mesh.vertices.tolist()
        ],
        faces=[
            tuple(int(value) for value in face)
            for face in mesh.faces.tolist()
        ],
        vertex_count=int(mesh.vertex_count),
        face_count=int(mesh.face_count),
        metadata=metadata,
    )


def _serialize_point_cloud_multiway_icp_response(
    version_id: str,
    request: PointCloudMultiwayIcpRequest,
    result,
    *,
    sdk_operation: str,
) -> PointCloudMultiwayIcpResponse:  # noqa: ANN001
    return PointCloudMultiwayIcpResponse(
        version_id=version_id,
        method=request.method,
        grouping=request.grouping,
        mode=request.mode,
        transforms=[
            {
                "rotation": [
                    [float(value) for value in row]
                    for row in transform.rotation.tolist()
                ],
                "translation": tuple(float(value) for value in transform.translation.tolist()),
                "transform": [
                    [float(value) for value in row]
                    for row in transform.transform.tolist()
                ],
            }
            for transform in result.transforms
        ],
        iterations=int(result.iterations),
        mean_square_distance=float(result.mean_square_distance),
        active_pair_count=int(result.active_pair_count),
        fixed_object_index=int(result.fixed_object_index),
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::ICPGroupPair::calculateTransformation",
            "meshlib_source": "MeshLib/source/MRMesh/MRICP.*",
        },
    )


def _serialize_offset_contours_response(
    version_id: str,
    contours,
    *,
    sdk_operation: str,
    origins: list | None = None,
) -> OffsetContoursResponse:  # noqa: ANN001
    serialized_contours = [
        [tuple(float(value) for value in point) for point in contour]
        for contour in contours
    ]
    return OffsetContoursResponse(
        version_id=version_id,
        contour_count=len(serialized_contours),
        point_count=sum(len(contour) for contour in serialized_contours),
        contours=serialized_contours,
        origins=list(origins or []),
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::offsetContours",
            "meshlib_source": "MeshLib/source/MRMesh/MROffsetContours.*",
        },
    )


def _serialize_distance_map_response(
    version_id: str,
    distance_map,
    *,
    sdk_operation: str,
    meshlib_reference: str = "MR::Cuda::distanceMapFromContours / MR::DistanceMap",
    meshlib_source: str = "MeshLib/source/MRCuda/MRCudaContoursDistanceMap.*; MeshLib/source/MRMesh/MRDistanceMap.*",
) -> DistanceMapResponse:  # noqa: ANN001
    return DistanceMapResponse(
        version_id=version_id,
        width=int(distance_map.width),
        height=int(distance_map.height),
        origin=tuple(float(value) for value in distance_map.origin),
        pixel_size=tuple(float(value) for value in distance_map.pixel_size),
        valid_count=int(distance_map.valid_count),
        min_value=float(distance_map.min_value),
        max_value=float(distance_map.max_value),
        values=[
            [float(value) for value in row]
            for row in distance_map.values.tolist()
        ],
        model_transform=(
            None
            if distance_map.model_transform is None
            else [float(value) for value in distance_map.model_transform]
        ),
        unit=distance_map.unit,
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": meshlib_reference,
            "meshlib_source": meshlib_source,
        },
    )


def _serialize_iso_line_segments_response(
    version_id: str,
    iso_segments,
    *,
    sdk_operation: str,
) -> IsoLineSegmentsResponse:  # noqa: ANN001
    return IsoLineSegmentsResponse(
        version_id=version_id,
        iso_value=float(iso_segments.iso_value),
        segment_count=int(iso_segments.segment_count),
        segments=[
            (
                (float(segment[0][0]), float(segment[0][1])),
                (float(segment[1][0]), float(segment[1][1])),
            )
            for segment in iso_segments.segments.tolist()
        ],
        unit=iso_segments.unit,
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::distanceMapTo2DIsoPolyline",
            "meshlib_source": "MeshLib/source/MRMesh/MRDistanceMap.*",
        },
    )


def _ensure_distance_maps_coregistered(left, right) -> None:  # noqa: ANN001
    """distance_map_merge aligns the two maps by PIXEL INDEX, so they must share the
    same world grid: identical origin and pixel_size. Differing extent (width/height)
    is fine — MeshLib overlays the common region — but a differing origin or pixel
    size means the maps occupy different world space and merging them by pixel index
    silently blends regions that don't overlap. Refuse that mismatch.
    """

    def _matches(a, b) -> bool:  # noqa: ANN001 - tolerant element compare; None == untracked
        if a is None or b is None:
            return True
        a_seq, b_seq = list(a), list(b)
        if len(a_seq) != len(b_seq):
            return False
        return all(abs(float(x) - float(y)) <= 1e-6 + 1e-6 * abs(float(y)) for x, y in zip(a_seq, b_seq))

    if not _matches(left.pixel_size, right.pixel_size):
        raise HTTPException(
            status_code=400,
            detail=(
                f"Distance maps must share pixel_size to merge (got {left.pixel_size} vs "
                f"{right.pixel_size}); merge aligns by pixel index, not world coordinates."
            ),
        )
    if not _matches(left.origin, right.origin):
        raise HTTPException(
            status_code=400,
            detail=(
                f"Distance maps must share origin to merge (got {left.origin} vs {right.origin}); "
                "merge aligns by pixel index, not world coordinates."
            ),
        )


def _distance_map_document_from_payload(payload) -> DistanceMapDocument:  # noqa: ANN001
    values = _flatten_numeric_values(payload.values)
    return DistanceMapDocument(
        width=payload.width,
        height=payload.height,
        origin=payload.origin,
        pixel_size=payload.pixel_size,
        values=payload.values,
        valid_count=int(payload.valid_count if payload.valid_count is not None else len(values)),
        min_value=float(payload.min_value if payload.min_value is not None else _min_numeric_value(values)),
        max_value=float(payload.max_value if payload.max_value is not None else _max_numeric_value(values)),
        model_transform=payload.model_transform,
        unit=payload.unit,
    )


def _flatten_numeric_values(values: object) -> list[float]:
    if hasattr(values, "reshape") and hasattr(values, "tolist"):
        values = values.reshape(-1).tolist()
    elif hasattr(values, "tolist"):
        values = values.tolist()
    if isinstance(values, (list, tuple)):
        flattened: list[float] = []
        for value in values:
            flattened.extend(_flatten_numeric_values(value))
        return flattened
    return [float(values)]


def _min_numeric_value(values: object) -> float:
    flattened = _flatten_numeric_values(values)
    return min(flattened) if flattened else 0.0


def _max_numeric_value(values: object) -> float:
    flattened = _flatten_numeric_values(values)
    return max(flattened) if flattened else 0.0


def _mesh_response_geometry(mesh) -> tuple[  # noqa: ANN001
    list[tuple[float, float, float]],
    list[tuple[int, int, int]],
    tuple[float, float, float],
    tuple[float, float, float],
]:
    raw_vertices = mesh.vertices.tolist() if hasattr(mesh.vertices, "tolist") else mesh.vertices
    raw_faces = mesh.faces.tolist() if hasattr(mesh.faces, "tolist") else mesh.faces
    vertices = [tuple(float(axis) for axis in vertex) for vertex in raw_vertices]
    faces = [tuple(int(axis) for axis in face) for face in raw_faces]
    if vertices:
        bounds_min = tuple(min(vertex[axis] for vertex in vertices) for axis in range(3))
        bounds_max = tuple(max(vertex[axis] for vertex in vertices) for axis in range(3))
    else:
        bounds_min = (0.0, 0.0, 0.0)
        bounds_max = (0.0, 0.0, 0.0)
    return vertices, faces, bounds_min, bounds_max


def _serialize_object_lines_response(
    version_id: str,
    document,
    *,
    sdk_operation: str,
) -> ObjectLinesResponse:  # noqa: ANN001
    object_lines = document.to_meshlib_json()
    return ObjectLinesResponse(
        version_id=version_id,
        point_count=int(len(document.points)),
        line_count=int(len(document.lines)),
        line_width=float(document.line_width),
        object_lines=object_lines,
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::ObjectLines / MR::PolylineTopology",
            "meshlib_source": "MeshLib/source/MRMesh/MRObjectLines.*; MeshLib/source/MRMesh/MRObjectLinesHolder.*; MeshLib/source/MRMesh/MRPolylineTopology.*",
        },
    )


def _serialize_object_lines_contours_response(
    version_id: str,
    contours,
    *,
    sdk_operation: str,
) -> ObjectLinesToContoursResponse:  # noqa: ANN001
    serialized_contours = [
        [tuple(float(value) for value in point) for point in contour]
        for contour in contours
    ]
    return ObjectLinesToContoursResponse(
        version_id=version_id,
        contour_count=len(serialized_contours),
        point_count=sum(len(contour) for contour in serialized_contours),
        contours=serialized_contours,
        metadata={
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": "MR::ObjectLines / MR::PolylineTopology",
            "meshlib_source": "MeshLib/source/MRMesh/MRObjectLines.*; MeshLib/source/MRMesh/MRObjectLinesHolder.*; MeshLib/source/MRMesh/MRPolylineTopology.*",
        },
    )


def _object_lines_text_file_name(file_name: str, default: str) -> str:
    safe_name = Path(file_name or default).name
    if not safe_name or safe_name in {".", ".."}:
        return default
    return safe_name


def _decode_object_lines_binary_payload(contents_base64: str) -> bytes:
    try:
        return base64.b64decode(contents_base64, validate=True)
    except (binascii.Error, ValueError) as exc:
        raise HTTPException(status_code=400, detail="Invalid base64 ObjectLines payload") from exc


def _decode_voxel_binary_payload(contents_base64: str, *, label: str) -> bytes:
    try:
        return base64.b64decode(contents_base64, validate=True)
    except (binascii.Error, ValueError) as exc:
        raise HTTPException(status_code=400, detail=f"Invalid base64 {label} payload") from exc


def _voxel_safe_file_name(file_name: str, default: str) -> str:
    safe_name = Path(file_name or default).name
    return safe_name or default


def _serialize_voxel_volume_load_response(
    version_id: str,
    volume,
    *,
    sdk_operation: str,
    meshlib_reference: str,
    meshlib_source: str,
    extra_metadata: dict[str, object] | None = None,
) -> VoxelVolumeLoadResponse:  # noqa: ANN001
    values = _flatten_numeric_values(volume.values)
    metadata = dict(getattr(volume, "metadata", {}) or {})
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": sdk_operation,
            "meshlib_reference": meshlib_reference,
            "meshlib_source": meshlib_source,
        }
    )
    if extra_metadata:
        metadata.update(extra_metadata)
    default_iso_value = metadata.get("default_iso_value")
    return VoxelVolumeLoadResponse(
        version_id=version_id,
        dimensions=tuple(int(value) for value in volume.dimensions),
        voxel_size=tuple(float(value) for value in volume.voxel_size),
        grid_level_set=bool(volume.grid_level_set),
        scalar_type=str(volume.scalar_type),
        value_count=len(values),
        values=values,
        min_value=float(volume.min_value),
        max_value=float(volume.max_value),
        default_iso_value=float(default_iso_value) if default_iso_value is not None else None,
        metadata=metadata,
    )


def _serialize_mesh_to_voxels_sdf(
    version_id: str,
    request: MeshToVoxelsSdfRequest,
    grid,
    occupancy,
    estimated_volume: float,
    surface_mesh,
    *,
    meshlib_reference: str,
) -> MeshToVoxelsSdfResponse:  # noqa: ANN001
    values = grid.values.reshape(-1)
    return MeshToVoxelsSdfResponse(
        version_id=version_id,
        mode=request.mode,
        voxel_size_mm=float(request.voxel_size_mm),
        surface_offset_voxels=float(request.surface_offset_voxels),
        padding_mm=float(request.surface_offset_voxels * request.voxel_size_mm),
        iso_value=float(request.iso_value),
        origin=tuple(float(value) for value in grid.origin),
        shape=tuple(int(value) for value in grid.shape),
        value_count=int(values.size),
        active_voxel_count=int(occupancy.sum()),
        min_value=float(values.min()) if values.size else 0.0,
        max_value=float(values.max()) if values.size else 0.0,
        estimated_volume_mm3=float(estimated_volume),
        surface_vertex_count=int(surface_mesh.vertex_count) if surface_mesh is not None else 0,
        surface_face_count=int(surface_mesh.face_count) if surface_mesh is not None else 0,
        metadata={
            "rust_backed": True,
            "meshlib_reference": meshlib_reference,
            "meshlib_source": "MeshLib/source/MRVoxels/MRVDBConversions.*",
            "surface_extracted": bool(surface_mesh is not None),
            "value_semantics": "signed_distance_mm" if request.mode == "signed" else "unsigned_distance_mm",
        },
    )


def _serialize_collision_detection(
    version_id: str,
    other_version_id: str,
    result,
) -> CollisionDetectResponse:  # noqa: ANN001
    metadata = dict(result.metadata)
    metadata.update(
        {
            "meshlib_reference": "findCollidingTriangles",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshCollide.*",
            "rust_backed": True,
        }
    )
    return CollisionDetectResponse(
        version_id=version_id,
        other_version_id=other_version_id,
        colliding=result.colliding,
        pair_count=result.pair_count,
        first_face_indices=list(result.first_face_indices),
        second_face_indices=list(result.second_face_indices),
        pairs=[
            CollisionFacePair(
                first_face=pair.first_face,
                second_face=pair.second_face,
                intersection_count=pair.intersection_count,
            )
            for pair in result.pairs
        ],
        truncated=result.truncated,
        metadata=metadata,
    )


def _serialize_exact_boolean_response(
    source_version_id: str,
    other_version_id: str,
    request: ExactBooleanRequest,
    version: ModelVersionRecord,
    artifact: ModelArtifactRecord,
    result,
) -> ExactBooleanResponse:  # noqa: ANN001
    diagnostics = dict(result.diagnostics)
    mesh_stats = diagnostics.get("mesh_stats")
    if not isinstance(mesh_stats, dict):
        mesh_stats = {}
        diagnostics["mesh_stats"] = mesh_stats
    mesh_stats.setdefault("vertex_count", int(result.mesh.vertex_count))
    mesh_stats.setdefault("face_count", int(result.mesh.face_count))

    return ExactBooleanResponse(
        version=serialize_version(version),
        source_version_id=source_version_id,
        other_version_id=other_version_id,
        operation=request.operation,
        artifact_id=artifact.id,
        artifact_url=f"/api/artifacts/{artifact.id}",
        output_vertex_count=int(result.mesh.vertex_count),
        output_face_count=int(result.mesh.face_count),
        diagnostics=diagnostics,
        metadata={
            "meshlib_reference": "MR::boolean",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshBoolean.*",
            "rust_backed": True,
            "source": "rust_exact_boolean",
        },
    )


def _serialize_voxel_boolean_response(
    source_version_id: str,
    other_version_id: str,
    request: VoxelBooleanRequest,
    version: ModelVersionRecord,
    artifact: ModelArtifactRecord,
    mesh,
) -> VoxelBooleanResponse:  # noqa: ANN001
    return VoxelBooleanResponse(
        version=serialize_version(version),
        source_version_id=source_version_id,
        other_version_id=other_version_id,
        operation=request.operation,
        voxel_size_mm=float(request.voxel_size_mm),
        padding_mm=None
        if getattr(request, "padding_mm", None) is None
        else float(getattr(request, "padding_mm")),
        refine=bool(getattr(request, "refine", False)),
        artifact_id=artifact.id,
        artifact_url=f"/api/artifacts/{artifact.id}",
        output_vertex_count=int(mesh.vertex_count),
        output_face_count=int(mesh.face_count),
        metadata={
            "meshlib_reference": "MRVoxels::voxelBoolean",
            "meshlib_source": "MeshLib/source/MRVoxels/MRBoolean.*",
            "rust_backed": True,
            "source": "rust_voxel_boolean",
        },
    )


def _serialize_offset_shell_response(
    source_version_id: str,
    mode: str,
    request: OffsetMeshRequest
    | OffsetSmoothingRequest
    | ShellMeshRequest
    | ThickenMeshRequest
    | WeightedShellRequest
    | PartialOffsetRequest
    | OffsetVertsRequest,
    version: ModelVersionRecord,
    artifact: ModelArtifactRecord,
    mesh,
) -> OffsetShellMeshResponse:  # noqa: ANN001
    is_shell = mode == "shell"
    meshlib_reference = {
        "offset": "MR::generalOffsetMesh",
        "shell": "MR::generalOffsetMesh Shell Mode",
        "thicken": "MR::thickenMesh",
        "weighted_shell": "MR::WeightedShell::meshShell",
        "partial_offset": "MR::partialOffsetMesh",
        "offset_verts": "MR::offsetVerts",
        "expand_shrink": "MR::generalOffsetMesh Expand/Shrink Mode",
        "shrink_expand": "MR::generalOffsetMesh Shrink/Expand Mode",
    }[mode]
    metadata_source = {
        "offset": "rust_voxel_offset",
        "shell": "rust_voxel_shell",
        "thicken": "rust_voxel_thicken",
        "weighted_shell": "rust_voxel_weighted_shell",
        "partial_offset": "rust_voxel_partial_offset",
        "offset_verts": "rust_offset_verts",
        "expand_shrink": "rust_voxel_expand_shrink",
        "shrink_expand": "rust_voxel_shrink_expand",
    }[mode]
    meshlib_source = {
        "offset": "MeshLib/source/MRVoxels/MROffset.*",
        "shell": "MeshLib/source/MRVoxels/MROffset.*",
        "thicken": "MeshLib/source/MRVoxels/MROffset.*",
        "weighted_shell": "MeshLib/source/MRVoxels/MRWeightedPointsShell.*",
        "partial_offset": "MeshLib/source/MRVoxels/MRPartialOffset.*",
        "offset_verts": "MeshLib/source/MRMesh/MROffsetVerts.*",
        "expand_shrink": "MeshLib/source/MRVoxels/MROffset.*",
        "shrink_expand": "MeshLib/source/MRVoxels/MROffset.*",
    }[mode]
    region_weights = (
        {str(entry.region_id): float(entry.weight_mm) for entry in request.region_weights}
        if isinstance(request, WeightedShellRequest)
        else None
    )
    selected_region_ids = list(request.region_ids) if isinstance(request, PartialOffsetRequest | OffsetVertsRequest) else None
    return OffsetShellMeshResponse(
        version=serialize_version(version),
        source_version_id=source_version_id,
        mode=mode,
        offset_mm=float(request.offset_mm)
        if isinstance(request, OffsetMeshRequest | WeightedShellRequest | PartialOffsetRequest | OffsetVertsRequest)
        else None,
        distance_mm=float(request.distance_mm) if isinstance(request, OffsetSmoothingRequest) else None,
        wall_thickness_mm=float(request.wall_thickness_mm) if is_shell and isinstance(request, ShellMeshRequest) else None,
        thickness_mm=float(request.thickness_mm) if isinstance(request, ThickenMeshRequest) else None,
        region_weights=region_weights,
        selected_region_ids=selected_region_ids,
        voxel_size_mm=float(getattr(request, "voxel_size_mm", 0.0)),
        padding_mm=None
        if getattr(request, "padding_mm", None) is None
        else float(getattr(request, "padding_mm")),
        refine=bool(getattr(request, "refine", False)),
        artifact_id=artifact.id,
        artifact_url=f"/api/artifacts/{artifact.id}",
        output_vertex_count=int(mesh.vertex_count),
        output_face_count=int(mesh.face_count),
        metadata={
            "meshlib_reference": meshlib_reference,
            "meshlib_source": meshlib_source,
            "rust_backed": True,
            "source": metadata_source,
        },
    )


def _finalize_voxel_to_mesh_output(mesh, operation_label: str):  # noqa: ANN001
    """Orient faces outward (the dense iso-surface mesher can emit inverted winding,
    so the result reads as negative-volume) and reject an empty mesh with guidance.
    Feeding a signed distance field to the dense mesher without ``grid_level_set``
    collapses it; the Dual mesher handles dense volumes without fragmenting.
    """
    if int(getattr(mesh, "vertex_count", 0)) == 0 or int(getattr(mesh, "face_count", 0)) == 0:
        raise ValueError(
            f"{operation_label} produced an empty mesh. If the volume is a signed distance "
            "field, set grid_level_set=true; for dense density volumes the Dual mesher "
            "(voxel→mesh Dual) avoids the fragmentation/collapse the simple mesher hits."
        )
    return default_sdk.orient_faces_outward(mesh)


def _reject_shattered_voxel_mesh(mesh, operation_label: str):  # noqa: ANN001
    """A mask / segmentation meshing of a connected region should yield ~1 solid. A
    gross explosion into many components means the mask coordinate layout doesn't
    match the volume (or the seeds are scattered) — refuse rather than ship hundreds
    of disconnected blobs. The threshold scales with face count so legitimate
    multi-object volumes still pass; only a clear shatter is blocked.
    """
    try:
        components = int(default_sdk.stats(mesh).connected_components)
    except Exception:  # noqa: BLE001 - stats unavailable -> don't block
        return mesh
    faces = int(getattr(mesh, "face_count", 0))
    if components > max(64, faces // 2000):
        raise ValueError(
            f"{operation_label} fragmented into {components} disconnected components; a connected "
            "mask/segmentation should produce roughly one solid. Verify the mask coordinate layout "
            "matches the volume (x-fastest) or supply a smooth density volume."
        )
    return mesh


def _reject_sheet_thicken_on_closed_solid(mesh) -> None:  # noqa: ANN001
    """Sheet-thicken (MeshLib thickenMesh) offsets a surface and joins the offset
    copy to the original along its OPEN boundary to form a slab. On a CLOSED solid
    there is no boundary to join, so it degenerates into the original solid plus an
    interpenetrating offset copy (two components), not a thickened sheet. Refuse
    with guidance toward the right tool for a solid.
    """
    try:
        health = default_sdk.health(mesh)
        boundary_edges = int(getattr(health, "boundary_edge_count", 0))
    except Exception:  # noqa: BLE001 - if health is unavailable, don't block the op
        return
    if boundary_edges == 0:
        raise HTTPException(
            status_code=400,
            detail=(
                "Sheet-thicken applies to open surfaces; this mesh is closed (watertight), "
                "so it would produce the original solid plus an interpenetrating offset copy "
                "rather than a thickened sheet. Use Thicken (global) to add wall thickness to "
                "a solid, or Offset for an inward/outward shell."
            ),
        )


def _ensure_nonempty_offset_shell_mesh(mesh, operation_label: str, *, voxel_size_mm: float | None = None) -> None:  # noqa: ANN001
    vertex_count = int(getattr(mesh, "vertex_count", 0))
    face_count = int(getattr(mesh, "face_count", 0))
    if vertex_count > 0 and face_count > 0:
        return
    hint = (
        f" Reduce voxel_size_mm below {voxel_size_mm:g} for this model scale."
        if voxel_size_mm is not None
        else " Reduce voxel_size_mm for this model scale."
    )
    raise HTTPException(
        status_code=400,
        detail=f"{operation_label} produced an empty mesh ({vertex_count} vertices, {face_count} faces).{hint}",
    )


def _ensure_offset_shell_resolution(
    source_mesh,  # noqa: ANN001
    output_mesh,  # noqa: ANN001
    operation_label: str,
    *,
    voxel_size_mm: float | None = None,
    min_face_ratio: float = 0.1,
) -> None:
    voxel_hint = (
        f" Reduce voxel_size_mm below {voxel_size_mm:g}, or use Offset Verts for a topology-preserving jewelry offset."
        if voxel_size_mm is not None
        else " Reduce voxel_size_mm, or use Offset Verts for a topology-preserving jewelry offset."
    )
    # Geometry acceptance (closure / fragmentation on a watertight source) is
    # computed by the Rust mesh_quality kernel.
    try:
        geometry_failures = default_sdk.offset_shell_failures(source_mesh, output_mesh)
    except Exception:  # noqa: BLE001 - verdict needs real meshes; skip if unavailable
        geometry_failures = []
    if geometry_failures:
        raise HTTPException(status_code=400, detail=f"{operation_label} {geometry_failures[0]}.{voxel_hint}")

    # Scalar face-count ratio policy (operates on Rust-computed counts).
    source_face_count = int(getattr(source_mesh, "face_count", 0))
    output_face_count = int(getattr(output_mesh, "face_count", 0))
    if source_face_count >= 1_000 and output_face_count < int(source_face_count * min_face_ratio):
        raise HTTPException(
            status_code=400,
            detail=(
                f"{operation_label} produced a low-resolution voxel remesh "
                f"({output_face_count} faces from {source_face_count} source faces).{voxel_hint}"
            ),
        )


@router.post("/versions/{version_id}/gcode/parse-paths", response_model=GcodeParsePathsResponse)
async def parse_gcode_paths_for_version(
    version_id: str,
    request: GcodeParsePathsRequest,
    db: Session = Depends(get_db),
) -> GcodeParsePathsResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for G-code parsing")
    try:
        document = default_sdk.parse_gcode_paths(
            request.source,
            machine_settings=request.machine_settings,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    return _serialize_gcode_paths(version_id, document)


@router.post("/versions/{version_id}/gcode/load-source", response_model=GcodeSourceResponse)
async def load_gcode_source_for_version(
    version_id: str,
    request: GcodeLoadSourceRequest,
    db: Session = Depends(get_db),
) -> GcodeSourceResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for G-code source loading")
    try:
        source_path = _materialize_gcode_source(version_id, request, "load-source")
        frames = default_sdk.load_gcode_source(source_path)
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    return _serialize_gcode_source_response(
        version_id,
        request.file_name,
        frames,
        sdk_operation="load_gcode_source",
        meshlib_reference="GcodeLoad::fromAnySupportedFormat",
    )


@router.post("/versions/{version_id}/gcode/write-source", response_model=GcodeSourceResponse)
async def write_gcode_source_for_version(
    version_id: str,
    request: GcodeWriteSourceRequest,
    db: Session = Depends(get_db),
) -> GcodeSourceResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for G-code source writing")
    try:
        source_path = _gcode_temp_path(version_id, request.file_name, "write-source")
        default_sdk.write_gcode_source(request.source_frames, source_path)
        frames = default_sdk.load_gcode_source(source_path)
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    return _serialize_gcode_source_response(
        version_id,
        request.file_name,
        frames,
        sdk_operation="write_gcode_source",
        meshlib_reference="ObjectGcode source frames",
    )


@router.post("/versions/{version_id}/gcode/parse-file-paths", response_model=GcodeParsePathsResponse)
async def parse_gcode_file_paths_for_version(
    version_id: str,
    request: GcodeParseFilePathsRequest,
    db: Session = Depends(get_db),
) -> GcodeParsePathsResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for G-code file path parsing")
    try:
        source_path = _materialize_gcode_source(version_id, request, "parse-file-paths")
        document = default_sdk.parse_gcode_file_paths(
            source_path,
            machine_settings=request.machine_settings,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    response = _serialize_gcode_paths(version_id, document)
    response.metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "parse_gcode_file_paths",
            "meshlib_reference": "GcodeLoad::fromAnySupportedFormat + GcodeProcessor",
        }
    )
    return response


@router.post("/versions/{version_id}/point-cloud/icp", response_model=PointCloudIcpResponse)
async def run_point_cloud_icp_for_version(
    version_id: str,
    request: PointCloudIcpRequest,
    db: Session = Depends(get_db),
) -> PointCloudIcpResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for point-cloud ICP")

    try:
        floating = PointCloudDocument(request.floating_points)
        reference = PointCloudDocument(request.reference_points)
        if request.method == "point_to_plane":
            if request.reference_normals is None:
                raise ValueError("reference_normals are required for point_to_plane ICP")
            result = default_sdk.pairwise_point_to_plane_icp(
                floating,
                reference,
                request.reference_normals,
                max_iterations=request.max_iterations,
                tolerance=request.tolerance,
                mode=request.mode,
                floating_normals=None
                if request.floating_normals is None
                else request.floating_normals,
                max_pair_distance=request.max_pair_distance,
                cos_threshold=request.cos_threshold,
                far_dist_factor=request.far_dist_factor,
                mutual_closest=request.mutual_closest,
            )
            sdk_operation = "pairwise_point_to_plane_icp"
        else:
            result = default_sdk.pairwise_point_to_point_icp(
                floating,
                reference,
                max_iterations=request.max_iterations,
                tolerance=request.tolerance,
                mode=request.mode,
            )
            sdk_operation = "pairwise_point_to_point_icp"
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return _serialize_point_cloud_icp_response(version_id, result, sdk_operation=sdk_operation)


@router.post("/versions/{version_id}/point-cloud/triangulate", response_model=PointCloudTriangulationResponse)
async def run_point_cloud_triangulation_for_version(
    version_id: str,
    request: PointCloudTriangulationRequest,
    db: Session = Depends(get_db),
) -> PointCloudTriangulationResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for point-cloud triangulation")

    try:
        cloud = PointCloudDocument(request.points)
        kwargs = {
            "radius": request.radius,
            "num_neighbors": request.num_neighbors,
            "boundary_angle": request.boundary_angle,
            "max_removes": request.max_removes,
            "crit_angle": request.crit_angle,
            "normals": request.normals,
            "untrusted_indices": request.untrusted_indices,
        }
        if request.stage == "candidate":
            result = default_sdk.point_cloud_triangulate_candidate_mesh(cloud, **kwargs)
            sdk_operation = "point_cloud_triangulate_candidate_mesh"
        elif request.stage == "cleaned":
            result = default_sdk.point_cloud_triangulate_cleaned_candidate_mesh(cloud, **kwargs)
            sdk_operation = "point_cloud_triangulate_cleaned_candidate_mesh"
        elif request.stage == "topology":
            result = default_sdk.point_cloud_triangulate_topology_candidate_mesh(cloud, **kwargs)
            sdk_operation = "point_cloud_triangulate_topology_candidate_mesh"
        else:
            result = default_sdk.point_cloud_triangulate_filled_candidate_mesh(
                cloud,
                crit_hole_length=request.crit_hole_length,
                **kwargs,
            )
            sdk_operation = "point_cloud_triangulate_filled_candidate_mesh"
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return _serialize_point_cloud_triangulation_response(
        version_id,
        request.stage,
        result,
        sdk_operation=sdk_operation,
    )


@router.post("/versions/{version_id}/point-cloud/icp/multiway", response_model=PointCloudMultiwayIcpResponse)
async def run_point_cloud_multiway_icp_for_version(
    version_id: str,
    request: PointCloudMultiwayIcpRequest,
    db: Session = Depends(get_db),
) -> PointCloudMultiwayIcpResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for point-cloud multiway ICP")

    objects = [PointCloudDocument(points) for points in request.objects]
    normals = request.normals or []
    common_kwargs = {
        "max_iterations": request.max_iterations,
        "tolerance": request.tolerance,
        "mode": request.mode,
        "fixed_object_index": request.fixed_object_index,
    }

    try:
        if request.grouping == "independent":
            if request.method == "point_to_point":
                result = default_sdk.multiway_point_to_point_icp(objects, **common_kwargs)
                sdk_operation = "multiway_point_to_point_icp"
            elif request.method == "point_to_plane":
                result = default_sdk.multiway_point_to_plane_icp(objects, normals, **common_kwargs)
                sdk_operation = "multiway_point_to_plane_icp"
            else:
                result = default_sdk.multiway_combined_icp(objects, normals, **common_kwargs)
                sdk_operation = "multiway_combined_icp"
        elif request.grouping == "all_object":
            if request.method == "point_to_point":
                result = default_sdk.multiway_all_object_point_to_point_icp(objects, **common_kwargs)
                sdk_operation = "multiway_all_object_point_to_point_icp"
            elif request.method == "point_to_plane":
                result = default_sdk.multiway_all_object_point_to_plane_icp(objects, normals, **common_kwargs)
                sdk_operation = "multiway_all_object_point_to_plane_icp"
            else:
                result = default_sdk.multiway_all_object_combined_icp(objects, normals, **common_kwargs)
                sdk_operation = "multiway_all_object_combined_icp"
        elif request.grouping == "sequential_cascade":
            cascade_kwargs = {**common_kwargs, "max_group_size": request.max_group_size}
            if request.method == "point_to_point":
                result = default_sdk.multiway_sequential_cascade_point_to_point_icp(objects, **cascade_kwargs)
                sdk_operation = "multiway_sequential_cascade_point_to_point_icp"
            elif request.method == "point_to_plane":
                result = default_sdk.multiway_sequential_cascade_point_to_plane_icp(objects, normals, **cascade_kwargs)
                sdk_operation = "multiway_sequential_cascade_point_to_plane_icp"
            else:
                result = default_sdk.multiway_sequential_cascade_combined_icp(objects, normals, **cascade_kwargs)
                sdk_operation = "multiway_sequential_cascade_combined_icp"
        else:
            cascade_kwargs = {**common_kwargs, "max_group_size": request.max_group_size}
            if request.method == "point_to_point":
                result = default_sdk.multiway_aabb_cascade_point_to_point_icp(objects, **cascade_kwargs)
                sdk_operation = "multiway_aabb_cascade_point_to_point_icp"
            elif request.method == "point_to_plane":
                result = default_sdk.multiway_aabb_cascade_point_to_plane_icp(objects, normals, **cascade_kwargs)
                sdk_operation = "multiway_aabb_cascade_point_to_plane_icp"
            else:
                result = default_sdk.multiway_aabb_cascade_combined_icp(objects, normals, **cascade_kwargs)
                sdk_operation = "multiway_aabb_cascade_combined_icp"
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return _serialize_point_cloud_multiway_icp_response(
        version_id,
        request,
        result,
        sdk_operation=sdk_operation,
    )


@router.post("/versions/{version_id}/contours/offset", response_model=OffsetContoursResponse)
async def run_offset_contours_for_version(
    version_id: str,
    request: OffsetContoursRequest,
    db: Session = Depends(get_db),
) -> OffsetContoursResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for offset-contours")
    if request.offset is None and request.offsets is None:
        raise HTTPException(status_code=400, detail="offset_contours requires either offset or offsets")

    kwargs = {
        "offset": request.offset,
        "offsets": request.offsets,
        "min_angle_precision": request.min_angle_precision,
        "mode": request.mode,
        "end_type": request.end_type,
        "corner_type": request.corner_type,
        "max_sharp_angle": request.max_sharp_angle,
        "z_restore": request.z_restore,
        "z_value": request.z_value,
        "z_values": request.z_values,
        "relax_iterations": request.relax_iterations,
    }
    try:
        if request.include_origins:
            result = default_sdk.offset_contours_with_origins(request.contours, **kwargs)
            return _serialize_offset_contours_response(
                version_id,
                result["contours"],
                origins=list(result.get("origins") or []),
                sdk_operation="offset_contours_with_origins",
            )
        contours = default_sdk.offset_contours(request.contours, **kwargs)
        return _serialize_offset_contours_response(
            version_id,
            contours,
            sdk_operation="offset_contours",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/contours", response_model=DistanceMapResponse)
async def run_distance_map_from_contours_for_version(
    version_id: str,
    request: DistanceMapContoursRequest,
    db: Session = Depends(get_db),
) -> DistanceMapResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-contours")

    try:
        distance_map = default_sdk.distance_map_from_contours(
            request.contours,
            width=request.width,
            height=request.height,
            origin=request.origin,
            pixel_size=request.pixel_size,
            signed=request.signed,
        )
        return _serialize_distance_map_response(
            version_id,
            distance_map,
            sdk_operation="distance_map_from_contours",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/mesh", response_model=DistanceMapResponse)
async def run_distance_map_from_mesh_for_version(
    version_id: str,
    request: DistanceMapFromMeshRequest,
    db: Session = Depends(get_db),
) -> DistanceMapResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-from-mesh")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        distance_map = default_sdk.distance_map_from_mesh(
            mesh,
            width=request.width,
            height=request.height,
            origin=request.origin,
            x_range=request.x_range,
            y_range=request.y_range,
            direction=request.direction,
            epsilon=request.epsilon,
        )
        return _serialize_distance_map_response(
            version_id,
            distance_map,
            sdk_operation="distance_map_from_mesh",
            meshlib_reference="MR::computeDistanceMap / MR::MeshToDistanceMapParams",
            meshlib_source="MeshLib/source/MRMesh/MRDistanceMap.*; MeshLib/source/MRMesh/MRDistanceMapParams.*",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/iso-lines", response_model=IsoLineSegmentsResponse)
async def run_distance_map_iso_lines_for_version(
    version_id: str,
    request: DistanceMapIsoLinesRequest,
    db: Session = Depends(get_db),
) -> IsoLineSegmentsResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-iso-lines")

    try:
        distance_map = _distance_map_document_from_payload(request)
        iso_segments = default_sdk.distance_map_to_iso_segments(
            distance_map,
            iso_value=request.iso_value,
        )
        return _serialize_iso_line_segments_response(
            version_id,
            iso_segments,
            sdk_operation="distance_map_to_iso_segments",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/merge", response_model=DistanceMapResponse)
async def run_distance_map_merge_for_version(
    version_id: str,
    request: DistanceMapMergeRequest,
    db: Session = Depends(get_db),
) -> DistanceMapResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-merge")

    left = _distance_map_document_from_payload(request.left)
    right = _distance_map_document_from_payload(request.right)
    _ensure_distance_maps_coregistered(left, right)
    try:
        merged = default_sdk.distance_map_merge(left, right, mode=request.mode)
        return _serialize_distance_map_response(
            version_id,
            merged,
            sdk_operation="distance_map_merge",
            meshlib_reference="MR::DistanceMap::max/min/operator-",
            meshlib_source="MeshLib/source/MRMesh/MRDistanceMap.*",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/contour-boolean", response_model=IsoLineSegmentsResponse)
async def run_distance_map_contour_boolean_for_version(
    version_id: str,
    request: DistanceMapContourBooleanRequest,
    db: Session = Depends(get_db),
) -> IsoLineSegmentsResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-contour-boolean")

    try:
        iso_segments = default_sdk.distance_map_contour_boolean(
            request.contours_a,
            request.contours_b,
            mode=request.mode,
            width=request.width,
            height=request.height,
            origin=request.origin,
            pixel_size=request.pixel_size,
            iso_value=request.iso_value,
        )
        response = _serialize_iso_line_segments_response(
            version_id,
            iso_segments,
            sdk_operation="distance_map_contour_boolean",
        )
        response.metadata.update(
            {
                "mode": request.mode,
                "meshlib_reference": "MR::contourUnion / MR::contourIntersection / MR::contourSubtract",
                "meshlib_source": "MeshLib/source/MRMesh/MRDistanceMap.*",
            }
        )
        return response
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/from-tiff", response_model=DistanceMapResponse)
async def run_distance_map_from_tiff_for_version(
    version_id: str,
    request: DistanceMapTiffImportRequest,
    db: Session = Depends(get_db),
) -> DistanceMapResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-from-tiff")

    try:
        source_path = _materialize_distance_map_tiff_payload(version_id, request, "from-tiff")
        distance_map = default_sdk.distance_map_from_tiff(source_path)
        return _serialize_distance_map_response(
            version_id,
            distance_map,
            sdk_operation="distance_map_from_tiff",
            meshlib_reference="MR::DistanceMapLoad::fromTiff",
            meshlib_source="MeshLib/source/MRMesh/MRDistanceMapLoad.*; MeshLib/source/MRIOExtras/MRTiff.*",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/distance-map/to-tiff", response_model=DistanceMapTiffExportResponse)
async def run_distance_map_to_tiff_for_version(
    version_id: str,
    request: DistanceMapTiffExportRequest,
    db: Session = Depends(get_db),
) -> DistanceMapTiffExportResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for distance-map-to-tiff")

    try:
        distance_map = _distance_map_document_from_payload(request)
        output_path = _distance_map_tiff_temp_path(version_id, request.file_name, "to-tiff")
        default_sdk.distance_map_to_tiff(distance_map, output_path)
        contents = output_path.read_bytes()
        return DistanceMapTiffExportResponse(
            version_id=version_id,
            file_name=output_path.name,
            byte_count=len(contents),
            contents_base64=base64.b64encode(contents).decode("ascii"),
            metadata={
                "rust_backed": True,
                "sdk_operation": "distance_map_to_tiff",
                "meshlib_reference": "MR::DistanceMapSave::toTiff",
                "meshlib_source": "MeshLib/source/MRMesh/MRDistanceMapSave.*; MeshLib/source/MRIOExtras/MRTiff.*",
            },
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/from-contours", response_model=ObjectLinesResponse)
async def run_object_lines_from_contours_for_version(
    version_id: str,
    request: ObjectLinesFromContoursRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-from-contours")

    try:
        document = default_sdk.object_lines_from_contours(
            request.contours,
            line_width=request.line_width,
            show_points=request.show_points,
            smooth_connections=request.smooth_connections,
        )
        return _serialize_object_lines_response(
            version_id,
            document,
            sdk_operation="object_lines_from_contours",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/to-contours", response_model=ObjectLinesToContoursResponse)
async def run_object_lines_to_contours_for_version(
    version_id: str,
    request: ObjectLinesToContoursRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesToContoursResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-to-contours")

    try:
        contours = default_sdk.object_lines_to_contours(request.object_lines)
        return _serialize_object_lines_contours_response(
            version_id,
            contours,
            sdk_operation="object_lines_to_contours",
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/load-pts", response_model=ObjectLinesResponse)
async def run_object_lines_load_pts_for_version(
    version_id: str,
    request: ObjectLinesPtsLoadRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-load-pts")

    try:
        document = default_sdk.object_lines_from_pts(request.source)
        response = _serialize_object_lines_response(
            version_id,
            document,
            sdk_operation="object_lines_from_pts",
        )
        response.metadata.update(
            {
                "meshlib_reference": "MR::LinesLoad::fromPts",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesLoad.*",
                "file_name": _object_lines_text_file_name(request.file_name, "object-lines.pts"),
            }
        )
        return response
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/load-mrlines", response_model=ObjectLinesResponse)
async def run_object_lines_load_mrlines_for_version(
    version_id: str,
    request: ObjectLinesBinaryLoadRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-load-mrlines")

    try:
        document = default_sdk.object_lines_from_mrlines(
            _decode_object_lines_binary_payload(request.contents_base64)
        )
        response = _serialize_object_lines_response(
            version_id,
            document,
            sdk_operation="object_lines_from_mrlines",
        )
        response.metadata.update(
            {
                "meshlib_reference": "MR::LinesLoad::fromMrLines",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesLoad.*",
                "file_name": _object_lines_text_file_name(request.file_name, "object-lines.mrlines"),
            }
        )
        return response
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/load-ply", response_model=ObjectLinesResponse)
async def run_object_lines_load_ply_for_version(
    version_id: str,
    request: ObjectLinesBinaryLoadRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-load-ply")

    try:
        document = default_sdk.object_lines_from_ply(
            _decode_object_lines_binary_payload(request.contents_base64)
        )
        response = _serialize_object_lines_response(
            version_id,
            document,
            sdk_operation="object_lines_from_ply",
        )
        response.metadata.update(
            {
                "meshlib_reference": "MR::LinesLoad::fromPly",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesLoad.*",
                "file_name": _object_lines_text_file_name(request.file_name, "object-lines.ply"),
            }
        )
        return response
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/save-pts", response_model=ObjectLinesTextExportResponse)
async def run_object_lines_save_pts_for_version(
    version_id: str,
    request: ObjectLinesTextExportRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesTextExportResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-save-pts")

    try:
        source = default_sdk.object_lines_to_pts(request.object_lines)
        return ObjectLinesTextExportResponse(
            version_id=version_id,
            file_name=_object_lines_text_file_name(request.file_name, "object-lines.pts"),
            source=source,
            byte_count=len(source.encode("utf-8")),
            metadata={
                "rust_backed": True,
                "sdk_operation": "object_lines_to_pts",
                "meshlib_reference": "MR::LinesSave::toPts",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesSave.*",
            },
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/save-mrlines", response_model=ObjectLinesBinaryExportResponse)
async def run_object_lines_save_mrlines_for_version(
    version_id: str,
    request: ObjectLinesBinaryExportRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesBinaryExportResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-save-mrlines")

    try:
        payload = default_sdk.object_lines_to_mrlines(request.object_lines)
        return ObjectLinesBinaryExportResponse(
            version_id=version_id,
            file_name=_object_lines_text_file_name(request.file_name, "object-lines.mrlines"),
            byte_count=len(payload),
            contents_base64=base64.b64encode(payload).decode("ascii"),
            metadata={
                "rust_backed": True,
                "sdk_operation": "object_lines_to_mrlines",
                "meshlib_reference": "MR::LinesSave::toMrLines",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesSave.*",
            },
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/save-ply", response_model=ObjectLinesBinaryExportResponse)
async def run_object_lines_save_ply_for_version(
    version_id: str,
    request: ObjectLinesBinaryExportRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesBinaryExportResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-save-ply")

    try:
        payload = default_sdk.object_lines_to_ply(request.object_lines)
        return ObjectLinesBinaryExportResponse(
            version_id=version_id,
            file_name=_object_lines_text_file_name(request.file_name, "object-lines.ply"),
            byte_count=len(payload),
            contents_base64=base64.b64encode(payload).decode("ascii"),
            metadata={
                "rust_backed": True,
                "sdk_operation": "object_lines_to_ply",
                "meshlib_reference": "MR::LinesSave::toPly",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesSave.*",
            },
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/load-svg", response_model=ObjectLinesResponse)
async def run_object_lines_load_svg_for_version(
    version_id: str,
    request: ObjectLinesSvgLoadRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-load-svg")

    try:
        document = default_sdk.object_lines_from_svg(request.source)
        response = _serialize_object_lines_response(
            version_id,
            document,
            sdk_operation="object_lines_from_svg",
        )
        response.metadata.update(
            {
                "meshlib_reference": "MR::LinesLoad::fromSvg",
                "meshlib_source": "MeshLib/source/MRIOExtras/MRSvg.*",
                "file_name": _object_lines_text_file_name(request.file_name, "object-lines.svg"),
            }
        )
        return response
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/object-lines/save-dxf", response_model=ObjectLinesTextExportResponse)
async def run_object_lines_save_dxf_for_version(
    version_id: str,
    request: ObjectLinesTextExportRequest,
    db: Session = Depends(get_db),
) -> ObjectLinesTextExportResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for object-lines-save-dxf")

    try:
        source = default_sdk.object_lines_to_dxf(request.object_lines)
        return ObjectLinesTextExportResponse(
            version_id=version_id,
            file_name=_object_lines_text_file_name(request.file_name, "object-lines.dxf"),
            source=source,
            byte_count=len(source.encode("utf-8")),
            metadata={
                "rust_backed": True,
                "sdk_operation": "object_lines_to_dxf",
                "meshlib_reference": "MR::LinesSave::toDxf",
                "meshlib_source": "MeshLib/source/MRMesh/MRLinesSave.*",
            },
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/collision/detect", response_model=CollisionDetectResponse)
async def detect_collision_for_version(
    version_id: str,
    request: CollisionDetectRequest,
    db: Session = Depends(get_db),
) -> CollisionDetectResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    other_version = db.get(ModelVersionRecord, request.other_version_id)
    if other_version is None:
        raise HTTPException(status_code=404, detail="Other version not found")
    if version.status != "ready" or other_version.status != "ready":
        raise HTTPException(status_code=409, detail="Both versions must be ready for collision detection")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    target_artifact = get_artifact_by_type(db, request.other_version_id, "normalized_mesh_ply")
    if source_artifact is None or target_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        target_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(target_artifact))
        result = default_sdk.exact_mesh_intersections(
            source_mesh,
            target_mesh,
            epsilon=request.epsilon,
            first_intersection_only=request.first_intersection_only,
            max_pairs=request.max_pairs,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return _serialize_collision_detection(version_id, request.other_version_id, result)


@router.post("/versions/{version_id}/offset/voxel", response_model=OffsetShellMeshResponse)
async def run_offset_mesh_for_version(
    version_id: str,
    request: OffsetMeshRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for offset")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    operation_label = f"Offset Mesh {request.offset_mm:g} mm"
    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        output_mesh = await run_in_threadpool(default_sdk.voxel_offset_mesh,
            source_mesh,
            offset_mm=request.offset_mm,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            refine=request.refine,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, operation_label, voxel_size_mm=request.voxel_size_mm)
    _ensure_offset_shell_resolution(source_mesh, output_mesh, operation_label, voxel_size_mm=request.voxel_size_mm)

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="offset",
        operation_label=operation_label,
        status="ready",
    )
    workdir = settings.TEMP_DIR / "offset_mesh" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_offset_{request.offset_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_voxel_offset",
            "meshlib_reference": "MR::generalOffsetMesh",
            "meshlib_source": "MeshLib/source/MRVoxels/MROffset.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "offset_mm": request.offset_mm,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        "offset",
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


def _run_offset_smoothing_sequence(
    source_mesh,  # noqa: ANN001
    request: OffsetSmoothingRequest,
    offset_sequence: list[float],
):
    output_mesh = source_mesh
    for offset_mm in offset_sequence:
        step_input = output_mesh
        output_mesh = default_sdk.voxel_offset_mesh(
            step_input,
            offset_mm=offset_mm,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            refine=request.refine,
        )
        if offset_mm < 0:
            # An inward (shrink) step on a feature thinner than the offset distance
            # shatters the mesh into disconnected fragments. The following outward
            # (expand) step then re-merges the surviving blobs into one watertight
            # component, so the post-sequence output check can no longer see the
            # damage — the geometry is already destroyed (e.g. a sub-mm snake band
            # fragmenting into 100+ pieces under a 0.25 mm shrink, rounded back into
            # one watertight but mangled blob). Catch it on the intermediate via the
            # Rust quality kernel's fragmentation verdict. A few boundary edges from
            # marching cubes at the thinnest spots are fine (the expand heals them),
            # so only the fragmentation clause is treated as fatal here.
            try:
                step_failures = default_sdk.offset_shell_failures(step_input, output_mesh)
            except Exception:  # noqa: BLE001 - verdict needs real meshes; skip if unavailable
                step_failures = []
            fragmented = next((f for f in step_failures if "fragment" in f.lower()), None)
            if fragmented:
                raise ValueError(
                    f"the inward {abs(offset_mm):g} mm step {fragmented} — the feature is thinner "
                    "than the offset distance. Use a smaller distance, a finer voxel size, or "
                    "Offset Verts for a topology-preserving offset."
                )
    return output_mesh


async def _run_offset_smoothing_for_version(
    version_id: str,
    request: OffsetSmoothingRequest,
    db: Session,
    *,
    mode: str,
    operation_label: str,
    offset_sequence: list[float],
    metadata_source: str,
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail=f"Version is not ready for {operation_label.lower()}")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        output_mesh = await run_in_threadpool(_run_offset_smoothing_sequence, source_mesh, request, offset_sequence)
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, operation_label, voxel_size_mm=request.voxel_size_mm)
    _ensure_offset_shell_resolution(source_mesh, output_mesh, operation_label, voxel_size_mm=request.voxel_size_mm)

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type=mode,
        operation_label=f"{operation_label} {request.distance_mm:g} mm",
        status="ready",
    )
    workdir = settings.TEMP_DIR / mode / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_{mode}_{request.distance_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": metadata_source,
            "meshlib_reference": f"MR::generalOffsetMesh {operation_label} Mode",
            "meshlib_source": "MeshLib/source/MRVoxels/MROffset.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "distance_mm": request.distance_mm,
            "offset_sequence_mm": offset_sequence,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        mode,
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


@router.post("/versions/{version_id}/offset/expand-shrink", response_model=OffsetShellMeshResponse)
async def run_expand_shrink_for_version(
    version_id: str,
    request: OffsetSmoothingRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    return await _run_offset_smoothing_for_version(
        version_id,
        request,
        db,
        mode="expand_shrink",
        operation_label="Expand/Shrink",
        offset_sequence=[request.distance_mm, -request.distance_mm],
        metadata_source="rust_voxel_expand_shrink",
    )


@router.post("/versions/{version_id}/offset/shrink-expand", response_model=OffsetShellMeshResponse)
async def run_shrink_expand_for_version(
    version_id: str,
    request: OffsetSmoothingRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    return await _run_offset_smoothing_for_version(
        version_id,
        request,
        db,
        mode="shrink_expand",
        operation_label="Shrink/Expand",
        offset_sequence=[-request.distance_mm, request.distance_mm],
        metadata_source="rust_voxel_shrink_expand",
    )


@router.post("/versions/{version_id}/shell/voxel", response_model=OffsetShellMeshResponse)
async def run_shell_mesh_for_version(
    version_id: str,
    request: ShellMeshRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for shell")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        output_mesh = await run_in_threadpool(default_sdk.voxel_shell_mesh,
            source_mesh,
            wall_thickness_mm=request.wall_thickness_mm,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            refine=request.refine,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, "Shell Mesh", voxel_size_mm=request.voxel_size_mm)
    _ensure_offset_shell_resolution(source_mesh, output_mesh, "Shell Mesh", voxel_size_mm=request.voxel_size_mm)

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="shell",
        operation_label=f"Shell Mesh {request.wall_thickness_mm:g} mm",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "shell_mesh" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_shell_{request.wall_thickness_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_voxel_shell",
            "meshlib_reference": "MR::generalOffsetMesh Shell Mode",
            "meshlib_source": "MeshLib/source/MRVoxels/MROffset.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "wall_thickness_mm": request.wall_thickness_mm,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        "shell",
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


@router.post("/versions/{version_id}/offset/thicken", response_model=OffsetShellMeshResponse)
async def run_thicken_mesh_for_version(
    version_id: str,
    request: ThickenMeshRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for thickening")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        _reject_sheet_thicken_on_closed_solid(source_mesh)
        output_mesh = default_sdk.voxel_thicken_mesh(
            source_mesh,
            thickness_mm=request.thickness_mm,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            refine=request.refine,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, "Thickening", voxel_size_mm=request.voxel_size_mm)
    _ensure_offset_shell_resolution(source_mesh, output_mesh, "Thickening", voxel_size_mm=request.voxel_size_mm)

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="thicken_mesh",
        operation_label=f"Thickening {request.thickness_mm:g} mm",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "thicken_mesh" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_thicken_{request.thickness_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_voxel_thicken",
            "meshlib_reference": "MR::thickenMesh",
            "meshlib_source": "MeshLib/source/MRVoxels/MROffset.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "thickness_mm": request.thickness_mm,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        "thicken",
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


@router.post("/versions/{version_id}/offset/weighted-shell", response_model=OffsetShellMeshResponse)
async def run_weighted_shell_for_version(
    version_id: str,
    request: WeightedShellRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for weighted shell")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")
    regions_artifact = get_artifact_by_type(db, version_id, "analysis_regions_json")
    region_payload = _load_json_artifact(regions_artifact)
    region_weights = {str(entry.region_id): float(entry.weight_mm) for entry in request.region_weights}
    if region_weights and not _region_payload_has_ids(region_payload, list(region_weights)):
        raise HTTPException(status_code=400, detail="Weighted shell region ids must exist in the region manifest")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        output_mesh = await run_in_threadpool(default_sdk.voxel_weighted_shell_mesh,
            source_mesh,
            regions=_region_entries_from_payload(region_payload),
            region_weights=region_weights,
            offset_mm=request.offset_mm,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            interpolation_distance_mm=request.interpolation_distance_mm,
            refine=request.refine,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, "Weighted Shell", voxel_size_mm=request.voxel_size_mm)
    _ensure_offset_shell_resolution(source_mesh, output_mesh, "Weighted Shell", voxel_size_mm=request.voxel_size_mm)

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="weighted_shell",
        operation_label=f"Weighted Shell {request.offset_mm:g} mm",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "weighted_shell" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_weighted_shell_{request.offset_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_voxel_weighted_shell",
            "meshlib_reference": "MR::WeightedShell::meshShell",
            "meshlib_source": "MeshLib/source/MRVoxels/MRWeightedPointsShell.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "offset_mm": request.offset_mm,
            "region_weights": region_weights,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "interpolation_distance_mm": request.interpolation_distance_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        "weighted_shell",
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


@router.post("/versions/{version_id}/offset/partial", response_model=OffsetShellMeshResponse)
async def run_partial_offset_for_version(
    version_id: str,
    request: PartialOffsetRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for partial offset")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")
    if not request.region_ids:
        raise HTTPException(status_code=400, detail="Partial offset requires at least one selected region")
    regions_artifact = get_artifact_by_type(db, version_id, "analysis_regions_json")
    region_payload = _load_json_artifact(regions_artifact)
    selected_region_ids = [str(region_id) for region_id in request.region_ids]
    if not _region_payload_has_ids(region_payload, selected_region_ids):
        raise HTTPException(status_code=400, detail="Partial offset region ids must exist in the region manifest")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        output_mesh = await run_in_threadpool(default_sdk.voxel_partial_offset_mesh,
            source_mesh,
            regions=_region_entries_from_payload(region_payload),
            selected_region_ids=selected_region_ids,
            offset_mm=request.offset_mm,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            refine=request.refine,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, "Partial Offset", voxel_size_mm=request.voxel_size_mm)
    _ensure_offset_shell_resolution(source_mesh, output_mesh, "Partial Offset", voxel_size_mm=request.voxel_size_mm)

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="partial_offset",
        operation_label=f"Partial Offset {request.offset_mm:g} mm",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "partial_offset" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_partial_offset_{request.offset_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_voxel_partial_offset",
            "meshlib_reference": "MR::partialOffsetMesh",
            "meshlib_source": "MeshLib/source/MRVoxels/MRPartialOffset.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "offset_mm": request.offset_mm,
            "selected_region_ids": selected_region_ids,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        "partial_offset",
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


@router.post("/versions/{version_id}/offset/verts", response_model=OffsetShellMeshResponse)
async def run_offset_verts_for_version(
    version_id: str,
    request: OffsetVertsRequest,
    db: Session = Depends(get_db),
) -> OffsetShellMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for offset verts")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if source_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")
    selected_region_ids = [str(region_id) for region_id in request.region_ids]
    regions_artifact = get_artifact_by_type(db, version_id, "analysis_regions_json") if selected_region_ids else None
    region_payload = _load_json_artifact(regions_artifact) if selected_region_ids else None
    if selected_region_ids and not _region_payload_has_ids(region_payload, selected_region_ids):
        raise HTTPException(status_code=400, detail="Offset verts region ids must exist in the region manifest")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        offsets_mm = [float(request.offset_mm)] * int(source_mesh.vertex_count)
        if selected_region_ids:
            offsets_mm = [0.0] * int(source_mesh.vertex_count)
            region_map = {
                str(region.get("region_id")): region
                for region in (region_payload or {}).get("regions", [])
            }
            for region_id in selected_region_ids:
                for vertex_index in region_map[region_id].get("vertex_indices", []):
                    index = int(vertex_index)
                    if index < 0 or index >= int(source_mesh.vertex_count):
                        raise ValueError(f"Offset verts region {region_id} references vertex {index}")
                    offsets_mm[index] = float(request.offset_mm)
        output_mesh = await run_in_threadpool(default_sdk.offset_verts_mesh, source_mesh, offsets_mm)
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    _ensure_nonempty_offset_shell_mesh(output_mesh, "Offset Verts")

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="offset_verts",
        operation_label=f"Offset Verts {request.offset_mm:g} mm",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "offset_verts" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_offset_verts_{request.offset_mm:g}mm.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_offset_verts",
            "meshlib_reference": "MR::offsetVerts",
            "meshlib_source": "MeshLib/source/MRMesh/MROffsetVerts.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "offset_mm": request.offset_mm,
            "selected_region_ids": selected_region_ids,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_offset_shell_response(
        version.id,
        "offset_verts",
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


def _boundary_edge_count(mesh) -> int:  # noqa: ANN001
    """Boundary (open) edge count via mesh health; 0 if health is unavailable."""
    try:
        return int(getattr(default_sdk.health(mesh), "boundary_edge_count", 0))
    except Exception:  # noqa: BLE001
        return 0


def _nonmanifold_edge_count(mesh) -> int:  # noqa: ANN001
    """Non-manifold edge count via mesh health; 0 if health is unavailable."""
    try:
        return int(getattr(default_sdk.health(mesh), "nonmanifold_edge_count", 0))
    except Exception:  # noqa: BLE001
        return 0


def _cap_planar_cut(mesh):  # noqa: ANN001
    """Close the open cut an exact boolean leaves when its cut contour cannot be
    stitched on organic input, then weld coincident vertices so the result is a clean
    (watertight + manifold) solid. A planar cut (a box leaves a planar boundary loop)
    closes exactly via fill_planar_holes; a NON-planar cut boundary (a sphere / organic
    tool) leaves a curved loop the planar fill can't close, so fall back to the general
    hole-fill (service_fill_holes), and a non-planar fill can introduce non-manifold
    edges so repair them and re-close. A no-op on an already-clean result (union, clean
    cube cuts) or an empty result; for planar cuts the exact volume split is preserved
    because the shared cut cap is added to both halves identically. Best-effort: if the
    cut is too degenerate to seal cleanly, residue survives for the endpoint to refuse.
    """
    if int(getattr(mesh, "face_count", 0)) == 0:
        return mesh
    if _boundary_edge_count(mesh) == 0 and _nonmanifold_edge_count(mesh) == 0:
        return mesh
    capped, _ = default_sdk.fill_planar_holes(mesh)
    # Non-planar residue (curved/organic cut) survives the planar fill — close it with
    # the general triangulating hole-fill.
    if _boundary_edge_count(capped) > 0:
        try:
            capped, _ = default_sdk.service_fill_holes(capped)
        except (RuntimeError, ValueError):  # noqa: BLE001 - leave residue for the endpoint guard
            pass
    # A non-planar fill can leave non-manifold edges; repair them, then re-close any
    # boundary the repair reopened.
    if _nonmanifold_edge_count(capped) > 0:
        try:
            capped, _ = default_sdk.repair_nonmanifold_edges(capped)
            if _boundary_edge_count(capped) > 0:
                capped, _ = default_sdk.fill_planar_holes(capped)
        except (RuntimeError, ValueError):  # noqa: BLE001 - leave residue for the endpoint guard
            pass
    welded, _ = default_sdk.weld_coincident_vertices(capped)
    return welded


def _enclosed_volume(mesh) -> float:  # noqa: ANN001
    """Absolute enclosed volume (mm^3) via Rust mesh stats; 0.0 if unavailable."""
    try:
        return abs(float(getattr(default_sdk.stats(mesh), "volume_mm3", 0.0)))
    except Exception:  # noqa: BLE001
        return 0.0


@router.post("/versions/{version_id}/boolean/exact", response_model=ExactBooleanResponse)
async def run_exact_boolean_for_version(
    version_id: str,
    request: ExactBooleanRequest,
    db: Session = Depends(get_db),
) -> ExactBooleanResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    other_version = db.get(ModelVersionRecord, request.other_version_id)
    if other_version is None:
        raise HTTPException(status_code=404, detail="Other version not found")
    if version.status != "ready" or other_version.status != "ready":
        raise HTTPException(status_code=409, detail="Both versions must be ready for exact boolean")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    target_artifact = get_artifact_by_type(db, request.other_version_id, "normalized_mesh_ply")
    if source_artifact is None or target_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        target_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(target_artifact))
        source_faces = int(getattr(source_mesh, "face_count", 0))
        target_faces = int(getattr(target_mesh, "face_count", 0))
        max_interactive_faces = settings.MESH_EDIT_EXACT_BOOLEAN_MAX_INTERACTIVE_FACES
        if max_interactive_faces > 0 and source_faces + target_faces > max_interactive_faces:
            raise HTTPException(
                status_code=400,
                detail=(
                    f"Exact Boolean is limited to {max_interactive_faces} combined faces for interactive jobs; "
                    f"source has {source_faces} faces and tool has {target_faces} faces. "
                    "Use a smaller selected/tool mesh or run an offline exact boolean job."
                ),
            )
        result = await run_in_threadpool(default_sdk.exact_boolean_mesh,
            source_mesh,
            target_mesh,
            operation=request.operation,
            epsilon=request.epsilon,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    # The Rust mesh_quality kernel computes the acceptance verdict: a near-zero-volume
    # sliver is degenerate residue, and an over-large result (vs the operand volumes) is
    # a watertight-but-wrong mis-classification the open/non-manifold guard cannot see.
    # Operand volumes come from the Rust stats kernel, so the geometry-fact thresholds
    # stay in the kernel rather than the service.
    try:
        boolean_failures = default_sdk.boolean_output_failures(
            result.mesh,
            operation=request.operation,
            source_volume_mm3=_enclosed_volume(source_mesh),
            target_volume_mm3=_enclosed_volume(target_mesh),
        )
    except Exception:  # noqa: BLE001 - advisory guard only
        boolean_failures = []
    if boolean_failures:
        raise HTTPException(status_code=400, detail=boolean_failures[0])

    # Cap the open cut so difference/intersection ship watertight (no-op on a closed
    # union result). Advisory: keep the raw boolean mesh if capping itself errors.
    try:
        result.mesh = await run_in_threadpool(_cap_planar_cut, result.mesh)
    except (RuntimeError, ValueError):
        pass

    operation_label = "difference" if request.operation == "difference_ab" else request.operation

    # Safety net: a result that still isn't a clean solid (watertight AND manifold)
    # after capping is not manufacturing-safe (e.g. a non-planar tool whose curved cut
    # even the general hole-fill / non-manifold repair could not seal). Refuse rather
    # than ship a defective mesh — no silent garbage.
    residual_open_edges = _boundary_edge_count(result.mesh)
    residual_nonmanifold_edges = _nonmanifold_edge_count(result.mesh)
    if residual_open_edges > 0 or residual_nonmanifold_edges > 0:
        raise HTTPException(
            status_code=400,
            detail=(
                f"Exact boolean ({operation_label}) produced a result that could not be sealed "
                f"into a clean solid ({residual_open_edges} open, {residual_nonmanifold_edges} "
                "non-manifold edges). The cut tool may graze the surface or be highly non-planar; "
                "try a voxel boolean or a simpler tool."
            ),
        )
    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="boolean",
        operation_label=f"Boolean {operation_label} with {request.other_version_id}",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "exact_boolean" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_{request.operation}_{request.other_version_id}.ply"
    default_sdk.save_mesh(result.mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_exact_boolean",
            "meshlib_reference": "MR::boolean",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshBoolean.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "other_version_id": request.other_version_id,
            "operation": request.operation,
            "epsilon": request.epsilon,
            "vertex_count": int(result.mesh.vertex_count),
            "face_count": int(result.mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_exact_boolean_response(version.id, request.other_version_id, request, output_version, output_artifact, result)


@router.post("/versions/{version_id}/boolean/voxel", response_model=VoxelBooleanResponse)
async def run_voxel_boolean_for_version(
    version_id: str,
    request: VoxelBooleanRequest,
    db: Session = Depends(get_db),
) -> VoxelBooleanResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    other_version = db.get(ModelVersionRecord, request.other_version_id)
    if other_version is None:
        raise HTTPException(status_code=404, detail="Other version not found")
    if version.status != "ready" or other_version.status != "ready":
        raise HTTPException(status_code=409, detail="Both versions must be ready for voxel boolean")

    source_artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    target_artifact = get_artifact_by_type(db, request.other_version_id, "normalized_mesh_ply")
    if source_artifact is None or target_artifact is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        source_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(source_artifact))
        target_mesh = default_sdk.load_mesh(_materialize_artifact_to_path(target_artifact))
        output_mesh = await run_in_threadpool(default_sdk.voxel_boolean_mesh,
            source_mesh,
            target_mesh,
            operation=request.operation,
            voxel_size_mm=request.voxel_size_mm,
            padding_mm=request.padding_mm,
            refine=request.refine,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    # Marching-cubes output can carry unwelded coincident vertices at cell seams that
    # read as a few open edges; weld them so the result ships as a clean closed solid.
    try:
        welded = await run_in_threadpool(default_sdk.weld_coincident_vertices, output_mesh)
        output_mesh = welded[0] if isinstance(welded, tuple) else welded
    except (RuntimeError, ValueError):
        pass

    # Safety net: a result that still isn't a clean solid is not manufacturing-safe.
    # Refuse rather than ship a defective mesh — no silent garbage.
    voxel_open_edges = _boundary_edge_count(output_mesh)
    voxel_nonmanifold_edges = _nonmanifold_edge_count(output_mesh)
    if voxel_open_edges > 0 or voxel_nonmanifold_edges > 0:
        raise HTTPException(
            status_code=400,
            detail=(
                f"Voxel boolean ({request.operation}) produced a result that is not a clean solid "
                f"({voxel_open_edges} open, {voxel_nonmanifold_edges} non-manifold edges); "
                "try a smaller voxel size."
            ),
        )

    output_version = create_version(
        db,
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="boolean",
        operation_label=f"Voxel Boolean {request.operation} with {request.other_version_id}",
        status="ready",
    )
    workdir = settings.TEMP_DIR / "voxel_boolean" / output_version.id
    workdir.mkdir(parents=True, exist_ok=True)
    output_path = workdir / f"{version_id}_voxel_{request.operation}_{request.other_version_id}.ply"
    default_sdk.save_mesh(output_mesh, output_path, file_type="ply")
    output_artifact = register_file_artifact(
        db,
        output_version.id,
        output_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json={
            "source": "rust_voxel_boolean",
            "meshlib_reference": "MRVoxels::voxelBoolean",
            "meshlib_source": "MeshLib/source/MRVoxels/MRBoolean.*",
            "rust_backed": True,
            "source_version_id": version.id,
            "other_version_id": request.other_version_id,
            "operation": request.operation,
            "voxel_size_mm": request.voxel_size_mm,
            "padding_mm": request.padding_mm,
            "refine": request.refine,
            "vertex_count": int(output_mesh.vertex_count),
            "face_count": int(output_mesh.face_count),
        },
    )
    db.commit()
    db.refresh(output_version)
    return _serialize_voxel_boolean_response(
        version.id,
        request.other_version_id,
        request,
        output_version,
        output_artifact,
        output_mesh,
    )


@router.post("/versions/{version_id}/voxels/mesh-to-sdf", response_model=MeshToVoxelsSdfResponse)
async def mesh_to_voxels_sdf_for_version(
    version_id: str,
    request: MeshToVoxelsSdfRequest,
    db: Session = Depends(get_db),
) -> MeshToVoxelsSdfResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel conversion")

    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if normalized is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    try:
        mesh = default_sdk.load_mesh(_materialize_artifact_to_path(normalized))
        if request.mode == "signed" and not default_sdk.health(mesh).is_closed:
            raise HTTPException(status_code=400, detail="Signed mesh-to-SDF requires a closed mesh")
        padding_mm = float(request.surface_offset_voxels * request.voxel_size_mm)
        grid = default_sdk.sample_sdf_grid(mesh, voxel_size_mm=request.voxel_size_mm, padding_mm=padding_mm)
        meshlib_reference = "meshToLevelSet"
        if request.mode == "unsigned":
            grid.values = abs(grid.values).astype("float32", copy=False)
            meshlib_reference = "meshToDistanceField"
        occupancy = default_sdk.sdf_occupancy(grid, iso_value=request.iso_value)
        estimated_volume = default_sdk.estimate_sdf_volume(grid, iso_value=request.iso_value)
        surface_mesh = (
            default_sdk.extract_sdf_isosurface(grid, iso_value=request.iso_value)
            if request.extract_surface
            else None
        )
    except HTTPException:
        raise
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return _serialize_mesh_to_voxels_sdf(
        version_id,
        request,
        grid,
        occupancy,
        estimated_volume,
        surface_mesh,
        meshlib_reference=meshlib_reference,
    )


@router.post("/versions/{version_id}/voxels/open-raw", response_model=VoxelVolumeLoadResponse)
async def open_raw_voxels_for_version(
    version_id: str,
    request: VoxelRawLoadRequest,
    db: Session = Depends(get_db),
) -> VoxelVolumeLoadResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for open-raw-voxels")

    safe_name = _voxel_safe_file_name(request.file_name, "volume.raw")
    with tempfile.TemporaryDirectory(prefix="meshinspector-raw-voxels-") as tmp_dir:
        raw_path = Path(tmp_dir) / safe_name
        raw_path.write_bytes(_decode_voxel_binary_payload(request.contents_base64, label="RAW voxels"))
        try:
            if request.auto_parameters:
                volume = default_sdk.load_raw_voxels_auto(raw_path)
                sdk_operation = "load_raw_voxels_auto"
            else:
                if request.dimensions is None:
                    raise ValueError("dimensions are required unless auto_parameters is true")
                volume = default_sdk.load_raw_voxels(
                    raw_path,
                    dimensions=request.dimensions,
                    voxel_size=request.voxel_size,
                    scalar_type=request.scalar_type,
                    grid_level_set=request.grid_level_set,
                )
                sdk_operation = "load_raw_voxels"
            return _serialize_voxel_volume_load_response(
                version_id,
                volume,
                sdk_operation=sdk_operation,
                meshlib_reference="MR::VoxelsLoad::fromRaw",
                meshlib_source="MeshLib/source/MRVoxels/MRVoxelsLoad.*",
                extra_metadata={"file_name": safe_name},
            )
        except (RuntimeError, ValueError) as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/voxels/open-tiff-dir", response_model=VoxelVolumeLoadResponse)
async def open_voxels_from_tiff_for_version(
    version_id: str,
    request: VoxelTiffLoadRequest,
    db: Session = Depends(get_db),
) -> VoxelVolumeLoadResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for open-voxels-from-tiff")

    safe_names: list[str] = []
    with tempfile.TemporaryDirectory(prefix="meshinspector-tiff-voxels-") as tmp_dir:
        tiff_dir = Path(tmp_dir)
        for index, (file_name, contents_base64) in enumerate(request.files.items()):
            safe_name = _voxel_safe_file_name(file_name, f"slice_{index:03d}.tiff")
            if safe_name in safe_names:
                stem = Path(safe_name).stem or "slice"
                suffix = Path(safe_name).suffix or ".tiff"
                safe_name = f"{stem}_{index:03d}{suffix}"
            safe_names.append(safe_name)
            (tiff_dir / safe_name).write_bytes(
                _decode_voxel_binary_payload(contents_base64, label=f"TIFF voxel slice {safe_name}")
            )
        try:
            volume = default_sdk.load_tiff_voxels_dir(
                tiff_dir,
                voxel_size=request.voxel_size,
                grid_level_set=request.grid_level_set,
            )
            return _serialize_voxel_volume_load_response(
                version_id,
                volume,
                sdk_operation="load_tiff_voxels_dir",
                meshlib_reference="MR::VoxelsLoad::loadTiffDir",
                meshlib_source="MeshLib/source/MRVoxels/MRVoxelsLoad.*",
                extra_metadata={"file_names": safe_names},
            )
        except (RuntimeError, ValueError) as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/versions/{version_id}/voxels/binary", response_model=VoxelBinaryOperationsResponse)
async def voxel_binary_operations_for_version(
    version_id: str,
    request: VoxelBinaryOperationsRequest,
    db: Session = Depends(get_db),
) -> VoxelBinaryOperationsResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel binary operations")

    try:
        left = SDFGrid(
            origin=request.origin,
            voxel_size_mm=request.voxel_size_mm,
            shape=request.shape,
            values=request.left_values,
        )
        right = SDFGrid(
            origin=request.origin,
            voxel_size_mm=request.voxel_size_mm,
            shape=request.shape,
            values=request.right_values,
        )
        result_grid, result_iso = default_sdk.voxel_binary_operation(
            left,
            right,
            operation=request.operation,
            left_iso_value=request.left_iso_value,
            right_iso_value=request.right_iso_value,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    values = _flatten_numeric_values(result_grid.values)
    return VoxelBinaryOperationsResponse(
        version_id=version_id,
        operation=request.operation,
        shape=tuple(int(axis) for axis in result_grid.shape),
        origin=tuple(float(axis) for axis in result_grid.origin),
        voxel_size_mm=float(result_grid.voxel_size_mm),
        values=values,
        result_iso_value=float(result_iso),
        min_value=_min_numeric_value(values),
        max_value=_max_numeric_value(values),
        metadata={
            "rust_backed": True,
            "sdk_operation": "voxel_binary_operation",
            "meshlib_reference": "MeshLib CommonPlugins BinaryOperations",
            "meshlib_operations": ["max", "min", "sum", "multiply", "divide", "union", "intersection", "difference"],
            "meshlib_source": "MeshLib/source/MRMesh/MRObjectVoxels.*",
        },
    )


@router.post("/versions/{version_id}/voxels/segmentation", response_model=VoxelSegmentationResponse)
async def voxel_segmentation_for_version(
    version_id: str,
    request: VoxelSegmentationRequest,
    db: Session = Depends(get_db),
) -> VoxelSegmentationResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel segmentation")

    try:
        mesh = default_sdk.voxel_segmentation_mesh(
            request.values,
            shape=request.shape,
            inside_seeds=request.inside_seeds,
            outside_seeds=request.outside_seeds,
            exponent_modifier=request.exponent_modifier,
            voxels_expansion=request.voxels_expansion,
            include_boundary_outside=request.include_boundary_outside,
            voxel_size=request.voxel_size,
        )
        mesh = _reject_shattered_voxel_mesh(mesh, "Voxel segmentation")
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    vertices, faces, bounds_min, bounds_max = _mesh_response_geometry(mesh)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_segmentation_mesh",
            "meshlib_reference": "MRVoxelGraphCut + MRVolumeSegment::createMeshFromSegmentation",
        }
    )
    return VoxelSegmentationResponse(
        version_id=version_id,
        vertex_count=int(mesh.vertex_count),
        face_count=int(mesh.face_count),
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        vertices=vertices,
        faces=faces,
        metadata=metadata,
    )


@router.post("/versions/{version_id}/voxels/mask-to-mesh", response_model=VoxelMaskToMeshResponse)
async def voxel_mask_to_mesh_for_version(
    version_id: str,
    request: VoxelMaskToMeshRequest,
    db: Session = Depends(get_db),
) -> VoxelMaskToMeshResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel mask-to-mesh conversion")

    try:
        mesh = await run_in_threadpool(default_sdk.voxel_mask_to_mesh,
            request.values,
            shape=request.shape,
            mask_coordinates=request.mask_coordinates,
            voxel_size=request.voxel_size,
            mask_expansion=request.mask_expansion,
            smooth_band_radius=request.smooth_band_radius,
        )
        mesh = _reject_shattered_voxel_mesh(mesh, "Voxel mask-to-mesh")
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    vertices, faces, bounds_min, bounds_max = _mesh_response_geometry(mesh)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_mask_to_mesh",
            "meshlib_reference": "MRVolumeSegment::meshFromVoxelsMask",
        }
    )
    return VoxelMaskToMeshResponse(
        version_id=version_id,
        vertex_count=int(mesh.vertex_count),
        face_count=int(mesh.face_count),
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        vertices=vertices,
        faces=faces,
        metadata=metadata,
    )


@router.post("/versions/{version_id}/voxels/to-mesh/simple", response_model=VoxelToMeshSimpleResponse)
async def voxel_to_mesh_simple_for_version(
    version_id: str,
    request: VoxelToMeshSimpleRequest,
    db: Session = Depends(get_db),
) -> VoxelToMeshSimpleResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel-to-mesh conversion")

    try:
        source = (
            default_sdk.voxel_volume_from_meshlib_values(
                request.values,
                dimensions=request.shape,
                voxel_size=request.voxel_size,
                grid_level_set=request.grid_level_set,
                scalar_type=request.scalar_type,
                min_value=request.min_value,
                max_value=request.max_value,
                iso_value=request.iso_value,
            )
            if request.grid_level_set
            else request.values
        )
        mesh = await run_in_threadpool(default_sdk.voxel_to_mesh_simple,
            source,
            shape=request.shape if not request.grid_level_set else None,
            voxel_size=request.voxel_size,
            iso_value=request.iso_value,
        )
        mesh = await run_in_threadpool(_finalize_voxel_to_mesh_output, mesh, "Voxel→mesh (simple)")
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    vertices, faces, bounds_min, bounds_max = _mesh_response_geometry(mesh)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_to_mesh_simple",
        }
    )
    return VoxelToMeshSimpleResponse(
        version_id=version_id,
        vertex_count=int(mesh.vertex_count),
        face_count=int(mesh.face_count),
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        vertices=vertices,
        faces=faces,
        metadata=metadata,
    )


@router.post("/versions/{version_id}/voxels/to-mesh/dual", response_model=VoxelToMeshDualResponse)
async def voxel_to_mesh_dual_for_version(
    version_id: str,
    request: VoxelToMeshDualRequest,
    db: Session = Depends(get_db),
) -> VoxelToMeshDualResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel-to-mesh dual conversion")

    try:
        sdk_operation = "voxel_to_mesh_dual"
        if request.model_bytes_base64:
            if request.model_extension.strip().lower() not in {".vdb", "vdb"}:
                raise ValueError("model_extension must be .vdb when model_bytes_base64 is provided")
            try:
                model_bytes = base64.b64decode(request.model_bytes_base64, validate=True)
            except (binascii.Error, ValueError) as exc:
                raise ValueError("model_bytes_base64 must be valid base64") from exc
            mesh = await run_in_threadpool(default_sdk.voxel_to_mesh_dual_vdb_payload,
                model_bytes,
                dimensions=request.shape,
                voxel_size=request.voxel_size,
                iso_value=float(request.iso_value or 0.0),
                adaptivity=request.adaptivity,
                max_faces=request.max_faces,
                max_vertices=request.max_vertices,
                relax_disoriented_triangles=request.relax_disoriented_triangles,
            )
            sdk_operation = "voxel_to_mesh_dual_vdb_payload"
        else:
            source = (
                default_sdk.voxel_volume_from_meshlib_values(
                    request.values,
                    dimensions=request.shape,
                    voxel_size=request.voxel_size,
                    grid_level_set=request.grid_level_set,
                    scalar_type=request.scalar_type,
                    min_value=request.min_value,
                    max_value=request.max_value,
                    iso_value=request.iso_value,
                )
                if request.grid_level_set
                else request.values
            )
            mesh = await run_in_threadpool(default_sdk.voxel_to_mesh_dual,
                source,
                shape=request.shape if not request.grid_level_set else None,
                voxel_size=request.voxel_size,
                iso_value=request.iso_value,
                adaptivity=request.adaptivity,
                max_faces=request.max_faces,
                max_vertices=request.max_vertices,
                relax_disoriented_triangles=request.relax_disoriented_triangles,
            )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    vertices, faces, bounds_min, bounds_max = _mesh_response_geometry(mesh)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": sdk_operation,
        }
    )
    return VoxelToMeshDualResponse(
        version_id=version_id,
        vertex_count=int(mesh.vertex_count),
        face_count=int(mesh.face_count),
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        vertices=vertices,
        faces=faces,
        metadata=metadata,
    )


@router.post("/versions/{version_id}/voxels/to-mesh/smart", response_model=VoxelToMeshSmartResponse)
async def voxel_to_mesh_smart_for_version(
    version_id: str,
    request: VoxelToMeshSmartRequest,
    db: Session = Depends(get_db),
) -> VoxelToMeshSmartResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel-to-mesh smart conversion")

    try:
        source = (
            default_sdk.voxel_volume_from_meshlib_values(
                request.values,
                dimensions=request.shape,
                voxel_size=request.voxel_size,
                grid_level_set=request.grid_level_set,
                scalar_type=request.scalar_type,
                min_value=request.min_value,
                max_value=request.max_value,
                iso_value=request.iso_value,
            )
            if request.grid_level_set
            else request.values
        )
        mesh = await run_in_threadpool(default_sdk.voxel_to_mesh_smart,
            source,
            shape=request.shape if not request.grid_level_set else None,
            voxel_size=request.voxel_size,
            iso_value=request.iso_value,
            iters=request.iters,
            sample_points=request.sample_points,
            degree=request.degree,
            outlier_threshold=request.outlier_threshold,
            intermediate_smooth_force=request.intermediate_smooth_force,
            preparation_smooth_force=request.preparation_smooth_force,
            smooth_shift_iterations=request.smooth_shift_iterations,
            final_relax_iterations=request.final_relax_iterations,
            final_relax_force=request.final_relax_force,
        )
        mesh = await run_in_threadpool(_finalize_voxel_to_mesh_output, mesh, "Voxel→mesh (smart)")
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    vertices, faces, bounds_min, bounds_max = _mesh_response_geometry(mesh)
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_to_mesh_smart",
        }
    )
    return VoxelToMeshSmartResponse(
        version_id=version_id,
        vertex_count=int(mesh.vertex_count),
        face_count=int(mesh.face_count),
        bounds_min=bounds_min,
        bounds_max=bounds_max,
        vertices=vertices,
        faces=faces,
        metadata=metadata,
    )


def _serialize_voxel_path_payload(result) -> VoxelPathPayload:  # noqa: ANN001
    return VoxelPathPayload(
        voxel_indices=[int(index) for index in result.voxel_indices],
        coordinates=[tuple(int(axis) for axis in coordinate) for coordinate in result.coordinates],
        total_metric=float(result.total_metric),
    )


@router.post("/versions/{version_id}/voxels/path", response_model=VoxelPathResponse)
async def voxel_path_for_version(
    version_id: str,
    request: VoxelPathRequest,
    db: Session = Depends(get_db),
) -> VoxelPathResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel path construction")

    try:
        result = default_sdk.voxel_path(
            request.values,
            shape=request.shape,
            start=request.start,
            finish=request.finish,
            metric=request.metric,
            max_dist_ratio=request.max_dist_ratio,
            plane=request.plane,
            quarters_mask=request.quarters_mask,
            exponent_modifier=request.exponent_modifier,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    payload = _serialize_voxel_path_payload(result)
    return VoxelPathResponse(
        version_id=version_id,
        voxel_indices=payload.voxel_indices,
        coordinates=payload.coordinates,
        total_metric=payload.total_metric,
        metadata={
            "rust_backed": True,
            "sdk_operation": "voxel_path",
            "meshlib_reference": "MRVoxelPath::buildSmallestMetricPath",
            "meshlib_metrics": ["Difference", "Exponent"],
            "meshlib_source": "MeshLib/source/MRVoxels/MRVoxelPath.*",
        },
    )


@router.post("/versions/{version_id}/voxels/path/build-four", response_model=VoxelPathBuildFourResponse)
async def voxel_path_build_four_for_version(
    version_id: str,
    request: VoxelPathBuildFourRequest,
    db: Session = Depends(get_db),
) -> VoxelPathBuildFourResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel path construction")

    try:
        result = default_sdk.voxel_path_build_four(
            request.values,
            shape=request.shape,
            start=request.start,
            finish=request.finish,
            metric=request.metric,
            max_dist_ratio=request.max_dist_ratio,
            plane=request.plane,
            exponent_modifier=request.exponent_modifier,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return VoxelPathBuildFourResponse(
        version_id=version_id,
        paths=[
            VoxelPathBuildFourEntry(
                quarters_mask=int(entry["quarters_mask"]),
                path=_serialize_voxel_path_payload(entry["path"]),
            )
            for entry in result
        ],
        metadata={
            "rust_backed": True,
            "sdk_operation": "voxel_path_build_four",
            "meshlib_reference": "MRVoxelPath::buildSmallestMetricPath",
            "meshlib_quarter_masks": [1, 2, 4, 8],
            "meshlib_source": "MeshLib/source/MRVoxels/MRVoxelPath.*",
        },
    )


@router.post("/versions/{version_id}/voxels/slice", response_model=VoxelSliceResponse)
async def voxel_slice_for_version(
    version_id: str,
    request: VoxelSliceRequest,
    db: Session = Depends(get_db),
) -> VoxelSliceResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel slice extraction")

    try:
        result = default_sdk.voxel_slice(
            request.values,
            shape=request.shape,
            plane=request.plane,
            slice_index=request.slice_index,
            min_value=request.min_value,
            max_value=request.max_value,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return VoxelSliceResponse(
        version_id=version_id,
        width=int(result.width),
        height=int(result.height),
        values=[float(value) for value in result.values.tolist()],
        normalized_values=[float(value) for value in result.normalized_values.tolist()],
        coordinates=[tuple(int(axis) for axis in coordinate) for coordinate in result.coordinates],
        metadata={
            "rust_backed": True,
            "sdk_operation": "voxel_slice",
            "meshlib_reference": "MRVoxelsSave::saveSliceToImage",
            "meshlib_slice": "MRMarkedVoxelSlice",
            "meshlib_source": "MeshLib/source/MRVoxels/MRVoxelsSave.*",
        },
    )


@router.post("/versions/{version_id}/voxels/line-graph", response_model=VoxelLineGraphResponse)
async def voxel_line_graph_for_version(
    version_id: str,
    request: VoxelLineGraphRequest,
    db: Session = Depends(get_db),
) -> VoxelLineGraphResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel line graph sampling")

    try:
        result = default_sdk.voxel_line_graph(
            request.values,
            shape=request.shape,
            axis=request.axis,
            fixed_coordinate=request.fixed_coordinate,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return VoxelLineGraphResponse(
        version_id=version_id,
        axis=int(result.axis),
        positions=[int(position) for position in result.positions],
        voxel_indices=[int(index) for index in result.voxel_indices],
        coordinates=[tuple(int(axis) for axis in coordinate) for coordinate in result.coordinates],
        values=[float(value) for value in result.values.tolist()],
        metadata={
            "rust_backed": True,
            "sdk_operation": "voxel_line_graph",
            "meshlib_reference": "MeshInspector Voxels Line Graph",
            "meshlib_indexing": "x-fastest dense voxel indexing",
            "meshlib_source": "MeshLib/source/MRVoxels/MRMarkedVoxelSlice.*",
        },
    )


@router.post("/versions/{version_id}/voxels/active-box", response_model=VoxelActiveBoxResponse)
async def voxel_active_box_for_version(
    version_id: str,
    request: VoxelActiveBoxRequest,
    db: Session = Depends(get_db),
) -> VoxelActiveBoxResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for active voxel box extraction")

    try:
        result = default_sdk.voxel_active_box(
            request.values,
            shape=request.shape,
            min_corner=request.min_corner,
            dimensions=request.dimensions,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return VoxelActiveBoxResponse(
        version_id=version_id,
        min_corner=tuple(int(axis) for axis in result.min_corner),
        dimensions=tuple(int(axis) for axis in result.dimensions),
        source_indices=[int(index) for index in result.source_indices],
        coordinates=[tuple(int(axis) for axis in coordinate) for coordinate in result.coordinates],
        values=[float(value) for value in result.values.tolist()],
        metadata={
            "rust_backed": True,
            "sdk_operation": "voxel_active_box",
            "meshlib_reference": "ObjectVoxels::setActiveBounds",
            "meshlib_bounds": "max-excluded active voxel box",
            "meshlib_source": "MeshLib/source/MRVoxels/MRObjectVoxels.*",
        },
    )


@router.post("/versions/{version_id}/voxels/volume-render-data", response_model=VoxelVolumeRenderDataResponse)
async def voxel_volume_render_data_for_version(
    version_id: str,
    request: VoxelVolumeRenderDataRequest,
    db: Session = Depends(get_db),
) -> VoxelVolumeRenderDataResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel volume rendering")

    try:
        result = default_sdk.voxel_volume_render_data(
            request.values,
            shape=request.shape,
            voxel_size=request.voxel_size,
            active_min_corner=request.active_min_corner,
            active_dimensions=request.active_dimensions,
            source_min_value=request.source_min_value,
            source_max_value=request.source_max_value,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    metadata = dict(result.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_volume_render_data",
            "meshlib_reference": "ObjectVoxels::prepareDataForVolumeRendering",
            "meshlib_source": "MeshLib/source/MRVoxels/MRObjectVoxels.*",
        }
    )

    return VoxelVolumeRenderDataResponse(
        version_id=version_id,
        dimensions=result.dimensions,
        voxel_size=result.voxel_size,
        source_indices=[int(index) for index in result.source_indices],
        coordinates=[tuple(int(axis) for axis in coordinate) for coordinate in result.coordinates],
        values=[float(value) for value in result.values.tolist()],
        min_value=float(result.min_value),
        max_value=float(result.max_value),
        metadata=metadata,
    )


@router.post("/versions/{version_id}/voxels/volume-render-lut", response_model=VoxelVolumeRenderLutResponse)
async def voxel_volume_render_lut_for_version(
    version_id: str,
    request: VoxelVolumeRenderLutRequest,
    db: Session = Depends(get_db),
) -> VoxelVolumeRenderLutResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel volume rendering")

    try:
        result = default_sdk.voxel_volume_render_lut(
            lut_type=request.lut_type,
            alpha_type=request.alpha_type,
            alpha_limit=request.alpha_limit,
            one_color=request.one_color,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    metadata = dict(result.metadata)
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_volume_render_lut",
            "meshlib_reference": "RenderVolumeObject::bindVolume_ denseMap",
            "meshlib_source": "MeshLib/source/MRViewer/MRRenderVolumeObject.*",
        }
    )

    return VoxelVolumeRenderLutResponse(
        version_id=version_id,
        lut_type=result.lut_type,
        alpha_type=result.alpha_type,
        alpha_limit=result.alpha_limit,
        colors_rgba=[tuple(int(channel) for channel in color) for color in result.colors_rgba],
        metadata=metadata,
    )


@router.post("/versions/{version_id}/voxels/volume-render-ray", response_model=VoxelVolumeRenderRayResponse)
async def voxel_volume_render_ray_for_version(
    version_id: str,
    request: VoxelVolumeRenderRayRequest,
    db: Session = Depends(get_db),
) -> VoxelVolumeRenderRayResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Version is not ready for voxel volume rendering")

    try:
        result = default_sdk.voxel_volume_render_ray(
            request.values,
            shape=request.shape,
            voxel_size=request.voxel_size,
            min_corner=request.min_corner,
            ray_start=request.ray_start,
            ray_direction=request.ray_direction,
            sampling_step=request.sampling_step,
            min_value=request.min_value,
            max_value=request.max_value,
            lut_type=request.lut_type,
            alpha_type=request.alpha_type,
            alpha_limit=request.alpha_limit,
            one_color=request.one_color,
            clipping_plane=request.clipping_plane,
            shading_mode=request.shading_mode,
            light_pos_eye=request.light_pos_eye,
            ambient_strength=request.ambient_strength,
            specular_strength=request.specular_strength,
            spec_exp=request.spec_exp,
            active_indices=None if request.active_indices is None else tuple(request.active_indices),
            max_steps=request.max_steps,
        )
    except (RuntimeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    metadata = dict(result.metadata)
    lighting = metadata.get("lighting")
    metadata.update(
        {
            "rust_backed": True,
            "sdk_operation": "voxel_volume_render_ray",
            "meshlib_reference": "MRVolumeShader",
            "meshlib_source": "MeshLib/source/MRViewer/MRVolumeShader.*",
        }
    )
    if isinstance(lighting, dict) and lighting.get("meshlib_shader"):
        metadata["meshlib_shader"] = str(lighting["meshlib_shader"])

    return VoxelVolumeRenderRayResponse(
        version_id=version_id,
        color_rgba=[float(value) for value in result.color_rgba.tolist()],
        first_opaque_world=result.first_opaque_world,
        visited_indices=[int(index) for index in result.visited_indices],
        accepted_indices=[int(index) for index in result.accepted_indices],
        metadata=metadata,
    )


@router.get("/versions/{version_id}/section", response_model=SectionContourPayload)
async def get_section_contour(
    version_id: str,
    section_constant: float = Query(0.0),
    axis_x: float = Query(0.0),
    axis_y: float = Query(1.0),
    axis_z: float = Query(0.0),
    selected_region_ids: str | None = Query(None),
    db: Session = Depends(get_db),
) -> SectionContourPayload:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    normalized = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if normalized is None:
        raise HTTPException(status_code=404, detail="Normalized mesh artifact not found")

    regions = _artifact_by_type_or_parent(db, version, "analysis_regions_json")
    region_payload = _load_json_artifact(regions)
    mesh = default_sdk.load_mesh(_materialize_artifact_to_path(normalized))
    selected_vertex_indices = _selected_region_vertex_indices(region_payload, selected_region_ids)
    if selected_vertex_indices and max(selected_vertex_indices) >= int(mesh.vertex_count):
        selected_vertex_indices = []
    try:
        contour = default_sdk.section_contour(
            mesh,
            section_constant=section_constant,
            plane_axis=(axis_x, axis_y, axis_z),
            selected_vertex_indices=selected_vertex_indices,
        )
    except ValueError as error:
        raise HTTPException(status_code=400, detail=str(error)) from error
    return _serialize_section_contour(contour)


@router.get("/versions/{version_id}/compare-cache", response_model=list[CompareCacheEntry])
async def get_compare_cache(version_id: str, db: Session = Depends(get_db)) -> list[CompareCacheEntry]:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    entries: list[CompareCacheEntry] = []
    for artifact in get_version_artifacts(db, version_id):
        if not artifact.artifact_type.startswith("analysis_compare_npz_"):
            continue
        other_version_id = str(artifact.metadata_json.get("other_version_id") or artifact.artifact_type.removeprefix("analysis_compare_npz_"))
        entries.append(
            CompareCacheEntry(
                other_version_id=other_version_id,
                artifact_id=artifact.id,
                created_at=artifact.created_at,
                generated_by=artifact.metadata_json.get("generated_by"),
                summary=CompareResponse.model_validate(summary) if isinstance((summary := artifact.metadata_json.get("summary")), dict) else None,
            )
        )
    return sorted(entries, key=lambda entry: entry.created_at, reverse=True)


@router.get("/versions/{version_id}/compare/{other_version_id}", response_model=CompareResponse)
async def get_compare_summary(version_id: str, other_version_id: str, db: Session = Depends(get_db)) -> CompareResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    artifact = get_artifact_by_type(db, version_id, f"analysis_compare_npz_{other_version_id}")
    if artifact is None:
        raise HTTPException(status_code=404, detail="Compare result not found")
    summary = artifact.metadata_json.get("summary") if artifact.metadata_json else None
    if not isinstance(summary, dict):
        raise HTTPException(status_code=409, detail="Compare result cached without summary metadata")
    return CompareResponse.model_validate(summary)


@router.post("/versions/{version_id}/branch", response_model=ModelVersionSummary)
async def branch_version(
    version_id: str,
    request: BranchVersionRequest,
    db: Session = Depends(get_db),
) -> ModelVersionSummary:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    if version.status != "ready":
        raise HTTPException(status_code=409, detail="Only ready versions can be branched")
    required_artifacts = ("normalized_mesh_ply", "preview_glb_high", "preview_glb_low", "manufacturing_stl")
    missing_artifacts = [artifact_type for artifact_type in required_artifacts if get_artifact_by_type(db, version.id, artifact_type) is None]
    if missing_artifacts:
        raise HTTPException(
            status_code=409,
            detail=f"Version is missing required artifacts for branching: {', '.join(missing_artifacts)}",
        )
    if get_snapshot(db, version.id, "manufacturability") is None:
        raise HTTPException(status_code=409, detail="Version is missing manufacturability analysis required for branching")
    cloned = duplicate_version(
        db,
        version,
        operation_type="branch",
        operation_label=request.operation_label.strip(),
    )
    db.commit()
    db.refresh(cloned)
    return serialize_version(cloned)


@router.get("/versions/{version_id}/inspection-snapshots", response_model=list[InspectionSnapshotResponse])
async def get_inspection_snapshots(version_id: str, db: Session = Depends(get_db)) -> list[InspectionSnapshotResponse]:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    return [serialize_inspection_snapshot(snapshot) for snapshot in list_snapshots_by_prefix(db, version_id, "inspection:")]


@router.post("/versions/{version_id}/inspection-snapshots", response_model=InspectionSnapshotResponse)
async def create_inspection_snapshot(
    version_id: str,
    request: InspectionSnapshotState,
    db: Session = Depends(get_db),
) -> InspectionSnapshotResponse:
    version = db.get(ModelVersionRecord, version_id)
    if version is None:
        raise HTTPException(status_code=404, detail="Version not found")
    snapshot = create_snapshot_record(
        db,
        version_id,
        f"inspection:{request.name.strip()}:{version_id}",
        request.model_dump(mode="json"),
    )
    db.commit()
    db.refresh(snapshot)
    return serialize_inspection_snapshot(snapshot)


@router.get("/versions/{version_id}/overlays/thickness")
async def get_thickness_overlay(version_id: str, db: Session = Depends(get_db)):
    artifact = get_artifact_by_type(db, version_id, "analysis_thickness_npz")
    if artifact is None:
        raise HTTPException(status_code=404, detail="Thickness overlay not found")
    return default_sdk.thickness_overlay_payload(_materialize_artifact_to_path(artifact))


@router.get("/versions/{version_id}/overlays/compare/{other_version_id}")
async def get_compare_overlay(version_id: str, other_version_id: str, db: Session = Depends(get_db)):
    version = db.get(ModelVersionRecord, version_id)
    other_version = db.get(ModelVersionRecord, other_version_id)
    if version is None or other_version is None:
        raise HTTPException(status_code=404, detail="Version not found")

    artifact_a = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    artifact_b = get_artifact_by_type(db, other_version_id, "normalized_mesh_ply")
    if artifact_a is None or artifact_b is None:
        raise HTTPException(status_code=404, detail="Comparison mesh artifact not found")

    cached = get_artifact_by_type(db, version_id, f"analysis_compare_npz_{other_version_id}")
    if cached is not None:
        return default_sdk.compare_overlay_payload(
            _materialize_artifact_to_path(cached),
            other_version_id=other_version_id,
        )
    raise HTTPException(status_code=404, detail="Compare overlay not cached; submit compare job first")


@router.get("/artifacts/{artifact_id}")
async def download_artifact(artifact_id: str, db: Session = Depends(get_db)):
    artifact = db.get(ModelArtifactRecord, artifact_id)
    if artifact is None:
        raise HTTPException(status_code=404, detail="Artifact not found")

    if object_store.driver == "local":
        path = object_store.get_local_path(artifact.storage_key)
        if not path.exists():
            raise HTTPException(status_code=404, detail="Artifact file not found")
        return FileResponse(path, media_type=artifact.mime_type, filename=Path(path).name)

    temp_path = _download_temp_path(artifact)
    object_store.download_to_path(artifact.storage_key, temp_path)
    return FileResponse(temp_path, media_type=artifact.mime_type, filename=temp_path.name)
