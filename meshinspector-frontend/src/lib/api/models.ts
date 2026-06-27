/**
 * API methods for model and version operations.
 */

import { fetchApi } from './client';
import type {
  BranchVersionRequest,
  BrushReplayRequest,
  CollisionDetectRequest,
  CollisionDetectResponse,
  CompareCacheEntry,
  CompareRequestV2,
  CompareSummary,
  CreateModelResponse,
  DecimateRequestV2,
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
  HollowRequestV2,
  InspectionSnapshotResponse,
  InspectionSnapshotState,
  InteractiveCommitRequest,
  IsoLineSegmentsResponse,
  JobResponse,
  JobEventResponse,
  MakeDeloneRequestV2,
  MakeManufacturableRequest,
  ManufacturabilitySnapshot,
  MeasureInspectRequest,
  MeasureInspectResponse,
  MeshCutMeasureTopologyRequest,
  MeshCutMeasureTopologyResponse,
  MeshToVoxelsSdfRequest,
  MeshToVoxelsSdfResponse,
  MeshLibWorkbenchManifest,
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
  OffsetContoursRequest,
  OffsetContoursResponse,
  OffsetMeshRequest,
  OffsetShellMeshResponse,
  OffsetSmoothingRequest,
  OffsetVertsRequest,
  PartialOffsetRequest,
  PointCloudIcpRequest,
  PointCloudIcpResponse,
  ResizeRequestV2,
  SectionContourPayload,
  SelectionCommitRequest,
  SelectionCommitResponse,
  ShellMeshRequest,
  ThickenMeshRequest,
  VersionSummary,
  WeightedShellRequest,
  ScoopRequestV2,
  ScalarOverlayResponse,
  SmoothRequestV2,
  SubdivideRequestV2,
  VersionDetailResponse,
  ViewerManifest,
  ThickenRequestV2,
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
  VoxelPathBuildFourRequest,
  VoxelPathBuildFourResponse,
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
} from './types';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

export async function uploadModel(file: File): Promise<CreateModelResponse> {
  const formData = new FormData();
  formData.append('file', file);

  let response: Response;
  try {
    response = await fetch(`${API_BASE}/api/models`, {
      method: 'POST',
      body: formData,
    });
  } catch (error) {
    throw new Error(
      error instanceof Error && error.message
        ? `Upload request failed: ${error.message}`
        : 'Upload request failed: backend unavailable or network request blocked'
    );
  }

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.detail || 'Upload failed');
  }

  return response.json();
}

export async function getModel(modelId: string) {
  return fetchApi(`/api/models/${modelId}`);
}

export async function getModelVersions(modelId: string): Promise<VersionSummary[]> {
  return fetchApi(`/api/models/${modelId}/versions`);
}

export async function getVersion(versionId: string): Promise<VersionDetailResponse> {
  return fetchApi(`/api/versions/${versionId}`);
}

