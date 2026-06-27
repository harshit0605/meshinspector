/**
 * API types for both compatibility and versioned production endpoints.
 */

export interface UploadResponse {
  model_id: string;
  filename: string;
  file_format: string;
  preview_url: string;
}

export interface AnalysisResultRaw {
  volume_mm3: number;
  weight_g: number;
  bbox_mm: [number, number, number];
  is_watertight: boolean;
  vertex_count: number;
  face_count: number;
}

export interface BoundingBox {
  x: number;
  y: number;
  z: number;
}

export interface AnalysisResponse {
  volume_mm3: number;
  weight_grams: number;
  bounding_box: BoundingBox;
  is_watertight: boolean;
  vertex_count: number;
  face_count: number;
}

export type AnalysisResult = AnalysisResponse;

export type MaterialType =
  | 'gold_24k'
  | 'gold_22k'
  | 'gold_18k'
  | 'gold_14k'
  | 'gold_10k'
  | 'silver_925'
  | 'platinum';

export interface ProcessRequest {
  model_id: string;
  ring_size?: number;
  wall_thickness_mm?: number;
  target_weight_g?: number;
  material: MaterialType;
}

export interface ProcessResponse {
  model_id: string;
  original_weight_g: number;
  final_weight_g: number;
  wall_thickness_mm: number | null;
  ring_size: number | null;
  preview_url: string;
  download_url_glb: string;
  download_url_stl: string;
  achieved_weight_g?: number | null;
  iterations?: number | null;
  warning?: string | null;
}

export interface ErrorResponse {
  error: string;
  detail?: string;
}

export interface ModelSummary {
  id: string;
  source_filename: string;
  source_type: string;
  created_at: string;
  latest_version_id: string | null;
}

export interface ScalarOverlayResponse {
  overlay_type: 'thickness' | 'compare';
  values: number[];
  min_value: number;
  max_value: number;
  center_value: number;
  threshold_mm?: number | null;
  summary?: Record<string, unknown>;
}

export interface CompareCacheEntry {
  other_version_id: string;
  artifact_id: string;
  created_at: string;
  generated_by?: string | null;
  summary?: CompareSummary | null;
}

export interface InspectionSnapshotState {
  name: string;
  axis_mode: 'auto' | 'manual';
  manual_axis: [number, number, number] | null;
  section_enabled: boolean;
  section_constant: number;
  selected_region_id: string | null;
  selected_region_ids: string[];
  heatmap_enabled: boolean;
  compare_enabled: boolean;
  compare_target_version_id: string | null;
}

export interface InspectionSnapshotResponse extends InspectionSnapshotState {
  id: string;
  version_id: string;
  created_at: string;
}

export interface SectionSliceStats {
  contour_count: number;
  segment_count: number;
  selected_region_segment_count: number;
  perimeter_mm: number | null;
  width_mm: number | null;
  depth_mm: number | null;
}

export interface SectionContourSegment {
  start: [number, number, number];
  end: [number, number, number];
  selected_region_hit: boolean;
}

export interface SectionContourPayload extends SectionSliceStats {
  section_constant: number;
  plane_axis: [number, number, number];
  plane_u_axis: [number, number, number];
  plane_v_axis: [number, number, number];
  plane_origin: [number, number, number];
  projected_bounds_min: [number, number] | null;
  projected_bounds_max: [number, number] | null;
  bounds_min: [number, number, number] | null;
  bounds_max: [number, number, number] | null;
  segments: SectionContourSegment[];
}

export interface VersionSummary {
  id: string;
  model_id: string;
  parent_version_id: string | null;
  operation_type: string;
  operation_label: string;
  status: string;
  created_at: string;
}

export interface BranchVersionRequest {
  operation_label: string;
}

export interface ArtifactSummary {
  id: string;
  artifact_type: string;
  mime_type: string;
  storage_key: string;
  size_bytes: number;
  metadata_json: Record<string, unknown>;
}

export interface JobResponse {
  id: string;
  version_id: string;
  operation_type: string;
  status: string;
  progress_pct: number;
  error_code?: string | null;
  error_message?: string | null;
  result_json?: Record<string, unknown> | null;
  started_at?: string | null;
  finished_at?: string | null;
  created_at: string;
}

export interface JobEventResponse {
  id: string;
  level: string;
  message: string;
  progress_pct?: number | null;
  created_at: string;
}

export interface CreateModelResponse {
  model: ModelSummary;
  version: VersionSummary;
  job?: JobResponse | null;
}

export interface MeshHealthSnapshot {
  is_closed: boolean;
  holes_count: number;
  self_intersections: number;
  disconnected_shells: number;
  health_score: number;
}

export interface DimensionsSnapshot {
  unit_system: 'mm';
  ring_axis?: [number, number, number] | null;
  ring_axis_confidence: number;
  estimated_ring_size_us: number | null;
  inner_diameter_mm: number | null;
  band_width_min_mm: number | null;
  band_width_max_mm: number | null;
  head_height_mm: number | null;
  bbox_mm: [number, number, number];
  needs_axis_confirmation: boolean;
}

export interface MaterialWeightEntry {
  volume_mm3: number;
  weight_g: number;
}

export interface ThicknessSnapshot {
  min_mm: number | null;
  avg_mm: number | null;
  max_mm: number | null;
  violation_count: number;
  threshold_mm: number;
  scalar_field_artifact_id: string | null;
}

