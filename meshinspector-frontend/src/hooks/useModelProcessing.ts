'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  branchVersion,
  createInspectionSnapshot,
  getCompareCache,
  getCompareSummary,
  getInspectionSnapshots,
  getCompareOverlay,
  getJob,
  getVersionJobs,
  getManufacturability,
  getMeshLibWorkbenchManifest,
  getModelVersions,
  getSectionContour,
  getThicknessOverlay,
  getVersion,
  getViewerManifest,
  submitCompare,
  submitBrushReplay,
  submitCollisionDetect,
  submitDecimate,
  submitDistanceMapContourBoolean,
  submitDistanceMapContours,
  submitDistanceMapFromMesh,
  submitDistanceMapIsoLines,
  submitDistanceMapMerge,
  submitDistanceMapFromTiff,
  submitDistanceMapToTiff,
  submitExactBoolean,
  submitExpandShrink,
  submitGcodeLoadSource,
  submitGcodeParseFilePaths,
  submitGcodeParsePaths,
  submitGcodeWriteSource,
  submitHollow,
  submitInteractiveCommit,
  submitMakeDelone,
  submitMakeManufacturable,
  submitMeasureInspect,
  submitMeshCutMeasureTopology,
  submitMeshToVoxelsSdf,
  submitObjectLinesFromContours,
  submitOpenRawVoxels,
  submitOpenVoxelsFromTiff,
  submitObjectLinesLoadMrLines,
  submitObjectLinesLoadPly,
  submitObjectLinesLoadPts,
  submitObjectLinesLoadSvg,
  submitObjectLinesSaveDxf,
  submitObjectLinesSaveMrLines,
  submitObjectLinesSavePly,
  submitObjectLinesSavePts,
  submitObjectLinesToContours,
  submitOffsetContours,
  submitOffsetMesh,
  submitOffsetVerts,
  submitPartialOffset,
  submitPointCloudIcp,
  submitRepair,
  submitResize,
  submitSelectionCommit,
  submitShellMesh,
  submitShrinkExpand,
  submitScoop,
  submitSmooth,
  submitSubdivide,
  submitThicken,
  submitThickenMesh,
  submitVoxelActiveBox,
  submitVoxelBinaryOperations,
  submitVoxelBoolean,
  submitVoxelLineGraph,
  submitVoxelMaskToMesh,
  submitVoxelPath,
  submitVoxelPathBuildFour,
  submitVoxelSegmentation,
  submitVoxelSlice,
  submitVoxelToMeshDual,
  submitVoxelToMeshSimple,
  submitVoxelToMeshSmart,
  submitVoxelVolumeRenderData,
  submitVoxelVolumeRenderLut,
  submitVoxelVolumeRenderRay,
  submitWeightedShell,
  uploadModel,
} from '@/lib/api/models';
import type {
  BranchVersionRequest,
  BrushReplayRequest,
  CollisionDetectRequest,
  CollisionDetectResponse,
  CompareRequestV2,
  CompareSummary,
  CompareCacheEntry,
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
  MakeDeloneRequestV2,
  MakeManufacturableRequest,
  MeshLibWorkbenchManifest,
  MeasureInspectRequest,
  MeasureInspectResponse,
  MeshCutMeasureTopologyRequest,
  MeshCutMeasureTopologyResponse,
  MeshToVoxelsSdfRequest,
  MeshToVoxelsSdfResponse,
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
  ScoopRequestV2,
  ScalarOverlayResponse,
  SectionContourPayload,
  SelectionCommitRequest,
  SelectionCommitResponse,
  ShellMeshRequest,
  SmoothRequestV2,
  SubdivideRequestV2,
  ThickenMeshRequest,
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
  WeightedShellRequest,
} from '@/lib/api/types';

export function useUploadModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: uploadModel,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['models'] });
    },
  });
}

export function useVersion(versionId: string | null) {
  return useQuery({
    queryKey: ['version', versionId],
    queryFn: () => getVersion(versionId!),
    enabled: !!versionId,
  });
}