export async function branchVersion(versionId: string, params: BranchVersionRequest): Promise<VersionSummary> {
  return fetchApi(`/api/versions/${versionId}/branch`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function getManufacturability(versionId: string): Promise<ManufacturabilitySnapshot> {
  return fetchApi(`/api/versions/${versionId}/manuf`);
}

export async function getViewerManifest(versionId: string): Promise<ViewerManifest> {
  return fetchApi(`/api/versions/${versionId}/viewer`);
}

export async function getMeshLibWorkbenchManifest(versionId: string): Promise<MeshLibWorkbenchManifest> {
  return fetchApi(`/api/versions/${versionId}/meshlib-workbench`);
}

export async function getSectionContour(
  versionId: string,
  params: {
    section_constant: number;
    plane_axis: [number, number, number];
    selected_region_ids?: string[];
  },
): Promise<SectionContourPayload> {
  const search = new URLSearchParams();
  search.set('section_constant', params.section_constant.toString());
  search.set('axis_x', params.plane_axis[0].toString());
  search.set('axis_y', params.plane_axis[1].toString());
  search.set('axis_z', params.plane_axis[2].toString());
  if (params.selected_region_ids?.length) {
    search.set('selected_region_ids', params.selected_region_ids.join(','));
  }
  return fetchApi(`/api/versions/${versionId}/section?${search.toString()}`);
}

export async function submitMeasureInspect(
  versionId: string,
  params: MeasureInspectRequest,
): Promise<MeasureInspectResponse> {
  return fetchApi(`/api/versions/${versionId}/measure-inspect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitMeshCutMeasureTopology(
  versionId: string,
  params: MeshCutMeasureTopologyRequest,
): Promise<MeshCutMeasureTopologyResponse> {
  return fetchApi(`/api/versions/${versionId}/mesh-cut-measure/topology`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitGcodeParsePaths(
  versionId: string,
  params: GcodeParsePathsRequest,
): Promise<GcodeParsePathsResponse> {
  return fetchApi(`/api/versions/${versionId}/gcode/parse-paths`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitGcodeLoadSource(
  versionId: string,
  params: GcodeLoadSourceRequest,
): Promise<GcodeSourceResponse> {
  return fetchApi(`/api/versions/${versionId}/gcode/load-source`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitGcodeWriteSource(
  versionId: string,
  params: GcodeWriteSourceRequest,
): Promise<GcodeSourceResponse> {
  return fetchApi(`/api/versions/${versionId}/gcode/write-source`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitGcodeParseFilePaths(
  versionId: string,
  params: GcodeParseFilePathsRequest,
): Promise<GcodeParsePathsResponse> {
  return fetchApi(`/api/versions/${versionId}/gcode/parse-file-paths`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitPointCloudIcp(
  versionId: string,
  params: PointCloudIcpRequest,
): Promise<PointCloudIcpResponse> {
  return fetchApi(`/api/versions/${versionId}/point-cloud/icp`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitOffsetContours(
  versionId: string,
  params: OffsetContoursRequest,
): Promise<OffsetContoursResponse> {
  return fetchApi(`/api/versions/${versionId}/contours/offset`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapContours(
  versionId: string,
  params: DistanceMapContoursRequest,
): Promise<DistanceMapResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/contours`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapFromMesh(
  versionId: string,
  params: DistanceMapFromMeshRequest,
): Promise<DistanceMapResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/mesh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapIsoLines(
  versionId: string,
  params: DistanceMapIsoLinesRequest,
): Promise<IsoLineSegmentsResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/iso-lines`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapMerge(
  versionId: string,
  params: DistanceMapMergeRequest,
): Promise<DistanceMapResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/merge`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapContourBoolean(
  versionId: string,
  params: DistanceMapContourBooleanRequest,
): Promise<IsoLineSegmentsResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/contour-boolean`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapFromTiff(
  versionId: string,
  params: DistanceMapTiffImportRequest,
): Promise<DistanceMapResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/from-tiff`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDistanceMapToTiff(
  versionId: string,
  params: DistanceMapTiffExportRequest,
): Promise<DistanceMapTiffExportResponse> {
  return fetchApi(`/api/versions/${versionId}/distance-map/to-tiff`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesFromContours(
  versionId: string,
  params: ObjectLinesFromContoursRequest,
): Promise<ObjectLinesResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/from-contours`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesLoadPts(
  versionId: string,
  params: ObjectLinesPtsLoadRequest,
): Promise<ObjectLinesResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/load-pts`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesLoadMrLines(
  versionId: string,
  params: ObjectLinesBinaryLoadRequest,
): Promise<ObjectLinesResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/load-mrlines`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesLoadPly(
  versionId: string,
  params: ObjectLinesBinaryLoadRequest,
): Promise<ObjectLinesResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/load-ply`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesSaveMrLines(
  versionId: string,
  params: ObjectLinesBinaryExportRequest,
): Promise<ObjectLinesBinaryExportResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/save-mrlines`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesSavePly(
  versionId: string,
  params: ObjectLinesBinaryExportRequest,
): Promise<ObjectLinesBinaryExportResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/save-ply`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesSavePts(
  versionId: string,
  params: ObjectLinesTextExportRequest,
): Promise<ObjectLinesTextExportResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/save-pts`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesLoadSvg(
  versionId: string,
  params: ObjectLinesSvgLoadRequest,
): Promise<ObjectLinesResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/load-svg`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesSaveDxf(
  versionId: string,
  params: ObjectLinesTextExportRequest,
): Promise<ObjectLinesTextExportResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/save-dxf`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitObjectLinesToContours(
  versionId: string,
  params: ObjectLinesToContoursRequest,
): Promise<ObjectLinesToContoursResponse> {
  return fetchApi(`/api/versions/${versionId}/object-lines/to-contours`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitMeshToVoxelsSdf(
  versionId: string,
  params: MeshToVoxelsSdfRequest,
): Promise<MeshToVoxelsSdfResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/mesh-to-sdf`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitOpenRawVoxels(
  versionId: string,
  params: VoxelRawLoadRequest,
): Promise<VoxelVolumeLoadResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/open-raw`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitOpenVoxelsFromTiff(
  versionId: string,
  params: VoxelTiffLoadRequest,
): Promise<VoxelVolumeLoadResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/open-tiff-dir`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelBinaryOperations(
  versionId: string,
  params: VoxelBinaryOperationsRequest,
): Promise<VoxelBinaryOperationsResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/binary`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelLineGraph(
  versionId: string,
  params: VoxelLineGraphRequest,
): Promise<VoxelLineGraphResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/line-graph`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelActiveBox(
  versionId: string,
  params: VoxelActiveBoxRequest,
): Promise<VoxelActiveBoxResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/active-box`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelSlice(
  versionId: string,
  params: VoxelSliceRequest,
): Promise<VoxelSliceResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/slice`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelPath(
  versionId: string,
  params: VoxelPathRequest,
): Promise<VoxelPathResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/path`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelPathBuildFour(
  versionId: string,
  params: VoxelPathBuildFourRequest,
): Promise<VoxelPathBuildFourResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/path/build-four`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelSegmentation(
  versionId: string,
  params: VoxelSegmentationRequest,
): Promise<VoxelSegmentationResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/segmentation`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelMaskToMesh(
  versionId: string,
  params: VoxelMaskToMeshRequest,
): Promise<VoxelMaskToMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/mask-to-mesh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelToMeshSimple(
  versionId: string,
  params: VoxelToMeshSimpleRequest,
): Promise<VoxelToMeshSimpleResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/to-mesh/simple`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelToMeshDual(
  versionId: string,
  params: VoxelToMeshDualRequest,
): Promise<VoxelToMeshDualResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/to-mesh/dual`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelToMeshSmart(
  versionId: string,
  params: VoxelToMeshSmartRequest,
): Promise<VoxelToMeshSmartResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/to-mesh/smart`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelVolumeRenderData(
  versionId: string,
  params: VoxelVolumeRenderDataRequest,
): Promise<VoxelVolumeRenderDataResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/volume-render-data`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelVolumeRenderLut(
  versionId: string,
  params: VoxelVolumeRenderLutRequest,
): Promise<VoxelVolumeRenderLutResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/volume-render-lut`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelVolumeRenderRay(
  versionId: string,
  params: VoxelVolumeRenderRayRequest,
): Promise<VoxelVolumeRenderRayResponse> {
  return fetchApi(`/api/versions/${versionId}/voxels/volume-render-ray`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitOffsetMesh(
  versionId: string,
  params: OffsetMeshRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/voxel`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitShellMesh(
  versionId: string,
  params: ShellMeshRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/shell/voxel`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitThickenMesh(
  versionId: string,
  params: ThickenMeshRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/thicken`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitWeightedShell(
  versionId: string,
  params: WeightedShellRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/weighted-shell`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitPartialOffset(
  versionId: string,
  params: PartialOffsetRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/partial`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitOffsetVerts(
  versionId: string,
  params: OffsetVertsRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/verts`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitExpandShrink(
  versionId: string,
  params: OffsetSmoothingRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/expand-shrink`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitShrinkExpand(
  versionId: string,
  params: OffsetSmoothingRequest,
): Promise<OffsetShellMeshResponse> {
  return fetchApi(`/api/versions/${versionId}/offset/shrink-expand`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitExactBoolean(
  versionId: string,
  params: ExactBooleanRequest,
): Promise<ExactBooleanResponse> {
  return fetchApi(`/api/versions/${versionId}/boolean/exact`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitVoxelBoolean(
  versionId: string,
  params: VoxelBooleanRequest,
): Promise<VoxelBooleanResponse> {
  return fetchApi(`/api/versions/${versionId}/boolean/voxel`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitCollisionDetect(
  versionId: string,
  params: CollisionDetectRequest,
): Promise<CollisionDetectResponse> {
  return fetchApi(`/api/versions/${versionId}/collision/detect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitSelectionCommit(
  versionId: string,
  params: SelectionCommitRequest,
): Promise<SelectionCommitResponse> {
  return fetchApi(`/api/versions/${versionId}/selection-commit`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitBrushReplay(
  versionId: string,
  params: BrushReplayRequest,
): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/brush-replay`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function getThicknessOverlay(versionId: string): Promise<ScalarOverlayResponse> {
  return fetchApi(`/api/versions/${versionId}/overlays/thickness`);
}

export async function getCompareOverlay(versionId: string, otherVersionId: string): Promise<ScalarOverlayResponse> {
  return fetchApi(`/api/versions/${versionId}/overlays/compare/${otherVersionId}`);
}

export async function getCompareCache(versionId: string): Promise<CompareCacheEntry[]> {
  return fetchApi(`/api/versions/${versionId}/compare-cache`);
}

export async function getCompareSummary(versionId: string, otherVersionId: string): Promise<CompareSummary> {
  return fetchApi(`/api/versions/${versionId}/compare/${otherVersionId}`);
}

export async function getInspectionSnapshots(versionId: string): Promise<InspectionSnapshotResponse[]> {
  return fetchApi(`/api/versions/${versionId}/inspection-snapshots`);
}

export async function createInspectionSnapshot(
  versionId: string,
  params: InspectionSnapshotState,
): Promise<InspectionSnapshotResponse> {
  return fetchApi(`/api/versions/${versionId}/inspection-snapshots`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function getJob(jobId: string): Promise<JobResponse> {
  return fetchApi(`/api/jobs/${jobId}`);
}

export async function getVersionJobs(versionId: string): Promise<JobResponse[]> {
  return fetchApi(`/api/versions/${versionId}/jobs`);
}

export async function streamJobEvents(
  jobId: string,
  handlers: {
    onEvent: (event: JobEventResponse) => void;
    onStatus?: (status: JobResponse) => void;
    onError?: (error: Event) => void;
  },
): Promise<() => void> {
  const source = new EventSource(`${API_BASE}/api/jobs/${jobId}/events`);
  source.onmessage = (event) => {
    handlers.onEvent(JSON.parse(event.data) as JobEventResponse);
  };
  source.addEventListener('status', (event) => {
    handlers.onStatus?.(JSON.parse((event as MessageEvent).data) as JobResponse);
  });
  source.onerror = (event) => {
    handlers.onError?.(event);
    source.close();
  };
  return () => source.close();
}

export async function submitRepair(versionId: string): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/repair`, { method: 'POST' });
}

export async function submitResize(versionId: string, params: ResizeRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/resize`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitHollow(versionId: string, params: HollowRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/hollow`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitThicken(versionId: string, params: ThickenRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/thicken`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitCompare(versionId: string, params: CompareRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/compare`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitSmooth(versionId: string, params: SmoothRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/smooth`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitDecimate(versionId: string, params: DecimateRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/decimate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitSubdivide(versionId: string, params: SubdivideRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/subdivide`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitMakeDelone(versionId: string, params: MakeDeloneRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/make-delone`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitScoop(versionId: string, params: ScoopRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/scoop`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitMakeManufacturable(
  versionId: string,
  params: MakeManufacturableRequest,
): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/make-manufacturable`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitInteractiveCommit(
  versionId: string,
  params: InteractiveCommitRequest,
  meshFile: File,
): Promise<JobResponse> {
  const formData = new FormData();
  formData.append('request_json', JSON.stringify(params));
  formData.append('mesh_file', meshFile);

  let response: Response;
  try {
    response = await fetch(`${API_BASE}/api/versions/${versionId}/interactive-commit`, {
      method: 'POST',
      body: formData,
    });
  } catch (error) {
    throw new Error(
      error instanceof Error && error.message
        ? `Interactive commit failed: ${error.message}`
        : 'Interactive commit failed: backend unavailable or network request blocked',
    );
  }

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.detail || 'Interactive commit failed');
  }

  return response.json();
}