export interface RegionManifestEntry {
  region_id: string;
  label: string;
  vertex_count: number;
  coverage_pct: number;
  min_thickness_mm?: number | null;
  avg_thickness_mm?: number | null;
  violation_count: number;
  protected_by_default: boolean;
  allowed_operations: string[];
  centroid_mm?: [number, number, number] | null;
}

export interface ManufacturabilitySnapshot {
  version_id: string;
  mesh_health: MeshHealthSnapshot;
  dimensions: DimensionsSnapshot;
  material_weight: Record<MaterialType, MaterialWeightEntry>;
  thickness: ThicknessSnapshot;
  regions: RegionManifestEntry[];
  recommendations: string[];
  export_ready: boolean;
}

export interface ViewerManifest {
  version_id: string;
  preview_low_url: string | null;
  preview_high_url: string | null;
  normalized_mesh_url: string | null;
  meshlib_scene_object_url: string | null;
  meshlib_scene_object_metadata: Record<string, unknown>;
  meshlib_scene_mru_url: string | null;
  meshlib_scene_mru_metadata: Record<string, unknown>;
  texture_artifact_url: string | null;
  texture_metadata: Record<string, unknown>;
  texture_artifacts: TextureArtifactManifest[];
  texture_per_face: number[];
  thickness_artifact_url: string | null;
  region_artifact_url: string | null;
  bounding_box: [number, number, number];
  default_material: MaterialType;
  available_overlays: string[];
  region_manifest: RegionManifestEntry[];
  measurements_summary: Record<string, unknown>;
  can_edit: boolean;
  needs_axis_confirmation: boolean;
}

export interface InteractiveSelectionPayload {
  mode: 'brush' | 'lasso' | 'rect' | 'pick' | 'faces' | 'vertices' | 'regions';
  vertex_ids: number[];
  face_ids: number[];
  region_ids: string[];
  brush_points_world: Array<[number, number, number]>;
  metadata: Record<string, unknown>;
}

export interface InteractiveCommitRequest {
  tool_id:
    | 'select_mark_region'
    | 'thicken_brush'
    | 'scoop_brush'
    | 'smooth_brush'
    | 'measure_inspect'
    | 'meshlib_workbench_export';
  operation_label: string;
  selection?: InteractiveSelectionPayload | null;
  brush_radius_mm?: number | null;
  falloff_mm?: number | null;
  target_thickness_mm?: number | null;
  depth_mm?: number | null;
  min_thickness_mm?: number | null;
  iterations?: number | null;
  strength?: number | null;
  preserve_detail: boolean;
  metadata: Record<string, unknown>;
}

export interface SelectionCommitRequest {
  tool_id: 'select_mark_region';
  operation_label: string;
  selection: InteractiveSelectionPayload;
  label?: string | null;
  create_object?: boolean;
  metadata: Record<string, unknown>;
}

export interface SelectionCommitResponse {
  version_id: string;
  artifact_id: string;
  artifact_url: string;
  artifact_type: 'meshlib_selection_json';
  selection_counts: Record<string, number>;
  resolved_counts: Record<string, number>;
  selected_object_version_id?: string | null;
  selected_object_artifact_id?: string | null;
  selected_object_artifact_url?: string | null;
  selected_object_artifact_type?: string | null;
  selected_object_counts?: Record<string, number> | null;
}

export interface BrushReplayStroke {
  tool_id: 'thicken_brush' | 'scoop_brush' | 'smooth_brush';
  selection: InteractiveSelectionPayload;
  amount_mm: number;
  falloff_mm: number;
  iterations: number;
  strength: number;
  metadata: Record<string, unknown>;
}

export interface BrushReplayRequest {
  operation_label: string;
  strokes: BrushReplayStroke[];
  metadata: Record<string, unknown>;
}

export interface MeasureInspectPair {
  start: [number, number, number];
  end: [number, number, number];
  label?: string | null;
  metric?: 'euclidean' | 'geodesic';
  start_vertex?: number | null;
  end_vertex?: number | null;
  control_vertices?: number[];
  close_path?: boolean;
  geodesic_max_path_len_mm?: number | null;
  include_refined_surface_path?: boolean;
}

export interface MeshCutMeasureTopologyRequest {
  control_vertices: number[];
  close_path?: boolean;
  max_path_len_mm?: number | null;
  operation_label?: string | null;
}

export interface MeshCutMeasureTopologyResponse {
  version: VersionSummary;
  source_version_id: string;
  artifact_id: string;
  artifact_url: string;
  control_vertices: number[];
  closed_path: boolean;
  length_mm: number;
  output_vertex_count: number;
  output_face_count: number;
  duplicate_vertex_map: number[][];
  source_path_vertex_indices: number[];
  result_cut_vertex_indices: number[][];
  cut_edge_pairs: number[][];
  result_cut_edge_pairs: number[][];
  bad_face_indices: number[];
  metadata: Record<string, unknown>;
}

export interface MeasureInspectSurfaceDistanceRequest {
  seed?: [number, number, number] | null;
  seed_vertex?: number | null;
  seed_vertices?: number[];
  seed_edges?: Array<[number, number]>;
  seed_face_ids?: number[];
  max_distance_mm?: number | null;
  iso_value_mm?: number | null;
  include_distances?: boolean;
  include_iso_segments?: boolean;
  include_extreme_edges?: boolean;
}

export type MeasureInspectFeatureKind = 'point' | 'sphere' | 'line' | 'plane' | 'circle' | 'cylinder' | 'cone';