export function useManufacturability(versionId: string | null, enabled = true) {
  return useQuery({
    queryKey: ['manufacturability', versionId],
    queryFn: () => getManufacturability(versionId!),
    enabled: !!versionId && enabled,
    staleTime: 1000 * 30,
  });
}

export function useViewerManifest(versionId: string | null, enabled = true) {
  return useQuery({
    queryKey: ['viewer-manifest', versionId],
    queryFn: () => getViewerManifest(versionId!),
    enabled: !!versionId && enabled,
  });
}

export function useMeshLibWorkbenchManifest(versionId: string | null, enabled = true) {
  return useQuery<MeshLibWorkbenchManifest>({
    queryKey: ['meshlib-workbench', versionId],
    queryFn: () => getMeshLibWorkbenchManifest(versionId!),
    enabled: !!versionId && enabled,
    staleTime: 1000 * 60,
  });
}

export function useSectionContour(
  versionId: string | null,
  enabled: boolean,
  sectionConstant: number,
  planeAxis: [number, number, number],
  selectedRegionIds: string[],
) {
  return useQuery<SectionContourPayload>({
    queryKey: ['section-contour', versionId, sectionConstant, planeAxis, selectedRegionIds],
    queryFn: () =>
      getSectionContour(versionId!, {
        section_constant: sectionConstant,
        plane_axis: planeAxis,
        selected_region_ids: selectedRegionIds,
      }),
    enabled: !!versionId && enabled,
    staleTime: 1000 * 10,
  });
}

export function useMeasureInspectOperation() {
  return useMutation<
    MeasureInspectResponse,
    Error,
    { versionId: string; params: MeasureInspectRequest }
  >({
    mutationFn: ({ versionId, params }) => submitMeasureInspect(versionId, params),
  });
}

export function useMeshCutMeasureTopologyOperation() {
  const queryClient = useQueryClient();
  return useMutation<
    MeshCutMeasureTopologyResponse,
    Error,
    { versionId: string; params: MeshCutMeasureTopologyRequest }
  >({
    mutationFn: ({ versionId, params }) => submitMeshCutMeasureTopology(versionId, params),
    onSuccess: (response, variables) => {
      const childVersionId = response.version?.id;
      queryClient.invalidateQueries({ queryKey: ['version', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['version-jobs', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['model-versions'] });
      if (childVersionId) {
        queryClient.invalidateQueries({ queryKey: ['version', childVersionId] });
        queryClient.invalidateQueries({ queryKey: ['viewer-manifest', childVersionId] });
        queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', childVersionId] });
      }
    },
  });
}

export function useGcodeParsePathsOperation() {
  return useMutation<
    GcodeParsePathsResponse,
    Error,
    { versionId: string; params: GcodeParsePathsRequest }
  >({
    mutationFn: ({ versionId, params }) => submitGcodeParsePaths(versionId, params),
  });
}

export function useGcodeLoadSourceOperation() {
  return useMutation<
    GcodeSourceResponse,
    Error,
    { versionId: string; params: GcodeLoadSourceRequest }
  >({
    mutationFn: ({ versionId, params }) => submitGcodeLoadSource(versionId, params),
  });
}

export function useGcodeWriteSourceOperation() {
  return useMutation<
    GcodeSourceResponse,
    Error,
    { versionId: string; params: GcodeWriteSourceRequest }
  >({
    mutationFn: ({ versionId, params }) => submitGcodeWriteSource(versionId, params),
  });
}

export function useGcodeParseFilePathsOperation() {
  return useMutation<
    GcodeParsePathsResponse,
    Error,
    { versionId: string; params: GcodeParseFilePathsRequest }
  >({
    mutationFn: ({ versionId, params }) => submitGcodeParseFilePaths(versionId, params),
  });
}

export function usePointCloudIcpOperation() {
  return useMutation<
    PointCloudIcpResponse,
    Error,
    { versionId: string; params: PointCloudIcpRequest }
  >({
    mutationFn: ({ versionId, params }) => submitPointCloudIcp(versionId, params),
  });
}

export function useOffsetContoursOperation() {
  return useMutation<
    OffsetContoursResponse,
    Error,
    { versionId: string; params: OffsetContoursRequest }
  >({
    mutationFn: ({ versionId, params }) => submitOffsetContours(versionId, params),
  });
}

export function useDistanceMapContoursOperation() {
  return useMutation<
    DistanceMapResponse,
    Error,
    { versionId: string; params: DistanceMapContoursRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapContours(versionId, params),
  });
}

export function useDistanceMapFromMeshOperation() {
  return useMutation<
    DistanceMapResponse,
    Error,
    { versionId: string; params: DistanceMapFromMeshRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapFromMesh(versionId, params),
  });
}

