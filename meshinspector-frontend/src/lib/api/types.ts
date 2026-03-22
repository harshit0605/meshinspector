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

export interface SmoothRequestV2 {
  region_id?: string | null;
  region_ids?: string[] | null;
  iterations: number;
  strength: number;
  global_mode: boolean;
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