export interface MeasureInspectFeaturePrimitive {
  feature_id: string;
  kind: MeasureInspectFeatureKind;
  center?: [number, number, number];
  direction?: [number, number, number] | null;
  normal?: [number, number, number] | null;
  radius_mm?: number;
  length_mm?: number;
}

export interface MeasureInspectFeaturePair {
  first_feature_id: string;
  second_feature_id: string;
  label?: string | null;
}

export interface MeasureInspectFeatureRefineRequest {
  feature_id: string;
  distance_limit_mm?: number;
  normal_tolerance_degrees?: number;
  max_iterations?: number;
  label?: string | null;
}

export interface MeasureInspectRequest {
  points: Array<[number, number, number]>;
  point_pairs: MeasureInspectPair[];
  features?: MeasureInspectFeaturePrimitive[];
  feature_pairs?: MeasureInspectFeaturePair[];
  feature_refinements?: MeasureInspectFeatureRefineRequest[];
  include_feature_objects?: boolean;
  feature_object_infinite_extent_mm?: number;
  surface_distance?: MeasureInspectSurfaceDistanceRequest | null;
  include_local_thickness: boolean;
}

export interface MeasureInspectPointResult {
  query_point: [number, number, number];
  closest_point: [number, number, number];
  face_index: number;
  distance_to_surface_mm: number;
  local_thickness_mm: number | null;
}

export interface MeasureInspectSurfacePathRefinement {
  start_vertex: number;
  end_vertex: number;
  start_face_index: number;
  end_face_index: number;
  shared_edge: [number, number];
  crossing_t: number;
  crossing_point: [number, number, number];
  points: Array<[number, number, number]>;
  edge_lengths_mm: number[];
  length_mm: number;
  graph_vertex_indices: number[];
  graph_length_mm: number;
  unfolded_quadrangle_convex: boolean;
  meshlib_reference: string;
}

export interface MeasureInspectPairResult {
  start: [number, number, number];
  end: [number, number, number];
  distance_mm: number;
  midpoint: [number, number, number];
  label?: string | null;
  metric: 'euclidean' | 'geodesic';
  control_vertex_indices: number[];
  control_vertex_offsets: number[];
  path_vertex_indices: number[];
  path_points: Array<[number, number, number]>;
  path_point_normals: Array<[number, number, number]>;
  edge_lengths_mm: number[];
  leg_lengths_mm: number[];
  leg_vertex_offsets: number[];
  line_segments: number;
  closed_path: boolean;
  path_object_lines?: Record<string, unknown> | null;
  path_object_points?: Record<string, unknown> | null;
  cut_contours?: Record<string, unknown> | null;
  surface_path_refinement?: MeasureInspectSurfacePathRefinement | null;
  meshlib_reference?: string | null;
}

export interface MeasureInspectSurfaceDistanceResult {
  seed: [number, number, number] | null;
  seed_vertex: number;
  seed_vertices: number[];
  seed_edges: Array<[number, number]>;
  seed_face_ids: number[];
  seed_face_boundary_edges: Array<[number, number]>;
  distances_mm: Array<number | null>;
  predecessor_vertices: Array<number | null>;
  reachable_vertex_count: number;
  max_distance_mm: number;
  iso_value_mm: number | null;
  selected_vertex_indices: number[];
  selected_face_indices: number[];
  crossing_face_indices: number[];
  boundary_edges: Array<[number, number]>;
  iso_segments: Array<[[number, number, number], [number, number, number]]>;
  ridge_edges: Array<[number, number]>;
  gorge_edges: Array<[number, number]>;
  clipped_vertices: Array<[number, number, number]>;
  clipped_faces: Array<[number, number, number]>;
  clipped_source_face_indices: number[];
  clipped_source_vertex_indices: Array<number | null>;
  meshlib_reference: string;
}

export interface MeasureInspectFeatureDistanceResult {
  status: 'ok' | 'bad_feature_pair' | 'bad_relative_location' | 'not_implemented' | 'not_finite';
  distance_mm?: number | null;
  closest_point_a?: [number, number, number] | null;
  closest_point_b?: [number, number, number] | null;
}

export interface MeasureInspectFeatureAngleResult {
  status: 'ok' | 'bad_feature_pair' | 'bad_relative_location' | 'not_implemented' | 'not_finite';
  angle_radians?: number | null;
  angle_degrees?: number | null;
  point_a?: [number, number, number] | null;
  point_b?: [number, number, number] | null;
  direction_a?: [number, number, number] | null;
  direction_b?: [number, number, number] | null;
  is_surface_normal_a: boolean;
  is_surface_normal_b: boolean;
}

export interface MeasureInspectFeatureIntersectionResult {
  kind: MeasureInspectFeatureKind;
  center: [number, number, number];
  direction?: [number, number, number] | null;
  radius_mm?: number | null;
  length_mm?: number | null;
  start_point?: [number, number, number] | null;
  end_point?: [number, number, number] | null;
  meshlib_primitive: string;
}

export interface MeasureInspectFeaturePairResult {
  first_feature_id: string;
  second_feature_id: string;
  first_kind: MeasureInspectFeatureKind;
  second_kind: MeasureInspectFeatureKind;
  label?: string | null;
  distance: MeasureInspectFeatureDistanceResult;
  center_distance: MeasureInspectFeatureDistanceResult;
  angle: MeasureInspectFeatureAngleResult;
  intersections: MeasureInspectFeatureIntersectionResult[];
  meshlib_reference: string;
}