export function useDistanceMapIsoLinesOperation() {
  return useMutation<
    IsoLineSegmentsResponse,
    Error,
    { versionId: string; params: DistanceMapIsoLinesRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapIsoLines(versionId, params),
  });
}

export function useDistanceMapMergeOperation() {
  return useMutation<
    DistanceMapResponse,
    Error,
    { versionId: string; params: DistanceMapMergeRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapMerge(versionId, params),
  });
}

export function useDistanceMapContourBooleanOperation() {
  return useMutation<
    IsoLineSegmentsResponse,
    Error,
    { versionId: string; params: DistanceMapContourBooleanRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapContourBoolean(versionId, params),
  });
}

export function useDistanceMapFromTiffOperation() {
  return useMutation<
    DistanceMapResponse,
    Error,
    { versionId: string; params: DistanceMapTiffImportRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapFromTiff(versionId, params),
  });
}

export function useDistanceMapToTiffOperation() {
  return useMutation<
    DistanceMapTiffExportResponse,
    Error,
    { versionId: string; params: DistanceMapTiffExportRequest }
  >({
    mutationFn: ({ versionId, params }) => submitDistanceMapToTiff(versionId, params),
  });
}

export function useObjectLinesFromContoursOperation() {
  return useMutation<
    ObjectLinesResponse,
    Error,
    { versionId: string; params: ObjectLinesFromContoursRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesFromContours(versionId, params),
  });
}

export function useObjectLinesLoadPtsOperation() {
  return useMutation<
    ObjectLinesResponse,
    Error,
    { versionId: string; params: ObjectLinesPtsLoadRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesLoadPts(versionId, params),
  });
}

export function useObjectLinesLoadMrLinesOperation() {
  return useMutation<
    ObjectLinesResponse,
    Error,
    { versionId: string; params: ObjectLinesBinaryLoadRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesLoadMrLines(versionId, params),
  });
}

export function useObjectLinesLoadPlyOperation() {
  return useMutation<
    ObjectLinesResponse,
    Error,
    { versionId: string; params: ObjectLinesBinaryLoadRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesLoadPly(versionId, params),
  });
}

export function useObjectLinesSaveMrLinesOperation() {
  return useMutation<
    ObjectLinesBinaryExportResponse,
    Error,
    { versionId: string; params: ObjectLinesBinaryExportRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesSaveMrLines(versionId, params),
  });
}

export function useObjectLinesSavePlyOperation() {
  return useMutation<
    ObjectLinesBinaryExportResponse,
    Error,
    { versionId: string; params: ObjectLinesBinaryExportRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesSavePly(versionId, params),
  });
}

export function useObjectLinesSavePtsOperation() {
  return useMutation<
    ObjectLinesTextExportResponse,
    Error,
    { versionId: string; params: ObjectLinesTextExportRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesSavePts(versionId, params),
  });
}

export function useObjectLinesLoadSvgOperation() {
  return useMutation<
    ObjectLinesResponse,
    Error,
    { versionId: string; params: ObjectLinesSvgLoadRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesLoadSvg(versionId, params),
  });
}

export function useObjectLinesSaveDxfOperation() {
  return useMutation<
    ObjectLinesTextExportResponse,
    Error,
    { versionId: string; params: ObjectLinesTextExportRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesSaveDxf(versionId, params),
  });
}

export function useObjectLinesToContoursOperation() {
  return useMutation<
    ObjectLinesToContoursResponse,
    Error,
    { versionId: string; params: ObjectLinesToContoursRequest }
  >({
    mutationFn: ({ versionId, params }) => submitObjectLinesToContours(versionId, params),
  });
}

export function useMeshToVoxelsSdfOperation() {
  return useMutation<
    MeshToVoxelsSdfResponse,
    Error,
    { versionId: string; params: MeshToVoxelsSdfRequest }
  >({
    mutationFn: ({ versionId, params }) => submitMeshToVoxelsSdf(versionId, params),
  });
}

export function useOpenRawVoxelsOperation() {
  return useMutation<
    VoxelVolumeLoadResponse,
    Error,
    { versionId: string; params: VoxelRawLoadRequest }
  >({
    mutationFn: ({ versionId, params }) => submitOpenRawVoxels(versionId, params),
  });
}

export function useOpenVoxelsFromTiffOperation() {
  return useMutation<
    VoxelVolumeLoadResponse,
    Error,
    { versionId: string; params: VoxelTiffLoadRequest }
  >({
    mutationFn: ({ versionId, params }) => submitOpenVoxelsFromTiff(versionId, params),
  });
}

export function useVoxelBinaryOperationsOperation() {
  return useMutation<
    VoxelBinaryOperationsResponse,
    Error,
    { versionId: string; params: VoxelBinaryOperationsRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelBinaryOperations(versionId, params),
  });
}

export function useVoxelLineGraphOperation() {
  return useMutation<
    VoxelLineGraphResponse,
    Error,
    { versionId: string; params: VoxelLineGraphRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelLineGraph(versionId, params),
  });
}

export function useVoxelActiveBoxOperation() {
  return useMutation<
    VoxelActiveBoxResponse,
    Error,
    { versionId: string; params: VoxelActiveBoxRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelActiveBox(versionId, params),
  });
}

export function useVoxelSliceOperation() {
  return useMutation<
    VoxelSliceResponse,
    Error,
    { versionId: string; params: VoxelSliceRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelSlice(versionId, params),
  });
}

export function useVoxelPathOperation() {
  return useMutation<
    VoxelPathResponse,
    Error,
    { versionId: string; params: VoxelPathRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelPath(versionId, params),
  });
}

export function useVoxelPathBuildFourOperation() {
  return useMutation<
    VoxelPathBuildFourResponse,
    Error,
    { versionId: string; params: VoxelPathBuildFourRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelPathBuildFour(versionId, params),
  });
}

export function useVoxelSegmentationOperation() {
  return useMutation<
    VoxelSegmentationResponse,
    Error,
    { versionId: string; params: VoxelSegmentationRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelSegmentation(versionId, params),
  });
}

export function useVoxelMaskToMeshOperation() {
  return useMutation<
    VoxelMaskToMeshResponse,
    Error,
    { versionId: string; params: VoxelMaskToMeshRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelMaskToMesh(versionId, params),
  });
}

export function useVoxelToMeshSimpleOperation() {
  return useMutation<
    VoxelToMeshSimpleResponse,
    Error,
    { versionId: string; params: VoxelToMeshSimpleRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelToMeshSimple(versionId, params),
  });
}

export function useVoxelToMeshDualOperation() {
  return useMutation<
    VoxelToMeshDualResponse,
    Error,
    { versionId: string; params: VoxelToMeshDualRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelToMeshDual(versionId, params),
  });
}

export function useVoxelToMeshSmartOperation() {
  return useMutation<
    VoxelToMeshSmartResponse,
    Error,
    { versionId: string; params: VoxelToMeshSmartRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelToMeshSmart(versionId, params),
  });
}

export function useVoxelVolumeRenderDataOperation() {
  return useMutation<
    VoxelVolumeRenderDataResponse,
    Error,
    { versionId: string; params: VoxelVolumeRenderDataRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelVolumeRenderData(versionId, params),
  });
}

export function useVoxelVolumeRenderLutOperation() {
  return useMutation<
    VoxelVolumeRenderLutResponse,
    Error,
    { versionId: string; params: VoxelVolumeRenderLutRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelVolumeRenderLut(versionId, params),
  });
}