export interface MeasureInspectFeatureObjectPropertyResult {
  name: string;
  kind: 'position' | 'linear_dimension' | 'direction' | 'angle' | 'other';
  scalar_value?: number | null;
  vector_value?: [number, number, number] | null;
}

export interface MeasureInspectFeatureObjectResult {
  feature_id: string;
  source_kind: MeasureInspectFeatureKind;
  object_type: 'PointObject' | 'SphereObject' | 'LineObject' | 'PlaneObject' | 'CircleObject' | 'CylinderObject';
  class_name: string;
  class_name_plural: string;
  shared_properties: MeasureInspectFeatureObjectPropertyResult[];
  meshlib_reference: string;
}

export interface MeasureInspectFeatureRefinementResult {
  feature_id: string;
  kind: MeasureInspectFeatureKind;
  label?: string | null;
  center: [number, number, number];
  direction?: [number, number, number] | null;
  radius_mm: number;
  length_mm: number;
  selected_vertex_indices: number[];
  selected_count: number;
  iterations: number;
  converged: boolean;
  feature_object?: MeasureInspectFeatureObjectResult | null;
  meshlib_reference: string;
}

export interface MeasureInspectResponse {
  version_id: string;
  points: MeasureInspectPointResult[];
  point_pairs: MeasureInspectPairResult[];
  feature_pairs: MeasureInspectFeaturePairResult[];
  feature_objects: MeasureInspectFeatureObjectResult[];
  feature_refinements: MeasureInspectFeatureRefinementResult[];
  surface_distance: MeasureInspectSurfaceDistanceResult | null;
}

export interface GcodeParsePathsRequest {
  source: string;
  machine_settings?: Record<string, unknown> | null;
}

export interface GcodeLoadSourceRequest {
  file_name?: string;
  source: string;
}

export interface GcodeSourceResponse {
  version_id: string;
  file_name: string;
  frame_count: number;
  source_frames: string[];
  metadata: Record<string, unknown>;
}

export interface GcodeWriteSourceRequest {
  file_name?: string;
  source_frames?: string[];
}

export interface GcodeParseFilePathsRequest extends GcodeLoadSourceRequest {
  machine_settings?: Record<string, unknown> | null;
}

export interface GcodeParsePathsResponse {
  version_id: string;
  frame_count: number;
  command_count: number;
  segment_count: number;
  max_feedrate: number;
  unit: string;
  segments: Array<Array<[number, number, number]>>;
  tool_directions: Array<Array<[number, number, number]>>;
  source_frame_indices: number[];
  idle: boolean[];
  feedrates: number[];
  warnings: string[];
  metadata: Record<string, unknown>;
}

export interface PointCloudIcpRequest {
  floating_points: Array<[number, number, number]>;
  reference_points: Array<[number, number, number]>;
  method?: 'point_to_point' | 'point_to_plane';
  mode?: 'rigid' | 'translation';
  max_iterations?: number;
  tolerance?: number;
  reference_normals?: Array<[number, number, number]> | null;
  floating_normals?: Array<[number, number, number]> | null;
  max_pair_distance?: number | null;
  cos_threshold?: number | null;
  far_dist_factor?: number | null;
  mutual_closest?: boolean;
}

export interface PointCloudIcpResponse {
  version_id: string;
  method: 'point_to_point' | 'point_to_plane';
  mode: 'rigid' | 'translation';
  rotation: number[][];
  translation: [number, number, number];
  transform: number[][];
  iterations: number;
  mean_square_distance: number;
  active_pair_count: number;
  metadata: Record<string, unknown>;
}

export interface OffsetContoursRequest {
  contours: Array<Array<[number, number, number]>>;
  offset?: number | null;
  offsets?: number[][] | null;
  min_angle_precision?: number;
  mode?: 'offset' | 'shell';
  end_type?: 'round' | 'cut';
  corner_type?: 'round' | 'sharp';
  max_sharp_angle?: number;
  z_restore?: 'default' | 'none' | 'constant' | 'custom';
  z_value?: number | null;
  z_values?: number[][] | null;
  relax_iterations?: number;
  include_origins?: boolean;
}

export interface OffsetContoursResponse {
  version_id: string;
  contour_count: number;
  point_count: number;
  contours: Array<Array<[number, number, number]>>;
  origins: unknown[];
  metadata: Record<string, unknown>;
}

export interface DistanceMapContoursRequest {
  contours: Array<Array<[number, number]>>;
  width: number;
  height: number;
  origin?: [number, number];
  pixel_size?: number | [number, number];
  signed?: boolean;
}

export interface DistanceMapFromMeshRequest {
  width: number;
  height: number;
  origin: [number, number, number];
  x_range: [number, number, number];
  y_range: [number, number, number];
  direction: [number, number, number];
  epsilon?: number;
}

export interface DistanceMapResponse {
  version_id: string;
  width: number;
  height: number;
  origin: [number, number];
  pixel_size: [number, number];
  valid_count: number;
  min_value: number;
  max_value: number;
  values: number[][];
  model_transform: number[] | null;
  unit: string;
  metadata: Record<string, unknown>;
}

export interface DistanceMapIsoLinesRequest {
  width: number;
  height: number;
  origin?: [number, number];
  pixel_size?: [number, number];
  values: number[][];
  valid_count?: number | null;
  min_value?: number | null;
  max_value?: number | null;
  model_transform?: number[] | null;
  unit?: string;
  iso_value?: number;
}

export interface DistanceMapPayload {
  width: number;
  height: number;
  origin?: [number, number];
  pixel_size?: [number, number];
  values: number[][];
  valid_count?: number | null;
  min_value?: number | null;
  max_value?: number | null;
  model_transform?: number[] | null;
  unit?: string;
}

export interface DistanceMapMergeRequest {
  left: DistanceMapPayload;
  right: DistanceMapPayload;
  mode?: 'min' | 'max' | 'subtract';
}

export interface DistanceMapContourBooleanRequest {
  contours_a: Array<Array<[number, number]>>;
  contours_b: Array<Array<[number, number]>>;
  mode?: 'union' | 'intersection' | 'subtract';
  width: number;
  height: number;
  origin?: [number, number];
  pixel_size?: [number, number];
  iso_value?: number;
}

export interface DistanceMapTiffImportRequest {
  file_name?: string;
  contents_base64: string;
}

export interface DistanceMapTiffExportRequest extends DistanceMapPayload {
  file_name?: string;
}

export interface DistanceMapTiffExportResponse {
  version_id: string;
  file_name: string;
  byte_count: number;
  contents_base64: string;
  metadata: Record<string, unknown>;
}

export interface IsoLineSegmentsResponse {
  version_id: string;
  iso_value: number;
  segment_count: number;
  segments: Array<[[number, number], [number, number]]>;
  unit: string;
  metadata: Record<string, unknown>;
}

export interface ObjectLinesFromContoursRequest {
  contours: Array<Array<[number, number, number]>>;
  line_width?: number;
  show_points?: number;
  smooth_connections?: number;
}

export interface ObjectLinesPtsLoadRequest {
  file_name?: string;
  source: string;
}

export interface ObjectLinesSvgLoadRequest {
  file_name?: string;
  source: string;
}

export interface ObjectLinesBinaryLoadRequest {
  file_name?: string;
  contents_base64: string;
}

export interface ObjectLinesResponse {
  version_id: string;
  point_count: number;
  line_count: number;
  line_width: number;
  object_lines: Record<string, unknown>;
  metadata: Record<string, unknown>;
}

export interface ObjectLinesTextExportRequest {
  file_name?: string;
  object_lines: Record<string, unknown>;
}

export interface ObjectLinesTextExportResponse {
  version_id: string;
  file_name: string;
  source: string;
  byte_count: number;
  metadata: Record<string, unknown>;
}

export interface ObjectLinesBinaryExportRequest {
  file_name?: string;
  object_lines: Record<string, unknown>;
}

export interface ObjectLinesBinaryExportResponse {
  version_id: string;
  file_name: string;
  byte_count: number;
  contents_base64: string;
  metadata: Record<string, unknown>;
}

export interface ObjectLinesToContoursRequest {
  object_lines: Record<string, unknown>;
}

export interface ObjectLinesToContoursResponse {
  version_id: string;
  contour_count: number;
  point_count: number;
  contours: Array<Array<[number, number, number]>>;
  metadata: Record<string, unknown>;
}

export interface MeshToVoxelsSdfRequest {
  voxel_size_mm: number;
  surface_offset_voxels: number;
  mode: 'signed' | 'unsigned';
  iso_value?: number;
  extract_surface: boolean;
}

export interface MeshToVoxelsSdfResponse {
  version_id: string;
  mode: 'signed' | 'unsigned';
  voxel_size_mm: number;
  surface_offset_voxels: number;
  padding_mm: number;
  iso_value: number;
  origin: [number, number, number];
  shape: [number, number, number];
  value_count: number;
  active_voxel_count: number;
  min_value: number;
  max_value: number;
  estimated_volume_mm3: number;
  surface_vertex_count: number;
  surface_face_count: number;
  metadata: Record<string, unknown>;
}

export interface VoxelRawLoadRequest {
  file_name?: string;
  contents_base64: string;
  dimensions?: [number, number, number] | null;
  voxel_size?: [number, number, number];
  scalar_type?: string;
  grid_level_set?: boolean;
  auto_parameters?: boolean;
}

export interface VoxelTiffLoadRequest {
  files: Record<string, string>;
  voxel_size?: [number, number, number];
  grid_level_set?: boolean;
}

export interface VoxelVolumeLoadResponse {
  version_id: string;
  dimensions: [number, number, number];
  voxel_size: [number, number, number];
  grid_level_set: boolean;
  scalar_type: string;
  value_count: number;
  values: number[];
  min_value: number;
  max_value: number;
  default_iso_value: number | null;
  metadata: Record<string, unknown>;
}

export interface VoxelLineGraphRequest {
  values: number[];
  shape: [number, number, number];
  axis: string;
  fixed_coordinate: [number, number, number];
}

export interface VoxelLineGraphResponse {
  version_id: string;
  axis: number;
  positions: number[];
  voxel_indices: number[];
  coordinates: Array<[number, number, number]>;
  values: number[];
  metadata: Record<string, unknown>;
}

export interface VoxelActiveBoxRequest {
  values: number[];
  shape: [number, number, number];
  min_corner: [number, number, number];
  dimensions: [number, number, number];
}

export interface VoxelActiveBoxResponse {
  version_id: string;
  min_corner: [number, number, number];
  dimensions: [number, number, number];
  source_indices: number[];
  coordinates: Array<[number, number, number]>;
  values: number[];
  metadata: Record<string, unknown>;
}

export interface VoxelBinaryOperationsRequest {
  left_values: number[];
  right_values: number[];
  shape: [number, number, number];
  origin?: [number, number, number];
  voxel_size_mm?: number;
  operation: string;
  left_iso_value?: number;
  right_iso_value?: number;
}