export function useVoxelVolumeRenderRayOperation() {
  return useMutation<
    VoxelVolumeRenderRayResponse,
    Error,
    { versionId: string; params: VoxelVolumeRenderRayRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelVolumeRenderRay(versionId, params),
  });
}

export function useOffsetMeshOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: OffsetMeshRequest }
  >({
    mutationFn: ({ versionId, params }) => submitOffsetMesh(versionId, params),
  });
}

export function useShellMeshOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: ShellMeshRequest }
  >({
    mutationFn: ({ versionId, params }) => submitShellMesh(versionId, params),
  });
}

export function useThickenMeshOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: ThickenMeshRequest }
  >({
    mutationFn: ({ versionId, params }) => submitThickenMesh(versionId, params),
  });
}

export function useWeightedShellOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: WeightedShellRequest }
  >({
    mutationFn: ({ versionId, params }) => submitWeightedShell(versionId, params),
  });
}

export function usePartialOffsetOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: PartialOffsetRequest }
  >({
    mutationFn: ({ versionId, params }) => submitPartialOffset(versionId, params),
  });
}

export function useOffsetVertsOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: OffsetVertsRequest }
  >({
    mutationFn: ({ versionId, params }) => submitOffsetVerts(versionId, params),
  });
}

export function useExpandShrinkOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: OffsetSmoothingRequest }
  >({
    mutationFn: ({ versionId, params }) => submitExpandShrink(versionId, params),
  });
}

export function useShrinkExpandOperation() {
  return useMutation<
    OffsetShellMeshResponse,
    Error,
    { versionId: string; params: OffsetSmoothingRequest }
  >({
    mutationFn: ({ versionId, params }) => submitShrinkExpand(versionId, params),
  });
}

export function useExactBooleanOperation() {
  return useMutation<
    ExactBooleanResponse,
    Error,
    { versionId: string; params: ExactBooleanRequest }
  >({
    mutationFn: ({ versionId, params }) => submitExactBoolean(versionId, params),
  });
}

export function useVoxelBooleanOperation() {
  return useMutation<
    VoxelBooleanResponse,
    Error,
    { versionId: string; params: VoxelBooleanRequest }
  >({
    mutationFn: ({ versionId, params }) => submitVoxelBoolean(versionId, params),
  });
}

export function useCollisionDetectOperation() {
  return useMutation<
    CollisionDetectResponse,
    Error,
    { versionId: string; params: CollisionDetectRequest }
  >({
    mutationFn: ({ versionId, params }) => submitCollisionDetect(versionId, params),
  });
}

export function useSelectionCommitOperation() {
  return useMutation<
    SelectionCommitResponse,
    Error,
    { versionId: string; params: SelectionCommitRequest }
  >({
    mutationFn: ({ versionId, params }) => submitSelectionCommit(versionId, params),
  });
}

export function useBrushReplayOperation() {
  const queryClient = useQueryClient();
  return useMutation<JobResponse, Error, { versionId: string; params: BrushReplayRequest }>({
    mutationFn: ({ versionId, params }) => submitBrushReplay(versionId, params),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['version', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['viewer-manifest', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['version-jobs', variables.versionId] });
    },
  });
}

export function useModelVersions(modelId: string | null) {
  return useQuery({
    queryKey: ['model-versions', modelId],
    queryFn: () => getModelVersions(modelId!),
    enabled: !!modelId,
  });
}

export function useThicknessOverlay(versionId: string | null, enabled: boolean) {
  return useQuery<ScalarOverlayResponse>({
    queryKey: ['thickness-overlay', versionId],
    queryFn: () => getThicknessOverlay(versionId!),
    enabled: !!versionId && enabled,
  });
}

export function useCompareOverlay(versionId: string | null, otherVersionId: string | null, enabled: boolean) {
  return useQuery<ScalarOverlayResponse>({
    queryKey: ['compare-overlay', versionId, otherVersionId],
    queryFn: () => getCompareOverlay(versionId!, otherVersionId!),
    enabled: !!versionId && !!otherVersionId && enabled,
  });
}

export function useCompareCache(versionId: string | null) {
  return useQuery<CompareCacheEntry[]>({
    queryKey: ['compare-cache', versionId],
    queryFn: () => getCompareCache(versionId!),
    enabled: !!versionId,
  });
}

export function useCompareSummary(versionId: string | null, otherVersionId: string | null, enabled: boolean) {
  return useQuery<CompareSummary>({
    queryKey: ['compare-summary', versionId, otherVersionId],
    queryFn: () => getCompareSummary(versionId!, otherVersionId!),
    enabled: !!versionId && !!otherVersionId && enabled,
  });
}

export function useInspectionSnapshots(versionId: string | null) {
  return useQuery<InspectionSnapshotResponse[]>({
    queryKey: ['inspection-snapshots', versionId],
    queryFn: () => getInspectionSnapshots(versionId!),
    enabled: !!versionId,
  });
}

export function useJob(jobId: string | null) {
  return useQuery({
    queryKey: ['job', jobId],
    queryFn: () => getJob(jobId!),
    enabled: !!jobId,
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      return status && ['succeeded', 'failed'].includes(status) ? false : 2000;
    },
  });
}

export function useVersionJobs(versionId: string | null) {
  return useQuery<JobResponse[]>({
    queryKey: ['version-jobs', versionId],
    queryFn: () => getVersionJobs(versionId!),
    enabled: !!versionId,
    refetchInterval: 5000,
  });
}

function createOperationMutation<TArgs>(submitter: (versionId: string, params: TArgs) => Promise<unknown>) {
  return function useOperation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ versionId, params }: { versionId: string; params: TArgs }) => submitter(versionId, params),
      onSuccess: (_data, variables) => {
        queryClient.invalidateQueries({ queryKey: ['version', variables.versionId] });
        queryClient.invalidateQueries({ queryKey: ['version-jobs', variables.versionId] });
      },
    });
  };
}

export const useResizeOperation = createOperationMutation<ResizeRequestV2>(submitResize);
export const useHollowOperation = createOperationMutation<HollowRequestV2>(submitHollow);
export const useThickenOperation = createOperationMutation<ThickenRequestV2>(submitThicken);
export const useCompareOperation = createOperationMutation<CompareRequestV2>(submitCompare);
export const useMakeManufacturableOperation = createOperationMutation<MakeManufacturableRequest>(submitMakeManufacturable);
export const useScoopOperation = createOperationMutation<ScoopRequestV2>(submitScoop);
export const useSmoothOperation = createOperationMutation<SmoothRequestV2>(submitSmooth);
export const useDecimateOperation = createOperationMutation<DecimateRequestV2>(submitDecimate);
export const useSubdivideOperation = createOperationMutation<SubdivideRequestV2>(submitSubdivide);
export const useMakeDeloneOperation = createOperationMutation<MakeDeloneRequestV2>(submitMakeDelone);

export function useCreateInspectionSnapshot() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, params }: { versionId: string; params: InspectionSnapshotState }) =>
      createInspectionSnapshot(versionId, params),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['inspection-snapshots', variables.versionId] });
    },
  });
}

export function useBranchVersion() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, params }: { versionId: string; params: BranchVersionRequest }) =>
      branchVersion(versionId, params),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['model-versions', data.model_id] });
      queryClient.invalidateQueries({ queryKey: ['version', data.id] });
    },
  });
}

export function useRepairOperation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (versionId: string) => submitRepair(versionId),
    onSuccess: (_data, versionId) => {
      queryClient.invalidateQueries({ queryKey: ['version', versionId] });
      queryClient.invalidateQueries({ queryKey: ['version-jobs', versionId] });
    },
  });
}

export function useInteractiveCommitOperation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, params, meshFile }: { versionId: string; params: InteractiveCommitRequest; meshFile: File }) =>
      submitInteractiveCommit(versionId, params, meshFile),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['version', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['viewer-manifest', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', variables.versionId] });
      queryClient.invalidateQueries({ queryKey: ['version-jobs', variables.versionId] });
    },
  });
}