export interface VoxelBinaryOperationsResponse {
  version_id: string;
  operation: string;
  shape: [number, number, number];
  origin: [number, number, number];
  voxel_size_mm: number;
  values: number[];
  result_iso_value: number;
  min_value: number;
  max_value: number;
  metadata: Record<string, unknown>;
}

export interface VoxelSliceRequest {
  values: number[];
  shape: [number, number, number];
  plane: string;
  slice_index: number;
  min_value: number;
  max_value: number;
}

export interface VoxelSliceResponse {
  version_id: string;
  width: number;
  height: number;
  values: number[];
  normalized_values: number[];
  coordinates: Array<[number, number, number]>;
  metadata: Record<string, unknown>;
}

export interface VoxelPathRequest {
  values: number[];
  shape: [number, number, number];
  start: [number, number, number];
  finish: [number, number, number];
  metric?: string;
  max_dist_ratio?: number;
  plane?: string;
  quarters_mask?: number;
  exponent_modifier?: number;
}

export interface VoxelPathPayload {
  voxel_indices: number[];
  coordinates: Array<[number, number, number]>;
  total_metric: number;
}

export interface VoxelPathResponse extends VoxelPathPayload {
  version_id: string;
  metadata: Record<string, unknown>;
}

export interface VoxelPathBuildFourRequest {
  values: number[];
  shape: [number, number, number];
  start: [number, number, number];
  finish: [number, number, number];
  metric?: string;
  max_dist_ratio?: number;
  plane?: string;
  exponent_modifier?: number;
}

export interface VoxelPathBuildFourEntry {
  quarters_mask: number;
  path: VoxelPathPayload;
}

export interface VoxelPathBuildFourResponse {
  version_id: string;
  paths: VoxelPathBuildFourEntry[];
  metadata: Record<string, unknown>;
}

export interface VoxelToMeshSimpleRequest {
  values: number[];
  shape: [number, number, number];
  voxel_size?: [number, number, number];
  iso_value?: number | null;
  grid_level_set?: boolean;
  scalar_type?: string;
  min_value?: number | null;
  max_value?: number | null;
}

export interface VoxelToMeshSimpleResponse {
  version_id: string;
  vertex_count: number;
  face_count: number;
  bounds_min: [number, number, number];
  bounds_max: [number, number, number];
  vertices: Array<[number, number, number]>;
  faces: Array<[number, number, number]>;
  metadata: Record<string, unknown>;
}

export interface VoxelToMeshDualRequest extends Omit<VoxelToMeshSimpleRequest, 'values'> {
  values?: number[];
  model_bytes_base64?: string | null;
  model_extension?: string;
  adaptivity?: number;
  relax_disoriented_triangles?: boolean;
  max_faces?: number | null;
  max_vertices?: number | null;
}
export type VoxelToMeshDualResponse = VoxelToMeshSimpleResponse;

export interface VoxelToMeshSmartRequest extends VoxelToMeshSimpleRequest {
  iters?: number;
  sample_points?: number;
  degree?: number;
  outlier_threshold?: number;
  intermediate_smooth_force?: number;
  preparation_smooth_force?: number;
  smooth_shift_iterations?: number;
  final_relax_iterations?: number;
  final_relax_force?: number;
}

export type VoxelToMeshSmartResponse = VoxelToMeshSimpleResponse;

export interface VoxelSegmentationRequest {
  values: number[];
  shape: [number, number, number];
  voxel_size?: [number, number, number];
  inside_seeds: Array<[number, number, number]>;
  outside_seeds?: Array<[number, number, number]>;
  exponent_modifier?: number;
  voxels_expansion?: number;
  include_boundary_outside?: boolean;
}

export type VoxelSegmentationResponse = VoxelToMeshSimpleResponse;

export interface VoxelMaskToMeshRequest {
  values: number[];
  shape: [number, number, number];
  voxel_size?: [number, number, number];
  mask_coordinates: Array<[number, number, number]>;
  mask_expansion?: number;
  smooth_band_radius?: number;
}

export type VoxelMaskToMeshResponse = VoxelToMeshSimpleResponse;

export interface VoxelVolumeRenderDataRequest {
  values: number[];
  shape: [number, number, number];
  voxel_size?: [number, number, number];
  active_min_corner?: [number, number, number];
  active_dimensions?: [number, number, number] | null;
  source_min_value?: number | null;
  source_max_value?: number | null;
}

export interface VoxelVolumeRenderDataResponse {
  version_id: string;
  dimensions: [number, number, number];
  voxel_size: [number, number, number];
  source_indices: number[];
  coordinates: Array<[number, number, number]>;
  values: number[];
  min_value: number;
  max_value: number;
  metadata: Record<string, unknown>;
}

export interface VoxelVolumeRenderLutRequest {
  lut_type?: string;
  alpha_type?: string;
  alpha_limit?: number;
  one_color?: [number, number, number, number] | null;
}

export interface VoxelVolumeRenderLutResponse {
  version_id: string;
  lut_type: string;
  alpha_type: string;
  alpha_limit: number;
  colors_rgba: Array<[number, number, number, number]>;
  metadata: Record<string, unknown>;
}

export interface VoxelVolumeRenderRayRequest {
  values: number[];
  shape: [number, number, number];
  voxel_size?: [number, number, number];
  min_corner?: [number, number, number];
  ray_start: [number, number, number];
  ray_direction: [number, number, number];
  sampling_step: number;
  min_value?: number;
  max_value?: number;
  lut_type?: string;
  alpha_type?: string;
  alpha_limit?: number;
  one_color?: [number, number, number, number] | null;
  clipping_plane?: [number, number, number, number] | null;
  shading_mode?: string;
  light_pos_eye?: [number, number, number] | null;
  ambient_strength?: number;
  specular_strength?: number;
  spec_exp?: number;
  active_indices?: number[] | null;
  max_steps?: number;
}

export interface VoxelVolumeRenderRayResponse {
  version_id: string;
  color_rgba: number[];
  first_opaque_world: [number, number, number] | null;
  visited_indices: number[];
  accepted_indices: number[];
  metadata: Record<string, unknown>;
}

export interface OffsetMeshRequest {
  offset_mm: number;
  voxel_size_mm: number;
  padding_mm?: number | null;
  refine?: boolean;
}

export interface OffsetSmoothingRequest {
  distance_mm: number;
  voxel_size_mm: number;
  padding_mm?: number | null;
  refine?: boolean;
}

export interface ShellMeshRequest {
  wall_thickness_mm: number;
  voxel_size_mm: number;
  padding_mm?: number | null;
  refine?: boolean;
}

export interface ThickenMeshRequest {
  thickness_mm: number;
  voxel_size_mm: number;
  padding_mm?: number | null;
  refine?: boolean;
}

export interface WeightedShellRegionWeight {
  region_id: string;
  weight_mm: number;
}

export interface WeightedShellRequest {
  offset_mm: number;
  region_weights: WeightedShellRegionWeight[];
  voxel_size_mm: number;
  padding_mm?: number | null;
  interpolation_distance_mm?: number;
  refine?: boolean;
}

export interface PartialOffsetRequest {
  offset_mm: number;
  region_ids: string[];
  voxel_size_mm: number;
  padding_mm?: number | null;
  refine?: boolean;
}

export interface OffsetVertsRequest {
  offset_mm: number;
  region_ids: string[];
}

export interface OffsetShellMeshResponse {
  version: VersionSummary;
  source_version_id: string;
  mode:
    | 'offset'
    | 'shell'
    | 'thicken'
    | 'weighted_shell'
    | 'partial_offset'
    | 'offset_verts'
    | 'expand_shrink'
    | 'shrink_expand';
  offset_mm: number | null;
  distance_mm: number | null;
  wall_thickness_mm: number | null;
  thickness_mm: number | null;
  region_weights: Record<string, number> | null;
  selected_region_ids: string[] | null;
  voxel_size_mm: number;
  padding_mm: number | null;
  refine: boolean;
  artifact_id: string;
  artifact_url: string;
  output_vertex_count: number;
  output_face_count: number;
  metadata: Record<string, unknown>;
}

export type ExactBooleanOperation =
  | 'union'
  | 'intersection'
  | 'difference'
  | 'difference_ab'
  | 'difference_ba'
  | 'inside_a'
  | 'inside_b'
  | 'outside_a'
  | 'outside_b';

export interface ExactBooleanRequest {
  other_version_id: string;
  operation: ExactBooleanOperation;
  epsilon?: number;
}

export interface ExactBooleanResponse {
  version: VersionSummary;
  source_version_id: string;
  other_version_id: string;
  operation: ExactBooleanOperation | string;
  artifact_id: string;
  artifact_url: string;
  output_vertex_count: number;
  output_face_count: number;
  diagnostics: Record<string, unknown>;
  metadata: Record<string, unknown>;
}

export type VoxelBooleanOperation = 'union' | 'intersection' | 'difference';

export interface VoxelBooleanRequest {
  other_version_id: string;
  operation: VoxelBooleanOperation;
  voxel_size_mm: number;
  padding_mm?: number | null;
  refine?: boolean;
}

export interface VoxelBooleanResponse {
  version: VersionSummary;
  source_version_id: string;
  other_version_id: string;
  operation: VoxelBooleanOperation | string;
  voxel_size_mm: number;
  padding_mm: number | null;
  refine: boolean;
  artifact_id: string;
  artifact_url: string;
  output_vertex_count: number;
  output_face_count: number;
  metadata: Record<string, unknown>;
}

export interface CollisionDetectRequest {
  other_version_id: string;
  first_intersection_only: boolean;
  max_pairs?: number;
  epsilon?: number;
}

export interface CollisionFacePair {
  first_face: number;
  second_face: number;
  intersection_count: number;
}

export interface CollisionDetectResponse {
  version_id: string;
  other_version_id: string;
  colliding: boolean;
  pair_count: number;
  first_face_indices: number[];
  second_face_indices: number[];
  pairs: CollisionFacePair[];
  truncated: boolean;
  metadata: Record<string, unknown>;
}

export interface MeshLibWorkbenchCommandCapability {
  command_id: string;
  label: string;
  group: 'file' | 'prepare' | 'modify' | 'inspect' | 'review' | 'runtime';
  endpoint_url_key: string | null;
  endpoint_url: string | null;
  runtime_tool_id: string | null;
  rust_backed: boolean;
  sdk_operations: string[];
  notes: string[];
}

export interface MeshLibOfficialParityFeature {
  official_feature_id: string;
  label: string;
  group:
    | 'file'
    | 'selection'
    | 'repair'
    | 'edit'
    | 'boolean'
    | 'offset'
    | 'inspect'
    | 'compare'
    | 'point_cloud'
    | 'voxels'
    | 'distance_map'
    | 'automation';
  status: 'implemented' | 'partial' | 'missing';
  official_sources: string[];
  meshlib_source_paths: string[];
  rust_owner_modules: string[];
  bridge_modules: string[];
  backend_command_ids: string[];
  hosted_tool_ids: string[];
  validation_gates: string[];
  non_geometry_reason: string | null;
  notes: string[];
}

export interface TextureArtifactManifest {
  texture_index: number;
  artifact_url: string;
  metadata: Record<string, unknown>;
}

export interface MeshLibWorkbenchManifest {
  version_id: string;
  entry_html_url: string;
  runtime_asset_base_url: string;
  normalized_mesh_url: string | null;
  meshlib_scene_object_url: string | null;
  meshlib_scene_object_metadata: Record<string, unknown>;
  meshlib_scene_mru_url: string | null;
  meshlib_scene_mru_metadata: Record<string, unknown>;
  texture_artifact_url: string | null;
  texture_metadata: Record<string, unknown>;
  texture_artifacts: TextureArtifactManifest[];
  texture_per_face: number[];
  preview_low_url: string | null;
  preview_high_url: string | null;
  commit_endpoint_url: string;
  selection_endpoint_url: string;
  brush_endpoint_url: string;
  measurement_endpoint_url: string;
  mesh_cut_measure_topology_endpoint_url: string;
  built_in_ui: string[];
  interactive_tools: string[];
  command_capabilities: MeshLibWorkbenchCommandCapability[];
  official_parity_inventory: MeshLibOfficialParityFeature[];
  feature_flags: Record<string, boolean>;
  notes: string[];
}

export interface MeshLibRuntimeManifest {
  status: 'missing' | 'ready';
  message?: string;
  entry_html_url?: string;
  entry_js_url?: string;
  entry_wasm_url?: string;
}

export interface VersionDetailResponse {
  version: VersionSummary;
  artifacts: ArtifactSummary[];
  latest_snapshot: ManufacturabilitySnapshot | null;
}

export interface ResizeRequestV2 {
  target_ring_size_us: number;
  axis_mode: 'auto' | 'manual';
  manual_axis?: [number, number, number];
  preserve_head: boolean;
}

export interface HollowRequestV2 {
  mode: 'fixed_thickness' | 'target_weight';
  processing_mode?: 'interactive' | 'full_resolution';
  material: MaterialType;
  wall_thickness_mm?: number;
  target_weight_g?: number;
  min_allowed_thickness_mm: number;
  protect_regions: Array<'head' | 'gem_seat' | 'ornament_relief' | 'inner_band'>;
  add_drain_holes: boolean;
}

export interface ThickenRequestV2 {
  mode: 'global' | 'violations_only' | 'selected_region' | 'selected_regions';
  min_target_thickness_mm: number;
  region_id?: string;
  region_ids?: string[];
  smoothing_pass: boolean;
}

export interface CompareRequestV2 {
  other_version_id: string;
}

export interface CompareSummary {
  version_id: string;
  other_version_id: string;
  volume_delta_mm3: number;
  weight_delta_g: number;
  bbox_delta_mm: [number, number, number];
  min_signed_distance_mm?: number | null;
  max_signed_distance_mm?: number | null;
  mean_signed_distance_mm?: number | null;
}

export interface SmoothRequestV2 {
  region_id?: string | null;
  region_ids?: string[] | null;
  iterations: number;
  strength: number;
  global_mode: boolean;
}

export interface DecimateRequestV2 {
  strategy: 'minimize_error' | 'shortest_edge_first';
  max_error: number;
  target_face_count?: number | null;
  target_face_ratio?: number | null;
  max_edge_len?: number | null;
  max_bd_shift?: number | null;
  stabilizer: number;
  subdivide_parts: number;
  decimate_between_parts: boolean;
  region_faces?: number[];
  not_flippable_edges?: [number, number][];
  collapse_near_not_flippable: boolean;
  angle_weighted_dist_to_plane: boolean;
  max_deleted_vertices: number;
  max_deleted_faces: number;
  max_triangle_aspect_ratio: number;
  touch_near_bd_edges: boolean;
  touch_bd_verts: boolean;
  optimize_vertex_pos: boolean;
  pack_mesh: boolean;
  metadata?: Record<string, unknown>;
}

export interface SubdivideRequestV2 {
  max_edge_len: number;
  max_edge_splits: number;
  region_faces?: number[];
  not_flippable_edges?: [number, number][];
  subdivide_border: boolean;
  curvature_priority: number;
  project_on_original_mesh: boolean;
  smooth_mode: boolean;
  min_sharp_dihedral_angle: number;
  max_tri_aspect_ratio: number;
  max_splittable_tri_aspect_ratio: number | null;
  max_deviation_after_flip?: number | null;
  max_angle_change_after_flip?: number | null;
  critical_tri_aspect_ratio_flip?: number | null;
}

export interface MakeDeloneRequestV2 {
  num_iters: number;
  region_faces?: number[];
  max_deviation_after_flip?: number | null;
  max_angle_change?: number | null;
  critical_tri_aspect_ratio?: number | null;
  not_flippable_edges?: [number, number][];
  vert_region?: number[];
  metadata?: Record<string, unknown>;
}

export interface ScoopRequestV2 {
  region_id: string;
  depth_mm: number;
  falloff_mm: number;
  keep_min_thickness_mm: number;
}

export interface MakeManufacturableRequest {
  material: MaterialType;
  target_ring_size_us?: number;
  target_weight_g?: number;
  min_allowed_thickness_mm: number;
}
