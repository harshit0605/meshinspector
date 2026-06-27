'use client';

import { Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import dynamic from 'next/dynamic';
import { useQueryClient } from '@tanstack/react-query';
import JobActivityPanel from '@/features/editor/panels/JobActivityPanel';
import { useEditorStore } from '@/features/editor/store';
import { useJobEventStream } from '@/features/editor/hooks/useJobEventStream';
import { useJobPolling } from '@/features/editor/hooks/useJobPolling';
import CommandBar from '@/features/editor/workspace/CommandBar';
import ModelInspector from '@/features/editor/workspace/ModelInspector';
import ReviewInspector from '@/features/editor/workspace/ReviewInspector';
import StatusStrip from '@/features/editor/workspace/StatusStrip';
import ToolInspector from '@/features/editor/workspace/ToolInspector';
import { WORKSPACE_COMMANDS } from '@/features/editor/workspace/toolRegistry';
import type { RightDockTab, ToolbarGroup, WorkspaceCommandId } from '@/features/editor/workspace/types';
import type { WorkbenchHostCommandPayload } from '@/features/editor/viewer/MeshLibWorkbenchHost';
import {
  useCreateInspectionSnapshot,
  useBranchVersion,
  useBrushReplayOperation,
  useCollisionDetectOperation,
  useCompareCache,
  useCompareOperation,
  useCompareOverlay,
  useCompareSummary,
  useDecimateOperation,
  useDistanceMapContourBooleanOperation,
  useDistanceMapContoursOperation,
  useDistanceMapFromMeshOperation,
  useDistanceMapIsoLinesOperation,
  useDistanceMapMergeOperation,
  useDistanceMapFromTiffOperation,
  useDistanceMapToTiffOperation,
  useExactBooleanOperation,
  useExpandShrinkOperation,
  useGcodeParsePathsOperation,
  useGcodeLoadSourceOperation,
  useGcodeParseFilePathsOperation,
  useGcodeWriteSourceOperation,
  useHollowOperation,
  useInspectionSnapshots,
  useMakeDeloneOperation,
  useMeshLibWorkbenchManifest,
  useMakeManufacturableOperation,
  useManufacturability,
  useMeasureInspectOperation,
  useMeshCutMeasureTopologyOperation,
  useMeshToVoxelsSdfOperation,
  useModelVersions,
  useObjectLinesFromContoursOperation,
  useOpenRawVoxelsOperation,
  useOpenVoxelsFromTiffOperation,
  useObjectLinesLoadMrLinesOperation,
  useObjectLinesLoadPlyOperation,
  useObjectLinesLoadPtsOperation,
  useObjectLinesLoadSvgOperation,
  useObjectLinesSaveDxfOperation,
  useObjectLinesSaveMrLinesOperation,
  useObjectLinesSavePlyOperation,
  useObjectLinesSavePtsOperation,
  useObjectLinesToContoursOperation,
  useOffsetContoursOperation,
  useOffsetMeshOperation,
  useOffsetVertsOperation,
  usePartialOffsetOperation,
  usePointCloudIcpOperation,
  useRepairOperation,
  useResizeOperation,
  useSectionContour,
  useShellMeshOperation,
  useShrinkExpandOperation,
  useScoopOperation,
  useSmoothOperation,
  useSubdivideOperation,
  useThicknessOverlay,
  useThickenMeshOperation,
  useThickenOperation,
  useVoxelBooleanOperation,
  useVoxelActiveBoxOperation,
  useVoxelBinaryOperationsOperation,
  useVoxelLineGraphOperation,
  useVoxelMaskToMeshOperation,
  useVoxelPathBuildFourOperation,
  useVoxelPathOperation,
  useVoxelSegmentationOperation,
  useVoxelSliceOperation,
  useVoxelToMeshDualOperation,
  useVoxelToMeshSimpleOperation,
  useVoxelToMeshSmartOperation,
  useVoxelVolumeRenderDataOperation,
  useVoxelVolumeRenderLutOperation,
  useVoxelVolumeRenderRayOperation,
  useVersionJobs,
  useVersion,
  useViewerManifest,
  useWeightedShellOperation,
} from '@/hooks/useModelProcessing';
import type {
  CollisionDetectRequest,
  CollisionDetectResponse,
  BrushReplayRequest,
  BrushReplayStroke,
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
  HollowRequestV2,
  GcodeParsePathsRequest,
  GcodeParsePathsResponse,
  GcodeLoadSourceRequest,
  GcodeParseFilePathsRequest,
  GcodeWriteSourceRequest,
  InspectionSnapshotResponse,
  InspectionSnapshotState,
  InteractiveSelectionPayload,
  IsoLineSegmentsResponse,
  JobResponse,
  MakeDeloneRequestV2,
  MakeManufacturableRequest,
  ManufacturabilitySnapshot,
  MaterialType,
  MeasureInspectPair,
  MeasureInspectRequest,
  MeasureInspectResponse,
  MeshCutMeasureTopologyRequest,
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
  RegionManifestEntry,
  ResizeRequestV2,
  ScoopRequestV2,
  SectionContourPayload,
  ShellMeshRequest,
  SmoothRequestV2,
  SubdivideRequestV2,
  ThickenMeshRequest,
  ThickenRequestV2,
  TextureArtifactManifest,
  VoxelBooleanRequest,
  VoxelBooleanResponse,
  VoxelActiveBoxRequest,
  VoxelBinaryOperationsRequest,
  VoxelLineGraphRequest,
  VoxelMaskToMeshRequest,
  VoxelPathBuildFourRequest,
  VoxelPathRequest,
  VoxelRawLoadRequest,
  VoxelSegmentationRequest,
  VoxelSliceRequest,
  VoxelTiffLoadRequest,
  VoxelToMeshDualRequest,
  VoxelToMeshSimpleRequest,
  VoxelToMeshSmartRequest,
  VoxelVolumeLoadResponse,
  VoxelVolumeRenderDataRequest,
  VoxelVolumeRenderLutRequest,
  VoxelVolumeRenderRayRequest,
  VoxelVolumeRenderRayResponse,
  WeightedShellRequest,
} from '@/lib/api/types';
import { getArtifactUrl } from '@/lib/api/client';
import { getSectionContour } from '@/lib/api/models';

const ViewerEngine = dynamic(() => import('@/features/editor/viewer/ViewerEngine'), { ssr: false });
const MeshLibWorkbenchHost = dynamic(() => import('@/features/editor/viewer/MeshLibWorkbenchHost'), { ssr: false });

type CommandAvailability = {
  disabled: boolean;
  reason?: string;
};

type WorkbenchCommandInvocation = {
  payload: Record<string, unknown>;
  options: Record<string, unknown>;
  endpointUrl?: string | null;
  endpointUrlKey?: string | null;
  rustBacked?: boolean;
  sdkOperations?: string[];
};

const MATERIAL_TYPES = new Set<MaterialType>([
  'gold_24k',
  'gold_22k',
  'gold_18k',
  'gold_14k',
  'gold_10k',
  'silver_925',
  'platinum',
]);

const PROTECT_REGION_TYPES = new Set<HollowRequestV2['protect_regions'][number]>([
  'head',
  'gem_seat',
  'ornament_relief',
  'inner_band',
]);

function recordFromUnknown(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }
  return value as Record<string, unknown>;
}

function requestPayloadFromWorkbenchCommand(invocation?: WorkbenchCommandInvocation): Record<string, unknown> {
  const payload = invocation?.payload ?? {};
  const request = recordFromUnknown(payload.request);
  if (Object.keys(request).length > 0) {
    return request;
  }
  const params = recordFromUnknown(payload.params);
  if (Object.keys(params).length > 0) {
    return params;
  }
  return payload;
}

function statePayloadFromWorkbenchCommand(invocation?: WorkbenchCommandInvocation): Record<string, unknown> {
  const requestPayload = requestPayloadFromWorkbenchCommand(invocation);
  return {
    ...(invocation?.options ?? {}),
    ...requestPayload,
  };
}

function numberFromPayload(payload: Record<string, unknown>, keys: string[], fallback: number): number {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value;
    }
    if (typeof value === 'string' && value.trim() !== '') {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) {
        return parsed;
      }
    }
  }
  return fallback;
}

function hasAnyPayloadKey(payload: Record<string, unknown>, keys: string[]): boolean {
  return keys.some((key) => Object.prototype.hasOwnProperty.call(payload, key));
}

function optionalNumberFromPayload(payload: Record<string, unknown>, keys: string[]): number | null {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value;
    }
    if (typeof value === 'string' && value.trim() !== '') {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) {
        return parsed;
      }
    }
  }
  return null;
}

function explicitBooleanFromPayload(payload: Record<string, unknown>, keys: string[]): boolean | null {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'boolean') {
      return value;
    }
    if (typeof value === 'string') {
      if (['1', 'true', 'yes', 'on'].includes(value.toLowerCase())) {
        return true;
      }
      if (['0', 'false', 'no', 'off'].includes(value.toLowerCase())) {
        return false;
      }
    }
  }
  return null;
}

function booleanFromPayload(payload: Record<string, unknown>, keys: string[], fallback: boolean): boolean {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'boolean') {
      return value;
    }
    if (typeof value === 'string') {
      if (['1', 'true', 'yes', 'on'].includes(value.toLowerCase())) {
        return true;
      }
      if (['0', 'false', 'no', 'off'].includes(value.toLowerCase())) {
        return false;
      }
    }
  }
  return fallback;
}

function integerListFromPayload(payload: Record<string, unknown>, keys: string[], fallback: number[] = []): number[] {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'string' && value.trim() !== '') {
      return parseIntegerList(value);
    }
    if (Array.isArray(value)) {
      return value
        .map((item) => Number(item))
        .filter((item) => Number.isInteger(item) && item >= 0);
    }
  }
  return fallback;
}

function numberListFromPayload(payload: Record<string, unknown>, keys: string[], fallback: number[] = []): number[] {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'string' && value.trim() !== '') {
      const parsed = value
        .split(',')
        .map((item) => Number(item.trim()))
        .filter(Number.isFinite);
      if (parsed.length > 0) {
        return parsed;
      }
    }
    if (Array.isArray(value)) {
      const parsed = value.map((item) => Number(item)).filter(Number.isFinite);
      if (parsed.length > 0) {
        return parsed;
      }
    }
  }
  return fallback;
}

function vector3FromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  componentKeys: [string[], string[], string[]],
  fallback: [number, number, number],
): [number, number, number] {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value) || value.length < 3) {
      continue;
    }
    const vector = [Number(value[0]), Number(value[1]), Number(value[2])] as [number, number, number];
    if (vector.every(Number.isFinite)) {
      return vector;
    }
  }
  return [
    numberFromPayload(payload, componentKeys[0], fallback[0]),
    numberFromPayload(payload, componentKeys[1], fallback[1]),
    numberFromPayload(payload, componentKeys[2], fallback[2]),
  ];
}

function stringFromPayload(payload: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'string' && value.trim() !== '') {
      return value;
    }
  }
  return null;
}

function stringListFromPayload(payload: Record<string, unknown>, keys: string[]): string[] {
  for (const key of keys) {
    const value = payload[key];
    if (Array.isArray(value)) {
      return value.filter((item): item is string => typeof item === 'string' && item.trim() !== '');
    }
    if (typeof value === 'string' && value.trim() !== '') {
      return value.split(',').map((item) => item.trim()).filter(Boolean);
    }
  }
  return [];
}

function vectorListFromPayload(payload: Record<string, unknown>, keys: string[]): Array<[number, number, number]> {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value)) {
      continue;
    }
    return value
      .map((item): [number, number, number] | null => {
        if (!Array.isArray(item) || item.length < 3) return null;
        const vector = [Number(item[0]), Number(item[1]), Number(item[2])] as [number, number, number];
        return vector.every(Number.isFinite) ? vector : null;
      })
      .filter((item): item is [number, number, number] => item != null);
  }
  return [];
}

function runtimeBrushToolId(commandId: WorkspaceCommandId): BrushReplayStroke['tool_id'] | null {
  if (commandId === 'runtime-thicken-brush') return 'thicken_brush';
  if (commandId === 'runtime-scoop-brush') return 'scoop_brush';
  if (commandId === 'runtime-smooth-brush') return 'smooth_brush';
  return null;
}

function selectionFromWorkbenchBrushPayload(value: unknown): InteractiveSelectionPayload {
  const selection = recordFromUnknown(value);
  const mode = stringFromPayload(selection, ['mode']);
  return {
    mode: mode === 'brush' || mode === 'lasso' || mode === 'rect' || mode === 'pick' || mode === 'vertices' || mode === 'regions'
      ? mode
      : 'faces',
    vertex_ids: integerListFromPayload(selection, ['vertex_ids', 'vertexIds', 'vertices']),
    face_ids: integerListFromPayload(selection, ['face_ids', 'faceIds', 'faces'], [0]),
    region_ids: stringListFromPayload(selection, ['region_ids', 'regionIds', 'regions']),
    brush_points_world: vectorListFromPayload(selection, ['brush_points_world', 'brushPointsWorld', 'brush_points']),
    metadata: recordFromUnknown(selection.metadata),
  };
}

function brushReplayRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  commandId: WorkspaceCommandId,
): BrushReplayRequest | null {
  const fallbackToolId = runtimeBrushToolId(commandId);
  if (!fallbackToolId) {
    return null;
  }
  const rawStrokes = Array.isArray(payload.strokes) ? payload.strokes : [];
  const strokes = (rawStrokes.length > 0 ? rawStrokes : [payload])
    .map((value): BrushReplayStroke | null => {
      const stroke = recordFromUnknown(value);
      const toolId = stringFromPayload(stroke, ['tool_id', 'toolId', 'tool']) ?? fallbackToolId;
      if (toolId !== 'thicken_brush' && toolId !== 'scoop_brush' && toolId !== 'smooth_brush') {
        return null;
      }
      return {
        tool_id: toolId,
        selection: selectionFromWorkbenchBrushPayload(stroke.selection),
        amount_mm: numberFromPayload(stroke, ['amount_mm', 'amountMm', 'depth_mm', 'depthMm'], toolId === 'smooth_brush' ? 0 : 0.04),
        falloff_mm: numberFromPayload(stroke, ['falloff_mm', 'falloffMm', 'brush_radius_mm', 'brushRadiusMm'], 1.5),
        iterations: Math.max(1, Math.round(numberFromPayload(stroke, ['iterations'], 1))),
        strength: Math.max(0, Math.min(1, numberFromPayload(stroke, ['strength'], 0.35))),
        metadata: recordFromUnknown(stroke.metadata),
      };
    })
    .filter((stroke): stroke is BrushReplayStroke => stroke != null);
  if (strokes.length === 0) {
    return null;
  }
  return {
    operation_label: stringFromPayload(payload, ['operation_label', 'operationLabel', 'label']) ?? (
      fallbackToolId === 'thicken_brush' ? 'Thicken Brush' : fallbackToolId === 'scoop_brush' ? 'Scoop Brush' : 'Smooth Brush'
    ),
    strokes,
    metadata: recordFromUnknown(payload.metadata),
  };
}

function edgePairsFromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: [number, number][] = [],
): [number, number][] {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'string' && value.trim() !== '') {
      return parseEdgePairString(value);
    }
    if (Array.isArray(value)) {
      const pairs = value
        .map((item): [number, number] | null => {
          if (!Array.isArray(item) || item.length < 2) return null;
          const first = Number(item[0]);
          const second = Number(item[1]);
          return Number.isInteger(first) && Number.isInteger(second) && first >= 0 && second >= 0 && first !== second
            ? [first, second]
            : null;
        })
        .filter((item): item is [number, number] => item != null);
      return pairs;
    }
  }
  return fallback;
}

function parseEdgePairString(value: string): [number, number][] {
  return value
    .split(/[,\n;]+/)
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => item.split(/[-:\s]+/).map((part) => Number(part.trim())))
    .filter((parts): parts is [number, number] =>
      parts.length === 2 &&
      parts.every((part) => Number.isInteger(part) && part >= 0) &&
      parts[0] !== parts[1],
    );
}

function parseIntegerList(value: string): number[] {
  return value
    .split(/[,\n;\s]+/)
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => Number(item))
    .filter((item) => Number.isInteger(item) && item >= 0);
}

function formatIntegerList(values: number[]): string {
  return values.join(', ');
}

function formatEdgePairs(pairs: [number, number][]): string {
  return pairs.map((pair) => `${pair[0]}-${pair[1]}`).join(', ');
}

function materialFromPayload(payload: Record<string, unknown>, fallback: MaterialType): MaterialType {
  const material = stringFromPayload(payload, ['material', 'material_type']);
  return material && MATERIAL_TYPES.has(material as MaterialType) ? material as MaterialType : fallback;
}

function protectRegionsFromPayload(payload: Record<string, unknown>): HollowRequestV2['protect_regions'] {
  const regions = stringListFromPayload(payload, ['protect_regions', 'protected_regions']);
  const filtered = regions.filter((region): region is HollowRequestV2['protect_regions'][number] =>
    PROTECT_REGION_TYPES.has(region as HollowRequestV2['protect_regions'][number]),
  );
  return filtered.length > 0 ? filtered : ['head', 'gem_seat', 'ornament_relief'];
}

function hollowProcessingModeFromPayload(
  payload: Record<string, unknown>,
  fallback: HollowRequestV2['processing_mode'] = 'interactive',
): HollowRequestV2['processing_mode'] {
  const value = stringFromPayload(payload, ['processing_mode', 'processingMode', 'hollow_processing_mode']);
  if (value === 'full_resolution' || value === 'full' || value === 'offline' || value === 'offline_full') {
    return 'full_resolution';
  }
  if (value === 'interactive' || value === 'preview') {
    return 'interactive';
  }
  return booleanFromPayload(
    payload,
    ['full_resolution', 'fullResolution', 'run_full_resolution', 'runFullResolution'],
    fallback === 'full_resolution',
  )
    ? 'full_resolution'
    : 'interactive';
}

function vectorFromPayload(payload: Record<string, unknown>, key: string): [number, number, number] | undefined {
  const value = payload[key];
  if (!Array.isArray(value) || value.length < 3) {
    return undefined;
  }
  const vector = value.slice(0, 3).map((coordinate) => Number(coordinate));
  if (!vector.every(Number.isFinite)) {
    return undefined;
  }
  return [vector[0], vector[1], vector[2]];
}

function vectorFromPayloadKeys(payload: Record<string, unknown>, keys: string[]): [number, number, number] | undefined {
  for (const key of keys) {
    const vector = vectorFromPayload(payload, key);
    if (vector) {
      return vector;
    }
  }
  return undefined;
}

function sectionPlaneConstantFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: number,
  currentAxis: [number, number, number],
): number {
  const direct = numberFromPayload(payload, ['section_constant', 'plane', 'plane_offset', 'offset_mm', 'section_offset_mm'], Number.NaN);
  if (Number.isFinite(direct)) {
    return direct;
  }
  const planeOrigin = vectorFromPayloadKeys(payload, ['plane_origin', 'origin', 'point', 'center']);
  if (!planeOrigin) {
    return fallback;
  }
  const axis = normalizeAxis(vectorFromPayloadKeys(payload, ['plane_axis', 'section_axis', 'axis', 'manual_axis']) ?? currentAxis);
  return dot(planeOrigin, axis);
}

function sectionContourParamsFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackConstant: number,
  fallbackAxis: [number, number, number],
  fallbackRegionIds: string[],
): {
  section_constant: number;
  plane_axis: [number, number, number];
  selected_region_ids: string[];
} {
  const selectedRegionIdsFromWorkbench = stringListFromPayload(
    payload,
    ['selected_region_ids', 'region_ids', 'regions_selected', 'regions'],
  );
  return {
    section_constant: sectionPlaneConstantFromWorkbenchPayload(payload, fallbackConstant, fallbackAxis),
    plane_axis: normalizeAxis(vectorFromPayloadKeys(payload, ['plane_axis', 'section_axis', 'axis', 'manual_axis']) ?? fallbackAxis),
    selected_region_ids: selectedRegionIdsFromWorkbench.length > 0 ? selectedRegionIdsFromWorkbench : fallbackRegionIds,
  };
}

function measureInspectPointFromUnknown(value: unknown): [number, number, number] | null {
  if (!Array.isArray(value) || value.length < 3 || !value.slice(0, 3).every((coordinate) => Number.isFinite(Number(coordinate)))) {
    return null;
  }
  return [Number(value[0]), Number(value[1]), Number(value[2])];
}

function measureInspectMetricFromUnknown(value: unknown): MeasureInspectPair['metric'] | null {
  if (typeof value !== 'string') {
    return null;
  }
  const normalized = value.trim().toLowerCase();
  if (['geodesic', 'surface', 'surface_distance', 'surface-distance'].includes(normalized)) {
    return 'geodesic';
  }
  if (['euclidean', 'linear', 'straight', 'straight_line', 'straight-line'].includes(normalized)) {
    return 'euclidean';
  }
  return null;
}

function nonnegativeIntegerFromUnknown(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : null;
}

function positiveNumberFromUnknown(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function nonnegativeNumberFromUnknown(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

function appendMeasureInspectPoint(points: Array<[number, number, number]>, value: unknown) {
  const directPoint = measureInspectPointFromUnknown(value);
  if (directPoint) {
    points.push(directPoint);
    return;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      appendMeasureInspectPoint(points, item);
    }
  }
}

function appendMeasureInspectPair(
  pointPairs: MeasureInspectRequest['point_pairs'],
  value: unknown,
) {
  if (!value) {
    return;
  }
  if (Array.isArray(value)) {
    if (value.length >= 2) {
      const start = measureInspectPointFromUnknown(value[0]);
      const end = measureInspectPointFromUnknown(value[1]);
      if (start && end) {
        pointPairs.push({ start, end });
        return;
      }
    }
    for (const item of value) {
      appendMeasureInspectPair(pointPairs, item);
    }
    return;
  }
  if (typeof value !== 'object') {
    return;
  }
  const record = value as Record<string, unknown>;
  const start = measureInspectPointFromUnknown(record.start ?? record.from ?? record.p0 ?? record.a);
  const end = measureInspectPointFromUnknown(record.end ?? record.to ?? record.p1 ?? record.b);
  if (start && end) {
    const metric = measureInspectMetricFromUnknown(record.metric ?? record.distance_metric ?? record.distanceMetric);
    const startVertex = nonnegativeIntegerFromUnknown(record.start_vertex ?? record.startVertex ?? record.from_vertex ?? record.fromVertex);
    const endVertex = nonnegativeIntegerFromUnknown(record.end_vertex ?? record.endVertex ?? record.to_vertex ?? record.toVertex);
    const controlVertices = integerListFromPayload(record, [
      'control_vertices',
      'controlVertices',
      'control_vertex_indices',
      'controlVertexIndices',
      'path_vertices',
      'pathVertices',
      'polyline_vertices',
      'polylineVertices',
    ]);
    const closePath = booleanFromPayload(record, ['close_path', 'closePath', 'closed_path', 'closedPath', 'closed', 'is_closed', 'isClosed'], false);
    const includeRefinedSurfacePath = booleanFromPayload(
      record,
      ['include_refined_surface_path', 'includeRefinedSurfacePath', 'refine_surface_path', 'refineSurfacePath'],
      false,
    );
    const maxPathLen = positiveNumberFromUnknown(
      record.geodesic_max_path_len_mm ?? record.max_path_len_mm ?? record.maxPathLenMm ?? record.max_path_length_mm ?? record.maxPathLengthMm,
    );
    pointPairs.push({
      start,
      end,
      label: typeof record.label === 'string' ? record.label : null,
      ...(metric ? { metric } : {}),
      ...(startVertex != null ? { start_vertex: startVertex } : {}),
      ...(endVertex != null ? { end_vertex: endVertex } : {}),
      ...(controlVertices.length ? { control_vertices: controlVertices } : {}),
      ...(closePath ? { close_path: true } : {}),
      ...(includeRefinedSurfacePath ? { include_refined_surface_path: true } : {}),
      ...(maxPathLen != null ? { geodesic_max_path_len_mm: maxPathLen } : {}),
    });
  }
}

function measureInspectSurfaceDistanceFromWorkbenchPayload(
  payload: Record<string, unknown>,
): MeasureInspectRequest['surface_distance'] {
  const nested = recordFromUnknown(payload.surface_distance ?? payload.surfaceDistance);
  const record = Object.keys(nested).length > 0 ? nested : payload;
  const seed = measureInspectPointFromUnknown(record.seed ?? record.seed_point ?? record.seedPoint ?? record.point ?? record.point_world ?? record.world_point);
  const seedVertex = nonnegativeIntegerFromUnknown(record.seed_vertex ?? record.seedVertex);
  const seedVertices = integerListFromPayload(record, ['seed_vertices', 'seedVertices', 'source_vertices', 'sourceVertices']);
  const seedEdges = edgePairsFromPayload(record, ['seed_edges', 'seedEdges', 'source_edges', 'sourceEdges', 'selected_edges', 'selectedEdges']);
  const seedFaceIds = integerListFromPayload(record, ['seed_face_ids', 'seedFaceIds', 'source_face_ids', 'sourceFaceIds', 'selected_face_ids', 'selectedFaceIds']);
  const maxDistance = positiveNumberFromUnknown(record.max_distance_mm ?? record.maxDistanceMm ?? record.max_path_len_mm ?? record.maxPathLenMm);
  const isoValue = nonnegativeNumberFromUnknown(record.iso_value_mm ?? record.isoValueMm ?? record.iso_value ?? record.isoValue ?? record.value);
  const requested = Boolean(
    Object.keys(nested).length > 0
      || seed
      || seedVertex != null
      || seedVertices.length
      || seedEdges.length
      || seedFaceIds.length
      || isoValue != null
      || booleanFromPayload(payload, ['surface_distance', 'surfaceDistance'], false),
  );
  if (!requested) {
    return null;
  }
  return {
    ...(seed ? { seed } : {}),
    ...(seedVertex != null ? { seed_vertex: seedVertex } : {}),
    ...(seedVertices.length ? { seed_vertices: seedVertices } : {}),
    ...(seedEdges.length ? { seed_edges: seedEdges } : {}),
    ...(seedFaceIds.length ? { seed_face_ids: seedFaceIds } : {}),
    ...(maxDistance != null ? { max_distance_mm: maxDistance } : {}),
    ...(isoValue != null ? { iso_value_mm: isoValue } : {}),
    include_distances: booleanFromPayload(record, ['include_distances', 'includeDistances'], true),
    include_iso_segments: booleanFromPayload(record, ['include_iso_segments', 'includeIsoSegments'], true),
    include_extreme_edges: booleanFromPayload(record, ['include_extreme_edges', 'includeExtremeEdges'], false),
  };
}

function measureInspectRequestFromWorkbenchPayload(payload: Record<string, unknown>): MeasureInspectRequest {
  const points: Array<[number, number, number]> = [];
  const pointPairs: MeasureInspectRequest['point_pairs'] = [];
  const surfaceDistance = measureInspectSurfaceDistanceFromWorkbenchPayload(payload);
  const metric = measureInspectMetricFromUnknown(payload.metric ?? payload.distance_metric ?? payload.distanceMetric)
    ?? (booleanFromPayload(payload, ['geodesic', 'surface_distance', 'surfaceDistance'], false) ? 'geodesic' : null);
  for (const key of ['points', 'point', 'point_world', 'world_point', 'points_world', 'world_points', 'position']) {
    appendMeasureInspectPoint(points, payload[key]);
  }
  for (const key of ['point_pairs', 'pairs', 'distance_pairs', 'segments']) {
    appendMeasureInspectPair(pointPairs, payload[key]);
  }
  if (metric) {
    for (const pair of pointPairs) {
      pair.metric ??= metric;
    }
  }
  return {
    points: points.length || pointPairs.length || surfaceDistance ? points : [[0, 0, 0]],
    point_pairs: pointPairs,
    surface_distance: surfaceDistance,
    include_local_thickness: booleanFromPayload(payload, ['include_local_thickness', 'local_thickness'], true),
  };
}

function meshCutMeasureTopologyRequestFromWorkbenchPayload(payload: Record<string, unknown>): MeshCutMeasureTopologyRequest {
  const maxPathLenMm = positiveNumberFromUnknown(
    payload.max_path_len_mm
      ?? payload.maxPathLenMm
      ?? payload.max_path_length_mm
      ?? payload.maxPathLengthMm
      ?? payload.geodesic_max_path_len_mm
      ?? payload.geodesicMaxPathLenMm,
  );
  const operationLabel = stringFromPayload(payload, ['operation_label', 'operationLabel', 'label']);
  return {
    control_vertices: integerListFromPayload(
      payload,
      ['control_vertices', 'controlVertices', 'path_vertex_indices', 'pathVertexIndices', 'path_vertices', 'pathVertices'],
      [],
    ),
    close_path: booleanFromPayload(payload, ['close_path', 'closePath', 'closed_path', 'closedPath', 'closed'], false),
    max_path_len_mm: maxPathLenMm,
    operation_label: operationLabel ?? null,
  };
}

function gcodeRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackSource: string,
): GcodeParsePathsRequest {
  const source = stringFromPayload(payload, ['source', 'gcode_source', 'program', 'text', 'content']) ?? fallbackSource;
  const machineSettings = ['machine_settings', 'cnc_settings', 'settings']
    .map((key) => recordFromUnknown(payload[key]))
    .find((settings) => Object.keys(settings).length > 0) ?? null;
  return {
    source,
    machine_settings: machineSettings,
  };
}

function gcodeFileNameFromWorkbenchPayload(payload: Record<string, unknown>, fallback = 'program.gcode'): string {
  return stringFromPayload(payload, ['file_name', 'fileName', 'name', 'path']) ?? fallback;
}

function gcodeSourceFramesFromWorkbenchPayload(payload: Record<string, unknown>, fallbackSource: string): string[] {
  const explicitFrames = stringListFromPayload(payload, ['source_frames', 'sourceFrames', 'frames']);
  if (explicitFrames.length > 0) {
    return explicitFrames;
  }
  const source = stringFromPayload(payload, ['source', 'gcode_source', 'program', 'text', 'content']) ?? fallbackSource;
  return source.split(/\r?\n/).filter((frame) => frame.length > 0);
}

function gcodeLoadSourceRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackSource: string,
): GcodeLoadSourceRequest {
  const source = stringFromPayload(payload, ['source', 'gcode_source', 'program', 'text', 'content']) ?? fallbackSource;
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload),
    source,
  };
}

function gcodeWriteSourceRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackSource: string,
): GcodeWriteSourceRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload),
    source_frames: gcodeSourceFramesFromWorkbenchPayload(payload, fallbackSource),
  };
}

function gcodeParseFilePathsRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackSource: string,
): GcodeParseFilePathsRequest {
  return {
    ...gcodeLoadSourceRequestFromWorkbenchPayload(payload, fallbackSource),
    machine_settings: gcodeRequestFromWorkbenchPayload(payload, fallbackSource).machine_settings ?? null,
  };
}

const DEFAULT_ICP_REFERENCE_POINTS: Array<[number, number, number]> = [
  [0, 0, 0],
  [10, 0, 0],
  [0, 10, 0],
  [0, 0, 10],
  [8, 8, 8],
];

const DEFAULT_ICP_FLOATING_POINTS: Array<[number, number, number]> = DEFAULT_ICP_REFERENCE_POINTS.map(
  ([x, y, z]) => [x + 0.25, y - 0.1, z + 0.05],
);

const DEFAULT_OFFSET_CONTOURS: Array<Array<[number, number, number]>> = [
  [
    [0, 0, 0],
    [2, 0, 0],
    [2, 2, 0],
    [0, 2, 0],
  ],
];

const DEFAULT_OBJECT_LINES_CONTOURS: Array<Array<[number, number, number]>> = [
  [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
  ],
];

const DEFAULT_DISTANCE_MAP_CONTOURS: Array<Array<[number, number]>> = [
  [
    [0, 0],
    [2, 0],
    [2, 2],
    [0, 2],
    [0, 0],
  ],
];
const DEFAULT_DISTANCE_MAP_CONTOURS_B: Array<Array<[number, number]>> = [
  [
    [1, 0],
    [3, 0],
    [3, 2],
    [1, 2],
    [1, 0],
  ],
];

const DEFAULT_DISTANCE_MAP_VALUES = [
  [-1, 1],
  [-1, 1],
];
const DEFAULT_DISTANCE_MAP_TIFF_VALUES = [
  [1, 2],
  [3, 4],
];
const DEFAULT_DISTANCE_MAP_TIFF_BASE64 =
  'SUkqAL8AAAABAAAAAQAAAAEAAAABAAAALTMuNDAyODIzNDY2Mzg1Mjg4NmUzOAAAAAAAAAAEQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAkQAAAAAAAAAAAAAAAAAAAEEAAAAAAAAAAAAAAAAAAADRAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPA/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADwPwAAgD8AAABAAABAQAAAgEAQAAABBAABAAAAAgAAAAEBBAABAAAAAgAAAAIBAwABAAAAIAAAAAMBAwABAAAAAQAAAAYBAwABAAAAAAAAABEBBAABAAAArwAAABUBAwABAAAAAQAAABYBBAABAAAASOgBABcBBAABAAAAEAAAABoBBQABAAAACAAAABsBBQABAAAAEAAAACgBAwABAAAAAQAAAD0BAwABAAAAAQAAAFMBAwABAAAAAwAAANiFDAAQAAAALwAAAIGkAgAXAAAAGAAAAAAAAAA=';
const DEFAULT_RAW_VOXELS_BASE64 = 'AAAAgP//AEA=';
const DEFAULT_TIFF_VOXEL_SLICE_10_BASE64 =
  'SUkqAAgAAAAKAAABBAABAAAAAgAAAAEBBAABAAAAAQAAAAIBAwABAAAAIAAAAAMBAwABAAAAAQAAAAYBAwABAAAAAQAAABEBBAABAAAAhgAAABYBBAABAAAAAQAAABcBBAABAAAACAAAABwBAwABAAAAAQAAAFMBAwABAAAAAwAAAAAAAAAAACBBAAAwQQ==';
const DEFAULT_TIFF_VOXEL_SLICE_02_BASE64 =
  'SUkqAAgAAAAKAAABBAABAAAAAgAAAAEBBAABAAAAAQAAAAIBAwABAAAAIAAAAAMBAwABAAAAAQAAAAYBAwABAAAAAQAAABEBBAABAAAAhgAAABYBBAABAAAAAQAAABcBBAABAAAACAAAABwBAwABAAAAAQAAAFMBAwABAAAAAwAAAAAAAAAAAABAAABAQA==';
const DISTANCE_MAP_INVALID_VALUE = -3.4028234663852886e38;
const DEFAULT_DISTANCE_MAP_MERGE_RIGHT_VALUES = [
  [3, 5],
  [DISTANCE_MAP_INVALID_VALUE, 6],
];

const DEFAULT_OBJECT_LINES: Record<string, unknown> = {
  Type: ['LinesHolder', 'ObjectLines'],
  ShowPoints: 0,
  SmoothConnections: 0,
  ColoringType: 'Solid',
  LineColors: [],
  VertColors: [],
  LineWidth: 1,
  Polyline: {
    Points: [
      [0, 0, 0],
      [1, 0, 0],
      [1, 1, 0],
    ],
    Lines: [0, 1, 1, 2],
  },
};
const DEFAULT_OBJECT_LINES_PTS_SOURCE =
  'BEGIN_Polyline\n' +
  '0 0 0\n' +
  '1.25 0 0\n' +
  '1.25 1.5 0\n' +
  'END_Polyline\n' +
  'BEGIN_Polyline\n' +
  '2 -1 0.5\n' +
  '3 -1 0.5\n' +
  'END_Polyline\n';
const DEFAULT_OBJECT_LINES_SVG_SOURCE =
  '<svg xmlns="http://www.w3.org/2000/svg">' +
  '<line x1="1" y1="2" x2="4" y2="6" />' +
  '<polyline points="0,0 2,0 2,2" />' +
  '</svg>';
const DEFAULT_OBJECT_LINES_MRLINES_BASE64 =
  'AgAAAAAAAAAAAAAAAQAAAAEAAAACAAAAAAAAAAEAAAADAAAAAgAAAAAAAAAAAAAAAAAAAAAAgD8AAABAAABAQA==';
const DEFAULT_OBJECT_LINES_PLY_BASE64 =
  'cGx5CmZvcm1hdCBiaW5hcnlfbGl0dGxlX2VuZGlhbiAxLjAKY29tbWVudCBNZXNoSW5zcGVjdG9yLmNvbQplbGVtZW50IHZlcnRleCAyCnByb3BlcnR5IGZsb2F0IHgKcHJvcGVydHkgZmxvYXQgeQpwcm9wZXJ0eSBmbG9hdCB6CmVsZW1lbnQgZWRnZSAxCnByb3BlcnR5IGludCB2ZXJ0ZXgxCnByb3BlcnR5IGludCB2ZXJ0ZXgyCmVuZF9oZWFkZXIKAAAAAAAAAAAAAAAAAACAPwAAAEAAAEBAAAAAAAEAAAA=';

function pointArrayFromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: Array<[number, number, number]>,
): Array<[number, number, number]> {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value)) {
      continue;
    }
    const points = value
      .map((item): [number, number, number] | null => {
        if (!Array.isArray(item) || item.length < 3) return null;
        const x = Number(item[0]);
        const y = Number(item[1]);
        const z = Number(item[2]);
        return [x, y, z].every(Number.isFinite) ? [x, y, z] : null;
      })
      .filter((point): point is [number, number, number] => point != null);
    if (points.length > 0) {
      return points;
    }
  }
  return fallback.map((point) => [...point] as [number, number, number]);
}

function contourArrayFromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: Array<Array<[number, number, number]>>,
): Array<Array<[number, number, number]>> {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value)) {
      continue;
    }
    const candidateContours = value
      .map((contour): Array<[number, number, number]> | null => {
        if (!Array.isArray(contour)) {
          return null;
        }
        const points = contour
          .map((item): [number, number, number] | null => {
            if (!Array.isArray(item) || item.length < 2) return null;
            const x = Number(item[0]);
            const y = Number(item[1]);
            const z = item.length > 2 ? Number(item[2]) : 0;
            return [x, y, z].every(Number.isFinite) ? [x, y, z] : null;
          })
          .filter((point): point is [number, number, number] => point != null);
        return points.length >= 2 ? points : null;
      })
      .filter((contour): contour is Array<[number, number, number]> => contour != null);
    if (candidateContours.length > 0) {
      return candidateContours;
    }
  }
  return fallback.map((contour) => contour.map((point) => [...point] as [number, number, number]));
}

function contour2ArrayFromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: Array<Array<[number, number]>>,
): Array<Array<[number, number]>> {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value)) {
      continue;
    }
    const candidateContours = value
      .map((contour): Array<[number, number]> | null => {
        if (!Array.isArray(contour)) {
          return null;
        }
        const points = contour
          .map((item): [number, number] | null => {
            if (!Array.isArray(item) || item.length < 2) return null;
            const x = Number(item[0]);
            const y = Number(item[1]);
            return [x, y].every(Number.isFinite) ? [x, y] : null;
          })
          .filter((point): point is [number, number] => point != null);
        return points.length >= 2 ? points : null;
      })
      .filter((contour): contour is Array<[number, number]> => contour != null);
    if (candidateContours.length > 0) {
      return candidateContours;
    }
  }
  return fallback.map((contour) => contour.map((point) => [...point] as [number, number]));
}

function numberMatrixFromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: number[][],
): number[][] {
  for (const key of keys) {
    const value = payload[key];
    if (!Array.isArray(value)) {
      continue;
    }
    const matrix = value
      .map((row): number[] | null => {
        if (!Array.isArray(row)) {
          return null;
        }
        const values = row.map(Number);
        return values.length > 0 && values.every(Number.isFinite) ? values : null;
      })
      .filter((row): row is number[] => row != null);
    if (matrix.length > 0) {
      return matrix;
    }
  }
  return fallback.map((row) => [...row]);
}

function cloneObjectLinesPayload(payload: Record<string, unknown>): Record<string, unknown> {
  return JSON.parse(JSON.stringify(payload)) as Record<string, unknown>;
}

function objectLinesPayloadFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: Record<string, unknown> | null,
): Record<string, unknown> {
  for (const key of ['object_lines', 'objectLines', 'lines_object', 'linesObject']) {
    const value = recordFromUnknown(payload[key]);
    if (Object.keys(value).length > 0) {
      return value;
    }
  }
  const polyline = recordFromUnknown(payload.Polyline ?? payload.polyline);
  if (Object.keys(polyline).length > 0) {
    return {
      Type: ['LinesHolder', 'ObjectLines'],
      ShowPoints: Math.max(0, Math.round(numberFromPayload(payload, ['show_points', 'showPoints'], 0))),
      SmoothConnections: Math.max(0, Math.round(numberFromPayload(payload, ['smooth_connections', 'smoothConnections'], 0))),
      ColoringType: stringFromPayload(payload, ['coloring_type', 'coloringType']) ?? 'Solid',
      LineColors: [],
      VertColors: [],
      LineWidth: numberFromPayload(payload, ['line_width', 'lineWidth'], 1),
      Polyline: polyline,
    };
  }
  return cloneObjectLinesPayload(fallback ?? DEFAULT_OBJECT_LINES);
}

function offsetContoursRequestFromWorkbenchPayload(payload: Record<string, unknown>): OffsetContoursRequest {
  const mode = stringFromPayload(payload, ['mode', 'type']);
  const endType = stringFromPayload(payload, ['end_type', 'endType']);
  const cornerType = stringFromPayload(payload, ['corner_type', 'cornerType']);
  const zRestore = stringFromPayload(payload, ['z_restore', 'zRestore']);
  return {
    contours: contourArrayFromPayload(payload, ['contours', 'contour', 'polylines', 'paths'], DEFAULT_OFFSET_CONTOURS),
    offset: numberFromPayload(payload, ['offset', 'offset_mm', 'offsetMm', 'distance', 'distance_mm'], 0.25),
    min_angle_precision: numberFromPayload(payload, ['min_angle_precision', 'minAnglePrecision'], Math.PI / 9),
    mode: mode === 'shell' ? 'shell' : 'offset',
    end_type: endType === 'cut' ? 'cut' : 'round',
    corner_type: cornerType === 'sharp' ? 'sharp' : 'round',
    max_sharp_angle: numberFromPayload(payload, ['max_sharp_angle', 'maxSharpAngle'], (Math.PI * 2) / 3),
    z_restore: zRestore === 'none' || zRestore === 'constant' || zRestore === 'custom' ? zRestore : 'default',
    z_value: optionalNumberFromPayload(payload, ['z_value', 'zValue']),
    relax_iterations: Math.max(0, Math.round(numberFromPayload(payload, ['relax_iterations', 'relaxIterations'], 1))),
    include_origins: booleanFromPayload(payload, ['include_origins', 'includeOrigins', 'origins'], true),
  };
}

function distanceMapFromMeshRequestFromWorkbenchPayload(payload: Record<string, unknown>): DistanceMapFromMeshRequest {
  return {
    width: Math.max(1, Math.round(numberFromPayload(payload, ['width', 'res_x', 'resX'], 2))),
    height: Math.max(1, Math.round(numberFromPayload(payload, ['height', 'res_y', 'resY'], 2))),
    origin: vector3FromPayload(
      payload,
      ['origin', 'frame_origin', 'frameOrigin'],
      [['origin_x', 'originX'], ['origin_y', 'originY'], ['origin_z', 'originZ']],
      [0, 0, 0],
    ),
    x_range: vector3FromPayload(
      payload,
      ['x_range', 'xRange'],
      [['x_range_x', 'xRangeX'], ['x_range_y', 'xRangeY'], ['x_range_z', 'xRangeZ']],
      [2, 0, 0],
    ),
    y_range: vector3FromPayload(
      payload,
      ['y_range', 'yRange'],
      [['y_range_x', 'yRangeX'], ['y_range_y', 'yRangeY'], ['y_range_z', 'yRangeZ']],
      [0, 2, 0],
    ),
    direction: vector3FromPayload(
      payload,
      ['direction', 'ray_direction', 'rayDirection'],
      [['direction_x', 'directionX'], ['direction_y', 'directionY'], ['direction_z', 'directionZ']],
      [0, 0, 1],
    ),
    epsilon: numberFromPayload(payload, ['epsilon'], 1e-8),
  };
}

function distanceMapContoursRequestFromWorkbenchPayload(payload: Record<string, unknown>): DistanceMapContoursRequest {
  return {
    contours: contour2ArrayFromPayload(
      payload,
      ['contours', 'contour', 'polylines', 'paths'],
      DEFAULT_DISTANCE_MAP_CONTOURS,
    ),
    width: Math.max(1, Math.round(numberFromPayload(payload, ['width', 'res_x', 'resX'], 3))),
    height: Math.max(1, Math.round(numberFromPayload(payload, ['height', 'res_y', 'resY'], 3))),
    origin: [
      numberFromPayload(payload, ['origin_x', 'originX'], 0),
      numberFromPayload(payload, ['origin_y', 'originY'], 0),
    ],
    pixel_size: [
      numberFromPayload(payload, ['pixel_size_x', 'pixelSizeX'], 1),
      numberFromPayload(payload, ['pixel_size_y', 'pixelSizeY'], 1),
    ],
    signed: booleanFromPayload(payload, ['signed', 'with_sign', 'withSign'], true),
  };
}

function distanceMapIsoLinesRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: DistanceMapResponse | null,
): DistanceMapIsoLinesRequest {
  const fallbackValues = fallback?.values ?? DEFAULT_DISTANCE_MAP_VALUES;
  const values = numberMatrixFromPayload(payload, ['values', 'distance_values', 'distanceValues'], fallbackValues);
  return {
    width: Math.max(1, Math.round(numberFromPayload(payload, ['width', 'res_x', 'resX'], fallback?.width ?? 2))),
    height: Math.max(1, Math.round(numberFromPayload(payload, ['height', 'res_y', 'resY'], fallback?.height ?? 2))),
    origin: [
      numberFromPayload(payload, ['origin_x', 'originX'], fallback?.origin[0] ?? 0),
      numberFromPayload(payload, ['origin_y', 'originY'], fallback?.origin[1] ?? 0),
    ],
    pixel_size: [
      numberFromPayload(payload, ['pixel_size_x', 'pixelSizeX'], fallback?.pixel_size[0] ?? 1),
      numberFromPayload(payload, ['pixel_size_y', 'pixelSizeY'], fallback?.pixel_size[1] ?? 1),
    ],
    values,
    valid_count: Math.max(
      0,
      Math.round(numberFromPayload(payload, ['valid_count', 'validCount'], fallback?.valid_count ?? values.length * (values[0]?.length ?? 0))),
    ),
    min_value: numberFromPayload(payload, ['min_value', 'minValue'], fallback?.min_value ?? Math.min(...values.flat())),
    max_value: numberFromPayload(payload, ['max_value', 'maxValue'], fallback?.max_value ?? Math.max(...values.flat())),
    model_transform: fallback?.model_transform ?? null,
    unit: stringFromPayload(payload, ['unit']) ?? fallback?.unit ?? 'mm',
    iso_value: numberFromPayload(payload, ['iso_value', 'isoValue', 'level'], 0),
  };
}

function finiteValues(values: number[][]): number[] {
  return values.flat().filter(Number.isFinite);
}

function distanceMapPayloadFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackValues: number[][],
  fallback?: DistanceMapResponse | null,
) {
  const values = numberMatrixFromPayload(payload, ['values', 'distance_values', 'distanceValues'], fallback?.values ?? fallbackValues);
  const finite = finiteValues(values);
  return {
    width: Math.max(1, Math.round(numberFromPayload(payload, ['width', 'res_x', 'resX'], fallback?.width ?? values[0]?.length ?? 1))),
    height: Math.max(1, Math.round(numberFromPayload(payload, ['height', 'res_y', 'resY'], fallback?.height ?? values.length))),
    origin: [
      numberFromPayload(payload, ['origin_x', 'originX'], fallback?.origin[0] ?? 0),
      numberFromPayload(payload, ['origin_y', 'originY'], fallback?.origin[1] ?? 0),
    ] as [number, number],
    pixel_size: [
      numberFromPayload(payload, ['pixel_size_x', 'pixelSizeX'], fallback?.pixel_size[0] ?? 1),
      numberFromPayload(payload, ['pixel_size_y', 'pixelSizeY'], fallback?.pixel_size[1] ?? 1),
    ] as [number, number],
    values,
    valid_count: Math.max(
      0,
      Math.round(numberFromPayload(payload, ['valid_count', 'validCount'], fallback?.valid_count ?? finite.length)),
    ),
    min_value: numberFromPayload(payload, ['min_value', 'minValue'], fallback?.min_value ?? Math.min(...finite)),
    max_value: numberFromPayload(payload, ['max_value', 'maxValue'], fallback?.max_value ?? Math.max(...finite)),
    model_transform: fallback?.model_transform ?? null,
    unit: stringFromPayload(payload, ['unit']) ?? fallback?.unit ?? 'mm',
  };
}

function distanceMapMergeRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: DistanceMapResponse | null,
): DistanceMapMergeRequest {
  const leftPayload = recordFromUnknown(payload.left ?? payload.left_map ?? payload.leftMap ?? payload.map_a ?? payload.mapA);
  const rightPayload = recordFromUnknown(payload.right ?? payload.right_map ?? payload.rightMap ?? payload.map_b ?? payload.mapB);
  const mode = stringFromPayload(payload, ['mode', 'merge_mode', 'mergeMode', 'operation']);
  return {
    left: distanceMapPayloadFromWorkbenchPayload(
      Object.keys(leftPayload).length > 0 ? leftPayload : payload,
      DEFAULT_DISTANCE_MAP_VALUES,
      fallback,
    ),
    right: distanceMapPayloadFromWorkbenchPayload(
      rightPayload,
      DEFAULT_DISTANCE_MAP_MERGE_RIGHT_VALUES,
      null,
    ),
    mode: mode === 'max' || mode === 'subtract' ? mode : 'min',
  };
}

function distanceMapContourBooleanRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
): DistanceMapContourBooleanRequest {
  const mode = stringFromPayload(payload, ['mode', 'boolean_mode', 'booleanMode', 'operation']);
  return {
    contours_a: contour2ArrayFromPayload(
      payload,
      ['contours_a', 'contoursA', 'contour_a', 'contourA', 'left_contours', 'leftContours'],
      DEFAULT_DISTANCE_MAP_CONTOURS,
    ),
    contours_b: contour2ArrayFromPayload(
      payload,
      ['contours_b', 'contoursB', 'contour_b', 'contourB', 'right_contours', 'rightContours'],
      DEFAULT_DISTANCE_MAP_CONTOURS_B,
    ),
    mode: mode === 'intersection' || mode === 'subtract' ? mode : 'union',
    width: Math.max(1, Math.round(numberFromPayload(payload, ['width', 'res_x', 'resX'], 6))),
    height: Math.max(1, Math.round(numberFromPayload(payload, ['height', 'res_y', 'resY'], 5))),
    origin: [
      numberFromPayload(payload, ['origin_x', 'originX'], -1),
      numberFromPayload(payload, ['origin_y', 'originY'], -1),
    ],
    pixel_size: [
      numberFromPayload(payload, ['pixel_size_x', 'pixelSizeX'], 1),
      numberFromPayload(payload, ['pixel_size_y', 'pixelSizeY'], 1),
    ],
    iso_value: numberFromPayload(payload, ['iso_value', 'isoValue', 'offset_inside', 'offsetInside'], 0),
  };
}

function distanceMapTiffImportRequestFromWorkbenchPayload(payload: Record<string, unknown>): DistanceMapTiffImportRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'height-field.tiff'),
    contents_base64:
      stringFromPayload(payload, ['contents_base64', 'contentsBase64', 'tiff_base64', 'tiffBase64', 'data']) ??
      DEFAULT_DISTANCE_MAP_TIFF_BASE64,
  };
}

function distanceMapTiffExportRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: DistanceMapResponse | null,
): DistanceMapTiffExportRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'exported-height-field.tiff'),
    ...distanceMapPayloadFromWorkbenchPayload(
      payload,
      DEFAULT_DISTANCE_MAP_TIFF_VALUES,
      fallback,
    ),
  };
}

function objectLinesFromContoursRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
): ObjectLinesFromContoursRequest {
  return {
    contours: contourArrayFromPayload(
      payload,
      ['contours', 'contour', 'polylines', 'paths'],
      DEFAULT_OBJECT_LINES_CONTOURS,
    ),
    line_width: numberFromPayload(payload, ['line_width', 'lineWidth'], 1),
    show_points: Math.max(0, Math.round(numberFromPayload(payload, ['show_points', 'showPoints'], 0))),
    smooth_connections: Math.max(
      0,
      Math.round(numberFromPayload(payload, ['smooth_connections', 'smoothConnections'], 0)),
    ),
  };
}

function objectLinesToContoursRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: Record<string, unknown> | null,
): ObjectLinesToContoursRequest {
  return {
    object_lines: objectLinesPayloadFromWorkbenchPayload(payload, fallback),
  };
}

function objectLinesMrLinesLoadRequestFromWorkbenchPayload(payload: Record<string, unknown>): ObjectLinesBinaryLoadRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.mrlines'),
    contents_base64:
      stringFromPayload(payload, ['contents_base64', 'contentsBase64', 'mrlines_base64', 'mrLinesBase64', 'data']) ??
      DEFAULT_OBJECT_LINES_MRLINES_BASE64,
  };
}

function objectLinesMrLinesSaveRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: Record<string, unknown> | null,
): ObjectLinesBinaryExportRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.mrlines'),
    object_lines: objectLinesPayloadFromWorkbenchPayload(payload, fallback),
  };
}

function objectLinesPlyLoadRequestFromWorkbenchPayload(payload: Record<string, unknown>): ObjectLinesBinaryLoadRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.ply'),
    contents_base64:
      stringFromPayload(payload, ['contents_base64', 'contentsBase64', 'ply_base64', 'plyBase64', 'data']) ??
      DEFAULT_OBJECT_LINES_PLY_BASE64,
  };
}

function objectLinesPlySaveRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: Record<string, unknown> | null,
): ObjectLinesBinaryExportRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.ply'),
    object_lines: objectLinesPayloadFromWorkbenchPayload(payload, fallback),
  };
}

function objectLinesPtsLoadRequestFromWorkbenchPayload(payload: Record<string, unknown>): ObjectLinesPtsLoadRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.pts'),
    source:
      stringFromPayload(payload, ['source', 'pts_source', 'ptsSource', 'text', 'content']) ??
      DEFAULT_OBJECT_LINES_PTS_SOURCE,
  };
}

function objectLinesPtsSaveRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: Record<string, unknown> | null,
): ObjectLinesTextExportRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.pts'),
    object_lines: objectLinesPayloadFromWorkbenchPayload(payload, fallback),
  };
}

function objectLinesSvgLoadRequestFromWorkbenchPayload(payload: Record<string, unknown>): ObjectLinesSvgLoadRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.svg'),
    source:
      stringFromPayload(payload, ['source', 'svg_source', 'svgSource', 'text', 'content']) ??
      DEFAULT_OBJECT_LINES_SVG_SOURCE,
  };
}

function objectLinesDxfSaveRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback?: Record<string, unknown> | null,
): ObjectLinesTextExportRequest {
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'object-lines.dxf'),
    object_lines: objectLinesPayloadFromWorkbenchPayload(payload, fallback),
  };
}

function pointCloudIcpRequestFromWorkbenchPayload(payload: Record<string, unknown>): PointCloudIcpRequest {
  const methodRaw = stringFromPayload(payload, ['method', 'icp_method', 'metric']);
  const modeRaw = stringFromPayload(payload, ['mode', 'transform_mode', 'transformation_mode']);
  const referenceNormals = pointArrayFromPayload(
    payload,
    ['reference_normals', 'target_normals', 'fixed_normals'],
    [],
  );
  const floatingNormals = pointArrayFromPayload(
    payload,
    ['floating_normals', 'source_normals', 'moving_normals'],
    [],
  );
  const maxPairDistance = optionalNumberFromPayload(payload, ['max_pair_distance', 'maxPairDistance', 'distance_threshold']);
  const cosThreshold = optionalNumberFromPayload(payload, ['cos_threshold', 'cosThreshold', 'normal_cosine_threshold']);
  const farDistFactor = optionalNumberFromPayload(payload, ['far_dist_factor', 'farDistFactor']);
  return {
    floating_points: pointArrayFromPayload(
      payload,
      ['floating_points', 'source_points', 'moving_points', 'floating', 'source'],
      DEFAULT_ICP_FLOATING_POINTS,
    ),
    reference_points: pointArrayFromPayload(
      payload,
      ['reference_points', 'target_points', 'fixed_points', 'reference', 'target'],
      DEFAULT_ICP_REFERENCE_POINTS,
    ),
    method: methodRaw === 'point_to_plane' || methodRaw === 'point-to-plane' ? 'point_to_plane' : 'point_to_point',
    mode: modeRaw === 'translation' || modeRaw === 'translation_only' ? 'translation' : 'rigid',
    max_iterations: Math.max(1, Math.round(numberFromPayload(payload, ['max_iterations', 'maxIterations', 'iterations'], 20))),
    tolerance: Math.max(Number.EPSILON, numberFromPayload(payload, ['tolerance', 'exit_val', 'exitVal'], 1e-8)),
    reference_normals: referenceNormals.length > 0 ? referenceNormals : null,
    floating_normals: floatingNormals.length > 0 ? floatingNormals : null,
    max_pair_distance: maxPairDistance && maxPairDistance > 0 ? maxPairDistance : null,
    cos_threshold: cosThreshold != null ? Math.max(-1, Math.min(1, cosThreshold)) : null,
    far_dist_factor: farDistFactor && farDistFactor > 0 ? farDistFactor : null,
    mutual_closest: booleanFromPayload(payload, ['mutual_closest', 'mutualClosest', 'reciprocal'], false),
  };
}

function meshToVoxelsRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    voxelSizeMm: number;
    voxelSurfaceOffsetVoxels: number;
    voxelMode: 'signed' | 'unsigned';
    voxelExtractSurface: boolean;
  },
): MeshToVoxelsSdfRequest {
  const requestedMode = stringFromPayload(payload, ['mode', 'type', 'conversion_type']);
  return {
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    surface_offset_voxels: numberFromPayload(
      payload,
      ['surface_offset_voxels', 'surfaceOffsetVoxels', 'surface_offset', 'surfaceOffset'],
      fallback.voxelSurfaceOffsetVoxels,
    ),
    mode: requestedMode === 'unsigned' ? 'unsigned' : requestedMode === 'signed' ? 'signed' : fallback.voxelMode,
    iso_value: numberFromPayload(payload, ['iso_value', 'isoValue', 'iso'], 0),
    extract_surface: booleanFromPayload(
      payload,
      ['extract_surface', 'extractSurface', 'surface', 'extract_iso_surface'],
      fallback.voxelExtractSurface,
    ),
  };
}

function openRawVoxelsRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelRawLoadRequest {
  const autoParameters = booleanFromPayload(payload, ['auto_parameters', 'autoParameters', 'auto'], false);
  return {
    file_name: gcodeFileNameFromWorkbenchPayload(payload, 'explicit.raw'),
    contents_base64:
      stringFromPayload(payload, ['contents_base64', 'contentsBase64', 'raw_base64', 'rawBase64', 'data']) ??
      DEFAULT_RAW_VOXELS_BASE64,
    dimensions: autoParameters ? null : integerTuple3FromPayload(payload, ['dimensions', 'shape', 'dims'], [2, 2, 1]),
    voxel_size: vector3FromPayload(
      payload,
      ['voxel_size', 'voxelSize'],
      [['voxel_size_x', 'voxelSizeX'], ['voxel_size_y', 'voxelSizeY'], ['voxel_size_z', 'voxelSizeZ']],
      [0.5, 1.0, 2.0],
    ),
    scalar_type: stringFromPayload(payload, ['scalar_type', 'scalarType']) ?? 'uint16',
    grid_level_set: booleanFromPayload(payload, ['grid_level_set', 'gridLevelSet', 'level_set', 'levelSet'], false),
    auto_parameters: autoParameters,
  };
}

function openVoxelsFromTiffRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelTiffLoadRequest {
  const slice10 =
    stringFromPayload(payload, ['slice_10_base64', 'slice10Base64', 'tiff_base64', 'tiffBase64']) ??
    DEFAULT_TIFF_VOXEL_SLICE_10_BASE64;
  const slice02 =
    stringFromPayload(payload, ['slice_02_base64', 'slice02Base64']) ??
    DEFAULT_TIFF_VOXEL_SLICE_02_BASE64;
  return {
    files: {
      'slice_10.tiff': slice10,
      'slice_02.tiff': slice02,
    },
    voxel_size: vector3FromPayload(
      payload,
      ['voxel_size', 'voxelSize'],
      [['voxel_size_x', 'voxelSizeX'], ['voxel_size_y', 'voxelSizeY'], ['voxel_size_z', 'voxelSizeZ']],
      [0.5, 0.25, 2.0],
    ),
    grid_level_set: booleanFromPayload(payload, ['grid_level_set', 'gridLevelSet', 'level_set', 'levelSet'], false),
  };
}

function integerTuple3FromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: [number, number, number],
): [number, number, number] {
  const values = integerListFromPayload(payload, keys, fallback);
  if (values.length < 3) {
    return fallback;
  }
  return [
    Math.max(1, Math.round(values[0])),
    Math.max(1, Math.round(values[1])),
    Math.max(1, Math.round(values[2])),
  ];
}

function nonnegativeIntegerTuple3FromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: [number, number, number],
): [number, number, number] {
  const values = integerListFromPayload(payload, keys, fallback);
  if (values.length < 3) {
    return fallback;
  }
  return [
    Math.max(0, Math.round(values[0])),
    Math.max(0, Math.round(values[1])),
    Math.max(0, Math.round(values[2])),
  ];
}

function nonnegativeIntegerTuple3ListFromPayload(
  payload: Record<string, unknown>,
  keys: string[],
  fallback: Array<[number, number, number]>,
): Array<[number, number, number]> {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'string' && value.trim() !== '') {
      const parsed = parseIntegerList(value);
      if (parsed.length >= 3) {
        const tuples: Array<[number, number, number]> = [];
        for (let index = 0; index + 2 < parsed.length; index += 3) {
          tuples.push([parsed[index], parsed[index + 1], parsed[index + 2]]);
        }
        if (tuples.length > 0) {
          return tuples;
        }
      }
    }
    if (Array.isArray(value)) {
      const tuples = value
        .map((item): [number, number, number] | null => {
          if (!Array.isArray(item) || item.length < 3) return null;
          const x = Number(item[0]);
          const y = Number(item[1]);
          const z = Number(item[2]);
          return Number.isInteger(x) && Number.isInteger(y) && Number.isInteger(z) && x >= 0 && y >= 0 && z >= 0
            ? [x, y, z]
            : null;
        })
        .filter((item): item is [number, number, number] => item != null);
      if (tuples.length > 0) {
        return tuples;
      }
    }
  }
  return fallback;
}

function tuple4FromPayload(payload: Record<string, unknown>, keys: string[]): [number, number, number, number] | null {
  const values = numberListFromPayload(payload, keys);
  if (values.length < 4) {
    return null;
  }
  return [values[0], values[1], values[2], values[3]];
}

function voxelLineGraphRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelLineGraphRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [3, 2, 2]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => {
          const x = index % shape[0];
          const y = Math.floor(index / shape[0]) % shape[1];
          const z = Math.floor(index / (shape[0] * shape[1]));
          return x + 10 * y + 100 * z;
        });

  return {
    values,
    shape,
    axis: stringFromPayload(payload, ['axis', 'probe_axis', 'line_axis']) ?? 'x',
    fixed_coordinate: nonnegativeIntegerTuple3FromPayload(
      payload,
      ['fixed_coordinate', 'fixedCoordinate', 'coordinate', 'probe_coordinate'],
      [0, Math.min(1, shape[1] - 1), Math.min(1, shape[2] - 1)],
    ),
  };
}

function voxelBinaryOperationsRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelBinaryOperationsRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [2, 2, 2]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedLeftValues = numberListFromPayload(payload, ['left_values', 'leftValues', 'a_values', 'aValues']);
  const requestedRightValues = numberListFromPayload(payload, ['right_values', 'rightValues', 'b_values', 'bValues']);
  const leftValues =
    requestedLeftValues.length >= valueCount
      ? requestedLeftValues.slice(0, valueCount)
      : [1, 2, 3, 4, -1, -2, -3, -4].slice(0, valueCount);
  const rightValues =
    requestedRightValues.length >= valueCount
      ? requestedRightValues.slice(0, valueCount)
      : [0.5, -0.5, 1.5, -1.5, 2, -2, 4, -4].slice(0, valueCount);

  return {
    left_values: leftValues,
    right_values: rightValues,
    shape,
    origin: vectorFromPayloadKeys(payload, ['origin', 'grid_origin', 'gridOrigin']) ?? [0, 0, 0],
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], 1),
    operation: stringFromPayload(payload, ['operation', 'binary_operation', 'binaryOperation']) ?? 'sum',
    left_iso_value: numberFromPayload(payload, ['left_iso_value', 'leftIsoValue'], 1),
    right_iso_value: numberFromPayload(payload, ['right_iso_value', 'rightIsoValue'], 2),
  };
}

function voxelActiveBoxRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelActiveBoxRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [4, 3, 2]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => {
          const x = index % shape[0];
          const y = Math.floor(index / shape[0]) % shape[1];
          const z = Math.floor(index / (shape[0] * shape[1]));
          return x + 10 * y + 100 * z;
        });

  return {
    values,
    shape,
    min_corner: nonnegativeIntegerTuple3FromPayload(
      payload,
      ['min_corner', 'minCorner', 'active_min_corner', 'activeMinCorner'],
      [1, 1, 0],
    ),
    dimensions: integerTuple3FromPayload(
      payload,
      ['dimensions', 'active_dimensions', 'activeDimensions', 'active_shape', 'activeShape'],
      [2, 2, 2],
    ),
  };
}

function voxelSliceRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelSliceRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [2, 3, 4]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => {
          const x = index % shape[0];
          const y = Math.floor(index / shape[0]) % shape[1];
          const z = Math.floor(index / (shape[0] * shape[1]));
          return x + 10 * y + 100 * z;
        });

  return {
    values,
    shape,
    plane: stringFromPayload(payload, ['plane', 'slice_plane', 'axis_plane']) ?? 'xy',
    slice_index: Math.max(0, Math.round(numberFromPayload(payload, ['slice_index', 'sliceIndex', 'index'], 2))),
    min_value: numberFromPayload(payload, ['min_value', 'minValue'], 200),
    max_value: numberFromPayload(payload, ['max_value', 'maxValue'], 221),
  };
}

function voxelPathRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelPathRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [3, 3, 1]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => (index === 4 ? 10 : 0));

  return {
    values,
    shape,
    start: nonnegativeIntegerTuple3FromPayload(payload, ['start', 'start_coordinate', 'startCoordinate'], [0, 1, 0]),
    finish: nonnegativeIntegerTuple3FromPayload(payload, ['finish', 'end', 'finish_coordinate', 'finishCoordinate'], [2, 1, 0]),
    metric: stringFromPayload(payload, ['metric', 'path_metric']) ?? 'difference',
    max_dist_ratio: numberFromPayload(payload, ['max_dist_ratio', 'maxDistRatio'], 1.5),
    plane: stringFromPayload(payload, ['plane', 'slice_plane']) ?? 'none',
    quarters_mask: Math.max(0, Math.round(numberFromPayload(payload, ['quarters_mask', 'quartersMask'], 15))),
    exponent_modifier: numberFromPayload(payload, ['exponent_modifier', 'exponentModifier'], -1),
  };
}

function voxelPathBuildFourRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelPathBuildFourRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [5, 5, 5]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values = requestedValues.length >= valueCount ? requestedValues.slice(0, valueCount) : Array.from({ length: valueCount }, () => 0);

  return {
    values,
    shape,
    start: nonnegativeIntegerTuple3FromPayload(payload, ['start', 'start_coordinate', 'startCoordinate'], [0, 2, 2]),
    finish: nonnegativeIntegerTuple3FromPayload(payload, ['finish', 'end', 'finish_coordinate', 'finishCoordinate'], [4, 2, 2]),
    metric: stringFromPayload(payload, ['metric', 'path_metric']) ?? 'difference',
    max_dist_ratio: numberFromPayload(payload, ['max_dist_ratio', 'maxDistRatio'], 1.5),
    plane: stringFromPayload(payload, ['plane', 'slice_plane']) ?? 'none',
    exponent_modifier: numberFromPayload(payload, ['exponent_modifier', 'exponentModifier'], -1),
  };
}

function voxelSegmentationRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelSegmentationRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [5, 5, 5]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => (index === Math.floor(valueCount / 2) ? 10 : 0));

  return {
    values,
    shape,
    voxel_size: vectorFromPayloadKeys(payload, ['voxel_size', 'voxelSize', 'spacing']) ?? [0.5, 1, 2],
    inside_seeds: nonnegativeIntegerTuple3ListFromPayload(payload, ['inside_seeds', 'insideSeeds', 'seeds'], [[2, 2, 2]]),
    outside_seeds: nonnegativeIntegerTuple3ListFromPayload(payload, ['outside_seeds', 'outsideSeeds'], []),
    exponent_modifier: numberFromPayload(payload, ['exponent_modifier', 'exponentModifier'], 3000),
    voxels_expansion: Math.max(0, Math.round(numberFromPayload(payload, ['voxels_expansion', 'voxelsExpansion'], 25))),
    include_boundary_outside: booleanFromPayload(payload, ['include_boundary_outside', 'includeBoundaryOutside'], true),
  };
}

function voxelMaskToMeshRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelMaskToMeshRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [5, 5, 5]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => (index === Math.floor(valueCount / 2) ? 10 : 0));

  return {
    values,
    shape,
    voxel_size: vectorFromPayloadKeys(payload, ['voxel_size', 'voxelSize', 'spacing']) ?? [0.5, 1, 2],
    mask_coordinates: nonnegativeIntegerTuple3ListFromPayload(payload, ['mask_coordinates', 'maskCoordinates', 'mask'], [[2, 2, 2]]),
    mask_expansion: Math.max(0, Math.round(numberFromPayload(payload, ['mask_expansion', 'maskExpansion'], 25))),
    smooth_band_radius: Math.max(0, Math.round(numberFromPayload(payload, ['smooth_band_radius', 'smoothBandRadius'], 3))),
  };
}

function voxelToMeshSimpleRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelToMeshSimpleRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [5, 5, 5]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => (index === Math.floor(valueCount / 2) ? 10 : 0));

  return {
    values,
    shape,
    voxel_size: vectorFromPayloadKeys(payload, ['voxel_size', 'voxelSize', 'spacing']) ?? [0.5, 1, 2],
    iso_value: optionalNumberFromPayload(payload, ['iso_value', 'isoValue']),
    grid_level_set: booleanFromPayload(payload, ['grid_level_set', 'gridLevelSet', 'level_set', 'levelSet'], false),
    scalar_type: stringFromPayload(payload, ['scalar_type', 'scalarType']) ?? 'float32',
    min_value: optionalNumberFromPayload(payload, ['min_value', 'minValue']),
    max_value: optionalNumberFromPayload(payload, ['max_value', 'maxValue']),
  };
}

function voxelToMeshDualRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelToMeshDualRequest {
  const baseRequest = voxelToMeshSimpleRequestFromWorkbenchPayload(payload);
  const adaptivity = optionalNumberFromPayload(payload, ['adaptivity']);
  const maxFaces = optionalNumberFromPayload(payload, ['max_faces', 'maxFaces']);
  const maxVertices = optionalNumberFromPayload(payload, ['max_vertices', 'maxVertices']);
  const relaxDisorientedTriangles = booleanFromPayload(
    payload,
    ['relax_disoriented_triangles', 'relaxDisorientedTriangles'],
    true,
  );
  const limitRequest = {
    ...baseRequest,
    adaptivity: adaptivity == null ? 0 : Math.max(0, Math.min(1, adaptivity)),
    relax_disoriented_triangles: relaxDisorientedTriangles,
    max_faces: maxFaces == null ? null : Math.max(0, Math.round(maxFaces)),
    max_vertices: maxVertices == null ? null : Math.max(0, Math.round(maxVertices)),
  };
  const modelBytesBase64 = stringFromPayload(payload, [
    'model_bytes_base64',
    'modelBytesBase64',
    'vdb_base64',
    'vdbBase64',
    'contents_base64',
    'contentsBase64',
  ]);
  if (!modelBytesBase64) {
    return limitRequest;
  }
  return {
    ...limitRequest,
    values: [],
    model_bytes_base64: modelBytesBase64,
    model_extension: stringFromPayload(payload, ['model_extension', 'modelExtension', 'extension']) ?? '.vdb',
    grid_level_set: true,
  };
}

function voxelToMeshSmartRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelToMeshSmartRequest {
  return {
    ...voxelToMeshSimpleRequestFromWorkbenchPayload(payload),
    iters: Math.max(0, Math.round(numberFromPayload(payload, ['iters', 'iterations'], 30))),
    sample_points: Math.max(1, Math.round(numberFromPayload(payload, ['sample_points', 'samplePoints'], 6))),
    degree: Math.max(3, Math.min(6, Math.round(numberFromPayload(payload, ['degree', 'polynomial_degree', 'polynomialDegree'], 3)))),
    outlier_threshold: numberFromPayload(payload, ['outlier_threshold', 'outlierThreshold'], 1),
    intermediate_smooth_force: numberFromPayload(payload, ['intermediate_smooth_force', 'intermediateSmoothForce'], 0.3),
    preparation_smooth_force: numberFromPayload(payload, ['preparation_smooth_force', 'preparationSmoothForce'], 0.1),
    smooth_shift_iterations: Math.max(0, Math.round(numberFromPayload(payload, ['smooth_shift_iterations', 'smoothShiftIterations'], 15))),
    final_relax_iterations: Math.max(0, Math.round(numberFromPayload(payload, ['final_relax_iterations', 'finalRelaxIterations'], 15))),
    final_relax_force: numberFromPayload(payload, ['final_relax_force', 'finalRelaxForce'], 0.01),
  };
}

function voxelVolumeRenderDataRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelVolumeRenderDataRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [4, 3, 2]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => index);
  const voxelSize = vectorFromPayloadKeys(payload, ['voxel_size', 'voxelSize', 'spacing']) ?? [1, 1, 1];

  return {
    values,
    shape,
    voxel_size: voxelSize,
    active_min_corner: nonnegativeIntegerTuple3FromPayload(
      payload,
      ['active_min_corner', 'activeMinCorner', 'min_corner', 'minCorner'],
      [0, 0, 0],
    ),
    active_dimensions: integerTuple3FromPayload(
      payload,
      ['active_dimensions', 'activeDimensions', 'dimensions', 'active_shape', 'activeShape'],
      shape,
    ),
    source_min_value: optionalNumberFromPayload(payload, ['source_min_value', 'sourceMinValue', 'min_value', 'minValue']),
    source_max_value: optionalNumberFromPayload(payload, ['source_max_value', 'sourceMaxValue', 'max_value', 'maxValue']),
  };
}

function voxelVolumeRenderLutRequestFromWorkbenchPayload(payload: Record<string, unknown>): VoxelVolumeRenderLutRequest {
  return {
    lut_type: stringFromPayload(payload, ['lut_type', 'lutType', 'lut']) ?? 'rainbow',
    alpha_type: stringFromPayload(payload, ['alpha_type', 'alphaType']) ?? 'constant',
    alpha_limit: Math.max(0, Math.min(255, Math.round(numberFromPayload(payload, ['alpha_limit', 'alphaLimit'], 10)))),
    one_color: tuple4FromPayload(payload, ['one_color', 'oneColor']),
  };
}

function voxelVolumeRenderRayRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    rayStart: [number, number, number];
    rayDirection: [number, number, number];
    samplingStep: number;
    alphaLimit: number;
    maxSteps: number;
  },
): VoxelVolumeRenderRayRequest {
  const shape = integerTuple3FromPayload(payload, ['shape', 'grid_shape', 'voxel_shape'], [2, 2, 2]);
  const valueCount = shape[0] * shape[1] * shape[2];
  const requestedValues = numberListFromPayload(payload, ['values', 'voxel_values', 'data']);
  const values =
    requestedValues.length >= valueCount
      ? requestedValues.slice(0, valueCount)
      : Array.from({ length: valueCount }, (_value, index) => {
          const preset = [0, 0.2, 0.4, 0.6, 0.8, 1, 0.5, 0.1];
          return preset[index % preset.length];
        });
  const voxelSize = vectorFromPayloadKeys(payload, ['voxel_size', 'voxelSize', 'spacing']) ?? [1, 1, 1];
  const rayDirection = vectorFromPayloadKeys(payload, ['ray_direction', 'rayDirection', 'direction']) ?? fallback.rayDirection;
  const rayStart = vectorFromPayloadKeys(payload, ['ray_start', 'rayStart', 'origin', 'start']) ?? fallback.rayStart;
  const minCorner = nonnegativeIntegerTuple3FromPayload(payload, ['min_corner', 'minCorner'], [0, 0, 0]);
  const activeIndices = integerListFromPayload(payload, ['active_indices', 'activeIndices'], []);

  return {
    values,
    shape,
    voxel_size: voxelSize,
    min_corner: minCorner,
    ray_start: rayStart,
    ray_direction: rayDirection,
    sampling_step: numberFromPayload(payload, ['sampling_step', 'samplingStep', 'step'], fallback.samplingStep),
    min_value: numberFromPayload(payload, ['min_value', 'minValue'], 0),
    max_value: numberFromPayload(payload, ['max_value', 'maxValue'], 1),
    lut_type: stringFromPayload(payload, ['lut_type', 'lutType', 'lut']) ?? 'rainbow',
    alpha_type: stringFromPayload(payload, ['alpha_type', 'alphaType']) ?? 'constant',
    alpha_limit: Math.max(0, Math.min(255, Math.round(numberFromPayload(payload, ['alpha_limit', 'alphaLimit'], fallback.alphaLimit)))),
    one_color: tuple4FromPayload(payload, ['one_color', 'oneColor']),
    clipping_plane: tuple4FromPayload(payload, ['clipping_plane', 'clippingPlane']),
    shading_mode: stringFromPayload(payload, ['shading_mode', 'shadingMode']) ?? 'none',
    light_pos_eye: vectorFromPayloadKeys(payload, ['light_pos_eye', 'lightPosEye']) ?? null,
    ambient_strength: numberFromPayload(payload, ['ambient_strength', 'ambientStrength'], 0.1),
    specular_strength: numberFromPayload(payload, ['specular_strength', 'specularStrength'], 0.5),
    spec_exp: numberFromPayload(payload, ['spec_exp', 'specExp'], 35),
    active_indices: activeIndices.length > 0 ? activeIndices : null,
    max_steps: Math.max(1, Math.round(numberFromPayload(payload, ['max_steps', 'maxSteps'], fallback.maxSteps))),
  };
}

function offsetMeshRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    offsetMm: number;
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
  },
): OffsetMeshRequest {
  return {
    offset_mm: numberFromPayload(payload, ['offset_mm', 'offsetMm', 'distance_mm', 'distance', 'offset'], fallback.offsetMm),
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function shellMeshRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    wallThicknessMm: number;
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
  },
): ShellMeshRequest {
  return {
    wall_thickness_mm: numberFromPayload(
      payload,
      ['wall_thickness_mm', 'wallThicknessMm', 'thickness_mm', 'thickness', 'offset_mm', 'offset'],
      fallback.wallThicknessMm,
    ),
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function thickenMeshRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    thicknessMm: number;
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
  },
): ThickenMeshRequest {
  return {
    thickness_mm: numberFromPayload(
      payload,
      ['thickness_mm', 'thicknessMm', 'thickness', 'offset_mm', 'offsetMm', 'offset', 'distance_mm', 'distanceMm', 'distance'],
      fallback.thicknessMm,
    ),
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function weightedShellRegionWeightsFromPayload(
  payload: Record<string, unknown>,
  fallbackRegionIds: string[],
  fallbackWeightMm: number,
): WeightedShellRequest['region_weights'] {
  const explicit = payload.region_weights;
  if (Array.isArray(explicit)) {
    return explicit
      .map((entry) => recordFromUnknown(entry))
      .map((entry) => ({
        region_id: stringFromPayload(entry, ['region_id', 'id', 'region']) ?? '',
        weight_mm: numberFromPayload(entry, ['weight_mm', 'weightMm', 'weight', 'offset_mm', 'offset'], fallbackWeightMm),
      }))
      .filter((entry) => entry.region_id);
  }
  if (explicit && typeof explicit === 'object') {
    return Object.entries(explicit as Record<string, unknown>)
      .map(([regionId, value]) => ({
        region_id: regionId,
        weight_mm: typeof value === 'number' && Number.isFinite(value) ? value : Number(value),
      }))
      .filter((entry) => entry.region_id && Number.isFinite(entry.weight_mm));
  }
  const selectedRegionIds = stringListFromPayload(payload, ['selected_region_ids', 'region_ids', 'regions_selected', 'regions']);
  const singleRegionId = stringFromPayload(payload, ['selected_region_id', 'region_id', 'region']);
  const regionIds = selectedRegionIds.length > 0 ? selectedRegionIds : singleRegionId ? [singleRegionId] : fallbackRegionIds;
  const weightMm = numberFromPayload(payload, ['weight_mm', 'weightMm', 'region_weight_mm', 'regionWeightMm', 'weight', 'additional_offset_mm'], fallbackWeightMm);
  return regionIds.map((regionId) => ({ region_id: regionId, weight_mm: weightMm }));
}

function weightedShellRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    offsetMm: number;
    regionWeightMm: number;
    interpolationMm: number;
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
    regionIds: string[];
  },
): WeightedShellRequest {
  return {
    offset_mm: numberFromPayload(payload, ['offset_mm', 'offsetMm', 'base_offset_mm', 'baseOffsetMm', 'offset'], fallback.offsetMm),
    region_weights: weightedShellRegionWeightsFromPayload(payload, fallback.regionIds, fallback.regionWeightMm),
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    interpolation_distance_mm: numberFromPayload(
      payload,
      ['interpolation_distance_mm', 'interpolationDistanceMm', 'interpolation_mm', 'interpolation'],
      fallback.interpolationMm,
    ),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function partialOffsetRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    offsetMm: number;
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
    regionIds: string[];
  },
): PartialOffsetRequest {
  const selectedRegionIds = stringListFromPayload(payload, ['selected_region_ids', 'region_ids', 'regions_selected', 'regions']);
  const singleRegionId = stringFromPayload(payload, ['selected_region_id', 'region_id', 'region']);
  const regionIds = selectedRegionIds.length > 0 ? selectedRegionIds : singleRegionId ? [singleRegionId] : fallback.regionIds;
  return {
    offset_mm: numberFromPayload(payload, ['offset_mm', 'offsetMm', 'offset', 'distance_mm', 'distanceMm', 'distance'], fallback.offsetMm),
    region_ids: regionIds,
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function offsetVertsRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    offsetMm: number;
    regionIds: string[];
  },
): OffsetVertsRequest {
  const selectedRegionIds = stringListFromPayload(payload, ['selected_region_ids', 'region_ids', 'regions_selected', 'regions']);
  const singleRegionId = stringFromPayload(payload, ['selected_region_id', 'region_id', 'region']);
  return {
    offset_mm: numberFromPayload(payload, ['offset_mm', 'offsetMm', 'offset', 'distance_mm', 'distanceMm', 'distance'], fallback.offsetMm),
    region_ids: selectedRegionIds.length > 0 ? selectedRegionIds : singleRegionId ? [singleRegionId] : fallback.regionIds,
  };
}

function offsetSmoothingRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    distanceMm: number;
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
  },
): OffsetSmoothingRequest {
  return {
    distance_mm: numberFromPayload(
      payload,
      ['distance_mm', 'distanceMm', 'distance', 'offset_mm', 'offsetMm', 'offset'],
      fallback.distanceMm,
    ),
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function collisionRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    otherVersionId: string;
    firstIntersectionOnly: boolean;
    maxPairs: number;
  },
): CollisionDetectRequest {
  const otherVersionId =
    stringFromPayload(payload, ['other_version_id', 'target_version_id', 'compare_version_id', 'compare_target_version_id', 'version_id']) ??
    fallback.otherVersionId;
  return {
    other_version_id: otherVersionId,
    first_intersection_only: booleanFromPayload(
      payload,
      ['first_intersection_only', 'firstIntersectionOnly', 'first_only', 'firstOnly'],
      fallback.firstIntersectionOnly,
    ),
    max_pairs: numberFromPayload(payload, ['max_pairs', 'maxPairs', 'pair_limit', 'pairLimit'], fallback.maxPairs),
    epsilon: numberFromPayload(payload, ['epsilon', 'tolerance'], 1e-8),
  };
}

function exactBooleanRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    otherVersionId: string;
    operation: ExactBooleanRequest['operation'];
  },
): ExactBooleanRequest {
  const otherVersionId =
    stringFromPayload(payload, ['other_version_id', 'target_version_id', 'compare_version_id', 'compare_target_version_id', 'version_id']) ??
    fallback.otherVersionId;
  const requestedOperation = stringFromPayload(payload, ['operation', 'boolean_operation', 'op', 'mode']);
  const operationAliases: Record<string, ExactBooleanRequest['operation']> = {
    subtract: 'difference',
    subtraction: 'difference',
    difference: 'difference',
    difference_ab: 'difference_ab',
    'a-b': 'difference_ab',
    difference_ba: 'difference_ba',
    'b-a': 'difference_ba',
    union: 'union',
    intersection: 'intersection',
    intersect: 'intersection',
    inside_a: 'inside_a',
    inside_b: 'inside_b',
    outside_a: 'outside_a',
    outside_b: 'outside_b',
  };
  const operation = requestedOperation ? operationAliases[requestedOperation.toLowerCase()] ?? fallback.operation : fallback.operation;
  return {
    other_version_id: otherVersionId,
    operation,
    epsilon: numberFromPayload(payload, ['epsilon', 'tolerance'], 1e-8),
  };
}

function voxelBooleanRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: {
    otherVersionId: string;
    operation: VoxelBooleanRequest['operation'];
    voxelSizeMm: number;
    paddingMm: number;
    refine: boolean;
  },
): VoxelBooleanRequest {
  const otherVersionId =
    stringFromPayload(payload, ['other_version_id', 'target_version_id', 'compare_version_id', 'compare_target_version_id', 'version_id']) ??
    fallback.otherVersionId;
  const requestedOperation = stringFromPayload(payload, ['operation', 'boolean_operation', 'voxel_operation', 'op', 'mode']);
  const operationAliases: Record<string, VoxelBooleanRequest['operation']> = {
    union: 'union',
    intersection: 'intersection',
    intersect: 'intersection',
    difference: 'difference',
    difference_ab: 'difference',
    subtract: 'difference',
    subtraction: 'difference',
    'a-b': 'difference',
  };
  const operation = requestedOperation ? operationAliases[requestedOperation.toLowerCase()] ?? fallback.operation : fallback.operation;
  return {
    other_version_id: otherVersionId,
    operation,
    voxel_size_mm: numberFromPayload(payload, ['voxel_size_mm', 'voxelSizeMm', 'voxel_size', 'voxelSize'], fallback.voxelSizeMm),
    padding_mm: numberFromPayload(payload, ['padding_mm', 'paddingMm', 'padding'], fallback.paddingMm),
    refine: booleanFromPayload(payload, ['refine', 'refine_surface', 'refineSurface'], fallback.refine),
  };
}

function sectionSvgFromContour(contour: SectionContourPayload): string | null {
  if (!contour.segments.length || !contour.projected_bounds_min || !contour.projected_bounds_max) {
    return null;
  }
  const u = contour.plane_u_axis;
  const v = contour.plane_v_axis;
  const [minX, minY] = contour.projected_bounds_min;
  const [maxX, maxY] = contour.projected_bounds_max;
  const projectedSegments = contour.segments.map((segment) => ({
    x1: dot(segment.start, u),
    y1: dot(segment.start, v),
    x2: dot(segment.end, u),
    y2: dot(segment.end, v),
    selectedRegionHit: segment.selected_region_hit,
  }));
  const width = Math.max(maxX - minX, 1);
  const depth = Math.max(maxY - minY, 1);
  const margin = 12;
  const svgWidth = width + margin * 2;
  const svgHeight = depth + margin * 2;
  const lines = projectedSegments.map((segment) => {
    const x1 = segment.x1 - minX + margin;
    const y1 = maxY - segment.y1 + margin;
    const x2 = segment.x2 - minX + margin;
    const y2 = maxY - segment.y2 + margin;
    const stroke = segment.selectedRegionHit ? '#f59e0b' : '#f8fafc';
    return `<line x1="${x1.toFixed(3)}" y1="${y1.toFixed(3)}" x2="${x2.toFixed(3)}" y2="${y2.toFixed(3)}" stroke="${stroke}" stroke-width="0.75" />`;
  });
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${svgWidth.toFixed(2)}mm" height="${svgHeight.toFixed(2)}mm" viewBox="0 0 ${svgWidth.toFixed(3)} ${svgHeight.toFixed(3)}">`,
    '<rect width="100%" height="100%" fill="#09090b" />',
    ...lines,
    `<line x1="${margin}" y1="${svgHeight - margin}" x2="${svgWidth - margin}" y2="${svgHeight - margin}" stroke="#22c55e" stroke-width="0.5" />`,
    `<line x1="${svgWidth - margin}" y1="${margin}" x2="${svgWidth - margin}" y2="${svgHeight - margin}" stroke="#38bdf8" stroke-width="0.5" />`,
    `<text x="${margin}" y="${margin - 4}" fill="#f8fafc" font-size="4">Offset=${contour.section_constant.toFixed(2)}mm</text>`,
    `<text x="${margin}" y="${svgHeight - 2}" fill="#22c55e" font-size="4">W=${(contour.width_mm ?? 0).toFixed(2)}mm</text>`,
    `<text x="${svgWidth - margin + 2}" y="${margin + 6}" fill="#38bdf8" font-size="4">D=${(contour.depth_mm ?? 0).toFixed(2)}mm</text>`,
    '</svg>',
  ].join('');
}

function downloadSectionContourSvg(contour: SectionContourPayload, sourceVersionId: string) {
  const svg = sectionSvgFromContour(contour);
  if (!svg) {
    return;
  }
  const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `${sourceVersionId}-offset${contour.section_constant.toFixed(1)}.svg`;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
}

function sourceVersionIdFromWorkbenchPayload(payload: Record<string, unknown>, fallback: string): string {
  return stringFromPayload(payload, [
    'source_version_id',
    'restore_version_id',
    'branch_version_id',
    'target_version_id',
    'open_version_id',
    'history_version_id',
    'version_id',
  ]) ?? fallback;
}

function versionHistoryVersionIdFromWorkbenchPayload(payload: Record<string, unknown>): string | null {
  return stringFromPayload(payload, ['open_version_id', 'history_version_id', 'target_version_id', 'version_id']);
}

function compareVersionIdFromWorkbenchPayload(payload: Record<string, unknown>): string | null {
  return stringFromPayload(payload, ['other_version_id', 'compare_version_id', 'compare_target_version_id', 'target_version_id', 'version_id']);
}

function shouldDisableCompareFromWorkbenchPayload(payload: Record<string, unknown>): boolean {
  const action = stringFromPayload(payload, ['compare_action', 'action', 'mode', 'operation']);
  if (action && ['disable', 'off', 'close', 'clear', 'reset'].includes(action.toLowerCase())) {
    return true;
  }
  return booleanFromPayload(payload, ['enabled', 'compare_enabled', 'show'], true) === false;
}

function downloadUrlFromWorkbenchInvocation(invocation: WorkbenchCommandInvocation | undefined, fallback: string | null): string | null {
  if (invocation?.endpointUrl) {
    return invocation.endpointUrl;
  }
  const payload = requestPayloadFromWorkbenchCommand(invocation);
  return stringFromPayload(payload, ['artifact_url', 'download_url', 'url']) ?? fallback;
}

function jobIdFromWorkbenchPayload(payload: Record<string, unknown>): string | null {
  return stringFromPayload(payload, ['job_id', 'active_job_id', 'id']);
}

function shouldExecuteWorkbenchCommand(invocation?: WorkbenchCommandInvocation): boolean {
  if (!invocation) {
    return false;
  }
  const optionExecuteFlag = explicitBooleanFromPayload(invocation.options, ['execute', 'auto_execute', 'submit']);
  const payloadExecuteFlag = explicitBooleanFromPayload(invocation.payload, ['execute', 'auto_execute', 'submit']);
  if (optionExecuteFlag === false || payloadExecuteFlag === false) {
    return false;
  }
  if (optionExecuteFlag === true) {
    return true;
  }
  if (payloadExecuteFlag === true) {
    return true;
  }
  if ('request' in invocation.payload || 'params' in invocation.payload) {
    return true;
  }
  const requestPayload = requestPayloadFromWorkbenchCommand(invocation);
  return Object.keys(requestPayload).some((key) => !['label', 'metadata', 'operation_label'].includes(key));
}

function resizeRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackSize: number,
  axisMode: 'auto' | 'manual',
  manualAxis: [number, number, number] | null,
): ResizeRequestV2 {
  const requestedAxisMode = stringFromPayload(payload, ['axis_mode']) === 'manual' ? 'manual' : axisMode;
  const axis = vectorFromPayload(payload, 'manual_axis') ?? manualAxis ?? undefined;
  return {
    target_ring_size_us: numberFromPayload(payload, ['target_ring_size_us', 'target_size_us', 'ring_size_us', 'ring_size'], fallbackSize),
    axis_mode: requestedAxisMode === 'manual' && axis ? 'manual' : 'auto',
    manual_axis: requestedAxisMode === 'manual' ? axis : undefined,
    preserve_head: booleanFromPayload(payload, ['preserve_head', 'preserve_detail'], true),
  };
}

function hollowRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackMaterial: MaterialType,
  fallbackWallThickness: number,
  fallbackTargetWeight: number,
  fallbackMinThickness: number,
  addDrainHoles: boolean,
): HollowRequestV2 {
  const targetWeight = numberFromPayload(payload, ['target_weight_g', 'target_weight'], fallbackTargetWeight);
  const mode = stringFromPayload(payload, ['mode']) === 'target_weight' || 'target_weight_g' in payload
    ? 'target_weight'
    : 'fixed_thickness';
  return {
    mode,
    processing_mode: hollowProcessingModeFromPayload(payload),
    material: materialFromPayload(payload, fallbackMaterial),
    wall_thickness_mm: numberFromPayload(payload, ['wall_thickness_mm', 'wall_thickness', 'shell_thickness_mm'], fallbackWallThickness),
    target_weight_g: mode === 'target_weight' ? targetWeight : undefined,
    min_allowed_thickness_mm: numberFromPayload(payload, ['min_allowed_thickness_mm', 'min_thickness_mm'], fallbackMinThickness),
    protect_regions: protectRegionsFromPayload(payload),
    add_drain_holes: booleanFromPayload(payload, ['add_drain_holes', 'drain_holes'], addDrainHoles),
  };
}

function thickenRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackMode: ThickenRequestV2['mode'],
  fallbackThickness: number,
  fallbackRegionId: string | null,
  fallbackRegionIds: string[],
): ThickenRequestV2 {
  const regionIds = stringListFromPayload(payload, ['region_ids', 'regions', 'selected_region_ids']);
  const regionId = stringFromPayload(payload, ['region_id', 'region']) ?? fallbackRegionId ?? undefined;
  const mode = stringFromPayload(payload, ['mode']) as ThickenRequestV2['mode'] | null;
  return {
    mode: mode ?? (regionIds.length > 0 ? 'selected_regions' : regionId ? 'selected_region' : fallbackMode),
    min_target_thickness_mm: numberFromPayload(payload, ['min_target_thickness_mm', 'target_thickness_mm', 'thickness_mm'], fallbackThickness),
    region_id: regionId,
    region_ids: regionIds.length > 0 ? regionIds : fallbackRegionIds,
    smoothing_pass: booleanFromPayload(payload, ['smoothing_pass', 'smooth'], true),
  };
}

function smoothRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackIterations: number,
  fallbackStrength: number,
  fallbackRegionId: string | null,
  fallbackRegionIds: string[],
): SmoothRequestV2 {
  const regionIds = stringListFromPayload(payload, ['region_ids', 'regions', 'selected_region_ids']);
  const regionId = stringFromPayload(payload, ['region_id', 'region']) ?? fallbackRegionId;
  return {
    region_id: regionIds.length > 0 ? null : regionId,
    region_ids: regionIds.length > 0 ? regionIds : fallbackRegionIds,
    iterations: Math.max(1, Math.round(numberFromPayload(payload, ['iterations', 'passes'], fallbackIterations))),
    strength: Math.min(1, Math.max(0.01, numberFromPayload(payload, ['strength'], fallbackStrength))),
    global_mode: booleanFromPayload(payload, ['global_mode', 'global'], !regionId && regionIds.length === 0),
  };
}

function subdivideRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackMaxEdgeLen: number,
  fallbackMaxEdgeSplits: number,
  fallbackSubdivideBorder: boolean,
  fallbackCurvaturePriority: number,
  fallbackProjectOnOriginalMesh: boolean,
  fallbackSmoothMode: boolean,
  fallbackMinSharpDihedralAngle: number,
  fallbackMaxTriAspectRatio: number,
  fallbackMaxSplittableTriAspectRatio: number,
  fallbackMaxDeviationAfterFlip: number,
  fallbackMaxAngleChangeAfterFlip: number,
  fallbackCriticalTriAspectRatioFlip: number,
  fallbackRegionFaces: string,
  fallbackNotFlippableEdges: string,
): SubdivideRequestV2 {
  const explicitSplittableAspect = numberFromPayload(
    payload,
    ['max_splittable_tri_aspect_ratio', 'max_splittable_aspect_ratio', 'splittable_tri_aspect_ratio'],
    fallbackMaxSplittableTriAspectRatio,
  );
  const maxDeviationAfterFlip = numberFromPayload(
    payload,
    ['max_deviation_after_flip', 'maxDeviationAfterFlip', 'maxDeviation', 'deviation'],
    fallbackMaxDeviationAfterFlip,
  );
  const maxAngleChangeAfterFlip = numberFromPayload(
    payload,
    ['max_angle_change_after_flip', 'maxAngleChangeAfterFlip', 'max_angle_change', 'maxAngleChange'],
    fallbackMaxAngleChangeAfterFlip,
  );
  const criticalTriAspectRatioFlip = numberFromPayload(
    payload,
    ['critical_tri_aspect_ratio_flip', 'criticalAspectRatioFlip', 'critical_tri_aspect_ratio', 'criticalTriAspectRatio'],
    fallbackCriticalTriAspectRatioFlip,
  );
  const minSharpAngleDegrees = numberFromPayload(payload, ['min_sharp_dihedral_angle_degrees', 'min_sharp_angle_degrees'], Number.NaN);
  const minSharpAngle = Number.isFinite(minSharpAngleDegrees)
    ? minSharpAngleDegrees * Math.PI / 180
    : numberFromPayload(
        payload,
        ['min_sharp_dihedral_angle', 'min_sharp_angle', 'sharp_angle'],
        fallbackMinSharpDihedralAngle,
      );
  return {
    max_edge_len: numberFromPayload(payload, ['max_edge_len', 'max_edge_length', 'edge_length_mm'], fallbackMaxEdgeLen),
    max_edge_splits: Math.max(1, Math.round(numberFromPayload(payload, ['max_edge_splits', 'max_splits'], fallbackMaxEdgeSplits))),
    subdivide_border: booleanFromPayload(payload, ['subdivide_border', 'include_border', 'border'], fallbackSubdivideBorder),
    curvature_priority: numberFromPayload(payload, ['curvature_priority'], fallbackCurvaturePriority),
    project_on_original_mesh: booleanFromPayload(
      payload,
      ['project_on_original_mesh', 'projectOnOriginalMesh', 'project_original'],
      fallbackProjectOnOriginalMesh,
    ),
    smooth_mode: booleanFromPayload(payload, ['smooth_mode', 'smoothMode'], fallbackSmoothMode),
    min_sharp_dihedral_angle: minSharpAngle,
    max_tri_aspect_ratio: numberFromPayload(payload, ['max_tri_aspect_ratio', 'max_triangle_aspect_ratio'], fallbackMaxTriAspectRatio),
    max_splittable_tri_aspect_ratio: explicitSplittableAspect > 0 ? explicitSplittableAspect : null,
    max_deviation_after_flip: maxDeviationAfterFlip > 0 ? maxDeviationAfterFlip : null,
    max_angle_change_after_flip: maxAngleChangeAfterFlip > 0 ? maxAngleChangeAfterFlip : null,
    critical_tri_aspect_ratio_flip: criticalTriAspectRatioFlip > 0 ? criticalTriAspectRatioFlip : null,
    region_faces: integerListFromPayload(
      payload,
      ['region_faces', 'regionFaces', 'face_region', 'faceRegion'],
      parseIntegerList(fallbackRegionFaces),
    ),
    not_flippable_edges: edgePairsFromPayload(
      payload,
      ['not_flippable_edges', 'notFlippableEdges', 'protected_edges', 'protectedEdges'],
      parseEdgePairString(fallbackNotFlippableEdges),
    ),
  };
}

function makeDeloneRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackNumIters: number,
  fallbackMaxDeviationAfterFlip: number,
  fallbackMaxAngleChange: number,
  fallbackCriticalTriAspectRatio: number,
  fallbackRegionFaces: string,
  fallbackNotFlippableEdges: string,
  fallbackVertRegion: string,
): MakeDeloneRequestV2 {
  const maxDeviationAfterFlip = numberFromPayload(
    payload,
    ['max_deviation_after_flip', 'maxDeviationAfterFlip', 'maxDeviation', 'deviation'],
    fallbackMaxDeviationAfterFlip,
  );
  const maxAngleChange = numberFromPayload(
    payload,
    ['max_angle_change', 'maxAngleChange', 'angle_change', 'angleChange'],
    fallbackMaxAngleChange,
  );
  const criticalTriAspectRatio = numberFromPayload(
    payload,
    ['critical_tri_aspect_ratio', 'criticalTriAspectRatio', 'critical_aspect_ratio', 'criticalAspectRatio'],
    fallbackCriticalTriAspectRatio,
  );
  return {
    num_iters: Math.max(1, Math.round(numberFromPayload(payload, ['num_iters', 'numIters', 'iterations'], fallbackNumIters))),
    max_deviation_after_flip: maxDeviationAfterFlip > 0 ? maxDeviationAfterFlip : null,
    max_angle_change: maxAngleChange > 0 ? maxAngleChange : null,
    critical_tri_aspect_ratio: criticalTriAspectRatio > 0 ? criticalTriAspectRatio : null,
    region_faces: integerListFromPayload(
      payload,
      ['region_faces', 'regionFaces', 'face_region', 'faceRegion'],
      parseIntegerList(fallbackRegionFaces),
    ),
    not_flippable_edges: edgePairsFromPayload(
      payload,
      ['not_flippable_edges', 'notFlippableEdges', 'protected_edges', 'protectedEdges'],
      parseEdgePairString(fallbackNotFlippableEdges),
    ),
    vert_region: integerListFromPayload(
      payload,
      ['vert_region', 'vertRegion', 'vertex_region', 'vertexRegion', 'active_vertices', 'activeVertices'],
      parseIntegerList(fallbackVertRegion),
    ),
    metadata: recordFromUnknown(payload.metadata),
  };
}

function decimateRequestFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallbackStrategy: DecimateRequestV2['strategy'],
  fallbackMaxError: number,
  fallbackTargetFaces: number,
  fallbackTargetPercent: number,
  fallbackMaxEdgeLen: number,
  fallbackMaxBoundaryShift: number,
  fallbackStabilizer: number,
  fallbackParallelAlgorithm: boolean,
  fallbackSubdivideParts: number,
  fallbackRegionFaces: string,
  fallbackNotFlippableEdges: string,
  fallbackCollapseNearNotFlippable: boolean,
  fallbackAngleWeightedDistToPlane: boolean,
  fallbackMaxDeletedVertices: number,
  fallbackMaxDeletedFaces: number,
  fallbackMaxTriangleAspectRatio: number,
  fallbackTouchNearBoundaryEdges: boolean,
  fallbackTouchBoundaryVerts: boolean,
  fallbackOptimizeVertexPos: boolean,
  fallbackPackMesh: boolean,
): DecimateRequestV2 {
  const maxEdgeLen = numberFromPayload(payload, ['max_edge_len', 'max_edge_length', 'maxEdgeLen'], fallbackMaxEdgeLen);
  const maxBoundaryShift = numberFromPayload(
    payload,
    ['max_bd_shift', 'maxBdShift', 'max_boundary_shift', 'maxBoundaryShift'],
    fallbackMaxBoundaryShift,
  );
  const targetFaceKeys = ['target_face_count', 'targetFaceCount', 'target_triangles', 'targetTriangles', 'target_faces', 'targetFaces'];
  const targetRatioKeys = ['target_face_ratio', 'targetFaceRatio', 'target_percentage', 'targetPercentage', 'target_percent', 'targetPercent'];
  const hasExplicitTargetFaces = hasAnyPayloadKey(payload, targetFaceKeys);
  const hasExplicitTargetRatio = hasAnyPayloadKey(payload, targetRatioKeys);
  const rawTargetFaces = numberFromPayload(payload, targetFaceKeys, hasExplicitTargetRatio ? 0 : fallbackTargetFaces);
  const rawTargetRatio = numberFromPayload(
    payload,
    targetRatioKeys,
    hasExplicitTargetFaces ? 0 : fallbackTargetPercent > 0 ? fallbackTargetPercent / 100 : 0,
  );
  const targetFaceRatio = rawTargetRatio > 0 ? Math.min(1, rawTargetRatio > 1 ? rawTargetRatio / 100 : rawTargetRatio) : null;
  const rawStrategy = stringFromPayload(payload, ['strategy', 'decimate_strategy', 'decimateStrategy']);
  const normalizedStrategy = rawStrategy?.replace(/^.*\./, '').replace(/[-\s]/g, '_').toLowerCase();
  const strategy: DecimateRequestV2['strategy'] =
    normalizedStrategy === 'shortest_edge_first' || normalizedStrategy === 'shortestedgefirst'
      ? 'shortest_edge_first'
      : normalizedStrategy === 'minimize_error' || normalizedStrategy === 'minimizeerror'
        ? 'minimize_error'
        : fallbackStrategy;
  const parallelAlgorithm = booleanFromPayload(
    payload,
    ['parallel_algorithm', 'parallelAlgorithm', 'parallel', 'use_parallel', 'useParallel'],
    fallbackParallelAlgorithm,
  );
  const rawSubdivideParts = numberFromPayload(
    payload,
    ['subdivide_parts', 'subdivideParts', 'parallel_parts', 'parallelParts'],
    fallbackSubdivideParts,
  );
  const subdivideParts = parallelAlgorithm
    ? Math.max(2, Math.round(rawSubdivideParts))
    : Math.max(1, Math.round(rawSubdivideParts));
  const requestedMaxError = numberFromPayload(
    payload,
    ['max_error', 'maxError', 'shortest_edge_limit', 'edge_length_mm'],
    fallbackMaxError,
  );
  // Jewelry-scale safety: with NO explicit face target, an implausibly large error
  // tolerance (the workbench demo payload sends max_error≈1000mm) lets QEM collapse a
  // ~10mm model down to a degenerate blob (observed: 20000→38 faces). When there is no
  // target and the tolerance is larger than any real jewelry model (>100mm), fall back
  // to a sane 50% face-ratio target so decimation stops sensibly. Explicit targets and
  // reasonable max_error values are left untouched.
  const hasFaceTarget = rawTargetFaces > 0 || targetFaceRatio !== null;
  const degenerateNoTarget = !hasFaceTarget && (!(requestedMaxError > 0) || requestedMaxError > 100);
  const effectiveTargetRatio = degenerateNoTarget ? 0.5 : targetFaceRatio;
  return {
    strategy,
    max_error: requestedMaxError,
    target_face_count: rawTargetFaces > 0 ? Math.max(1, Math.round(rawTargetFaces)) : null,
    target_face_ratio: effectiveTargetRatio,
    max_edge_len: maxEdgeLen > 0 ? maxEdgeLen : null,
    max_bd_shift: maxBoundaryShift > 0 ? maxBoundaryShift : null,
    stabilizer: Math.max(0, numberFromPayload(payload, ['stabilizer', 'qemStabilizer'], fallbackStabilizer)),
    subdivide_parts: subdivideParts,
    decimate_between_parts: booleanFromPayload(
      payload,
      ['decimate_between_parts', 'decimateBetweenParts', 'between_parts', 'betweenParts'],
      true,
    ),
    region_faces: integerListFromPayload(payload, ['region_faces', 'regionFaces', 'face_region', 'faceRegion'], parseIntegerList(fallbackRegionFaces)),
    not_flippable_edges: edgePairsFromPayload(
      payload,
      ['not_flippable_edges', 'notFlippableEdges', 'protected_edges', 'protectedEdges'],
      parseEdgePairString(fallbackNotFlippableEdges),
    ),
    collapse_near_not_flippable: booleanFromPayload(
      payload,
      ['collapse_near_not_flippable', 'collapseNearNotFlippable', 'collapse_near_protected', 'collapseNearProtected'],
      fallbackCollapseNearNotFlippable,
    ),
    angle_weighted_dist_to_plane: booleanFromPayload(
      payload,
      ['angle_weighted_dist_to_plane', 'angleWeightedDistToPlane', 'angle_weighted_planes', 'angleWeightedPlanes'],
      fallbackAngleWeightedDistToPlane,
    ),
    max_deleted_vertices: Math.max(1, Math.round(numberFromPayload(payload, ['max_deleted_vertices', 'maxDeletedVertices'], fallbackMaxDeletedVertices))),
    max_deleted_faces: Math.max(1, Math.round(numberFromPayload(payload, ['max_deleted_faces', 'maxDeletedFaces'], fallbackMaxDeletedFaces))),
    max_triangle_aspect_ratio: Math.max(
      1,
      numberFromPayload(
        payload,
        ['max_triangle_aspect_ratio', 'maxTriangleAspectRatio', 'max_tri_aspect_ratio'],
        fallbackMaxTriangleAspectRatio,
      ),
    ),
    touch_near_bd_edges: booleanFromPayload(
      payload,
      ['touch_near_bd_edges', 'touchNearBdEdges', 'touch_boundary_edges', 'touchBoundaryEdges'],
      fallbackTouchNearBoundaryEdges,
    ),
    touch_bd_verts: booleanFromPayload(
      payload,
      ['touch_bd_verts', 'touchBdVerts', 'touch_boundary_verts', 'touchBoundaryVerts'],
      fallbackTouchBoundaryVerts,
    ),
    optimize_vertex_pos: booleanFromPayload(payload, ['optimize_vertex_pos', 'optimizeVertexPos'], fallbackOptimizeVertexPos),
    pack_mesh: booleanFromPayload(payload, ['pack_mesh', 'packMesh'], fallbackPackMesh),
    metadata: recordFromUnknown(payload.metadata),
  };
}

function normalizeAxis(axis: [number, number, number] | null | undefined): [number, number, number] {
  if (!axis) return [0, 1, 0];
  const length = Math.hypot(axis[0], axis[1], axis[2]);
  if (length < 1e-8) return [0, 1, 0];
  return [axis[0] / length, axis[1] / length, axis[2] / length];
}

function dot(point: [number, number, number], axis: [number, number, number]) {
  return point[0] * axis[0] + point[1] * axis[1] + point[2] * axis[2];
}

function inspectionSnapshotStateFromWorkbenchPayload(
  payload: Record<string, unknown>,
  fallback: InspectionSnapshotState,
): InspectionSnapshotState {
  const manualAxis = vectorFromPayloadKeys(payload, ['manual_axis', 'plane_axis', 'section_axis', 'axis']);
  const axisMode = stringFromPayload(payload, ['axis_mode']);
  const selectedRegionIds = stringListFromPayload(payload, ['selected_region_ids', 'region_ids', 'regions_selected', 'regions']);
  return {
    name: stringFromPayload(payload, ['snapshot_name', 'name', 'label']) ?? fallback.name,
    axis_mode: axisMode === 'manual' || manualAxis ? 'manual' : axisMode === 'auto' ? 'auto' : fallback.axis_mode,
    manual_axis: manualAxis ? normalizeAxis(manualAxis) : fallback.manual_axis,
    section_enabled: booleanFromPayload(payload, ['section_enabled', 'enabled', 'show_section'], fallback.section_enabled),
    section_constant: sectionPlaneConstantFromWorkbenchPayload(
      payload,
      fallback.section_constant,
      fallback.manual_axis ?? [0, 1, 0],
    ),
    selected_region_id: stringFromPayload(payload, ['selected_region_id', 'region_id', 'region']) ?? fallback.selected_region_id,
    selected_region_ids: selectedRegionIds.length > 0 ? selectedRegionIds : fallback.selected_region_ids,
    heatmap_enabled: booleanFromPayload(payload, ['heatmap_enabled', 'show_heatmap'], fallback.heatmap_enabled),
    compare_enabled: booleanFromPayload(payload, ['compare_enabled', 'show_compare'], fallback.compare_enabled),
    compare_target_version_id:
      stringFromPayload(payload, ['compare_target_version_id', 'compare_version_id', 'other_version_id']) ?? fallback.compare_target_version_id,
  };
}

function findInspectionSnapshotForWorkbenchPayload(
  payload: Record<string, unknown>,
  snapshots: InspectionSnapshotResponse[],
): InspectionSnapshotResponse | null {
  const snapshotId = stringFromPayload(payload, ['snapshot_id', 'inspection_snapshot_id', 'id']);
  if (snapshotId) {
    const byId = snapshots.find((snapshot) => snapshot.id === snapshotId);
    if (byId) {
      return byId;
    }
  }
  const snapshotName = stringFromPayload(payload, ['snapshot_name', 'name', 'label']);
  if (snapshotName) {
    return snapshots.find((snapshot) => snapshot.name === snapshotName) ?? null;
  }
  return null;
}

function ViewerPageContent() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const queryClient = useQueryClient();
  const modelId = searchParams.get('model');
  const urlVersionId = searchParams.get('version');
  const urlJobId = searchParams.get('job');

  const [versionId, setVersionId] = useState<string | null>(urlVersionId);
  const [activeJobId, setActiveJobId] = useState<string | null>(urlJobId);
  const [resizeAxisMode, setResizeAxisMode] = useState<'auto' | 'manual'>('auto');
  const [manualResizeAxis, setManualResizeAxis] = useState<[number, number, number] | null>(null);
  const [measureInspectResult, setMeasureInspectResult] = useState<MeasureInspectResponse | null>(null);
  const [gcodeParseResult, setGcodeParseResult] = useState<GcodeParsePathsResponse | null>(null);
  const [pointCloudIcpResult, setPointCloudIcpResult] = useState<PointCloudIcpResponse | null>(null);
  const [offsetContoursResult, setOffsetContoursResult] = useState<OffsetContoursResponse | null>(null);
  const [distanceMapFromMeshResult, setDistanceMapFromMeshResult] = useState<DistanceMapResponse | null>(null);
  const [distanceMapContoursResult, setDistanceMapContoursResult] = useState<DistanceMapResponse | null>(null);
  const [distanceMapIsoLinesResult, setDistanceMapIsoLinesResult] = useState<IsoLineSegmentsResponse | null>(null);
  const [distanceMapMergeResult, setDistanceMapMergeResult] = useState<DistanceMapResponse | null>(null);
  const [distanceMapContourBooleanResult, setDistanceMapContourBooleanResult] = useState<IsoLineSegmentsResponse | null>(null);
  const [distanceMapTiffImportResult, setDistanceMapTiffImportResult] = useState<DistanceMapResponse | null>(null);
  const [distanceMapTiffExportResult, setDistanceMapTiffExportResult] = useState<DistanceMapTiffExportResponse | null>(null);
  const [objectLinesResult, setObjectLinesResult] = useState<ObjectLinesResponse | null>(null);
  const [objectLinesContoursResult, setObjectLinesContoursResult] = useState<ObjectLinesToContoursResponse | null>(null);
  const [objectLinesPtsExportResult, setObjectLinesPtsExportResult] = useState<ObjectLinesTextExportResponse | null>(null);
  const [objectLinesDxfExportResult, setObjectLinesDxfExportResult] = useState<ObjectLinesTextExportResponse | null>(null);
  const [objectLinesMrLinesExportResult, setObjectLinesMrLinesExportResult] = useState<ObjectLinesBinaryExportResponse | null>(null);
  const [objectLinesPlyExportResult, setObjectLinesPlyExportResult] = useState<ObjectLinesBinaryExportResponse | null>(null);
  const [meshToVoxelsResult, setMeshToVoxelsResult] = useState<MeshToVoxelsSdfResponse | null>(null);
  const [voxelLoadResult, setVoxelLoadResult] = useState<VoxelVolumeLoadResponse | null>(null);
  const [voxelVolumeRenderRayResult, setVoxelVolumeRenderRayResult] = useState<VoxelVolumeRenderRayResponse | null>(null);
  const [offsetShellResult, setOffsetShellResult] = useState<OffsetShellMeshResponse | null>(null);
  const [exactBooleanResult, setExactBooleanResult] = useState<ExactBooleanResponse | null>(null);
  const [voxelBooleanResult, setVoxelBooleanResult] = useState<VoxelBooleanResponse | null>(null);
  const [collisionResult, setCollisionResult] = useState<CollisionDetectResponse | null>(null);
  const [urlStateReady, setUrlStateReady] = useState(false);
  const urlSyncRef = useRef<string | null>(null);

  const wireframe = useEditorStore((state) => state.wireframe);
  const sectionEnabled = useEditorStore((state) => state.sectionEnabled);
  const sectionConstant = useEditorStore((state) => state.sectionConstant);
  const heatmapEnabled = useEditorStore((state) => state.heatmapEnabled);
  const regionOverlayEnabled = useEditorStore((state) => state.regionOverlayEnabled);
  const selectedRegionId = useEditorStore((state) => state.selectedRegionId);
  const selectedRegionIds = useEditorStore((state) => state.selectedRegionIds);
  const compareOverlayEnabled = useEditorStore((state) => state.compareOverlayEnabled);
  const compareTargetVersionId = useEditorStore((state) => state.compareTargetVersionId);
  const selectedMaterial = useEditorStore((state) => state.selectedMaterial);
  const activeToolbarGroup = useEditorStore((state) => state.activeToolbarGroup);
  const openPopoverGroup = useEditorStore((state) => state.openPopoverGroup);
  const activeTool = useEditorStore((state) => state.activeTool);
  const rightDockTab = useEditorStore((state) => state.rightDockTab);
  const reviewPane = useEditorStore((state) => state.reviewPane);
  const toolDrafts = useEditorStore((state) => state.toolDrafts);
  const setWireframe = useEditorStore((state) => state.setWireframe);
  const setSectionEnabled = useEditorStore((state) => state.setSectionEnabled);
  const setSectionConstant = useEditorStore((state) => state.setSectionConstant);
  const setHeatmapEnabled = useEditorStore((state) => state.setHeatmapEnabled);
  const setRegionOverlayEnabled = useEditorStore((state) => state.setRegionOverlayEnabled);
  const setSelectedRegionId = useEditorStore((state) => state.setSelectedRegionId);
  const setSelectedRegionIds = useEditorStore((state) => state.setSelectedRegionIds);
  const toggleSelectedRegionId = useEditorStore((state) => state.toggleSelectedRegionId);
  const setCompareOverlayEnabled = useEditorStore((state) => state.setCompareOverlayEnabled);
  const setCompareTargetVersionId = useEditorStore((state) => state.setCompareTargetVersionId);
  const setSelectedMaterial = useEditorStore((state) => state.setSelectedMaterial);
  const setActiveToolbarGroup = useEditorStore((state) => state.setActiveToolbarGroup);
  const setOpenPopoverGroup = useEditorStore((state) => state.setOpenPopoverGroup);
  const setActiveTool = useEditorStore((state) => state.setActiveTool);
  const setRightDockTab = useEditorStore((state) => state.setRightDockTab);
  const setReviewPane = useEditorStore((state) => state.setReviewPane);
  const updateToolDrafts = useEditorStore((state) => state.updateToolDrafts);
  const resetWorkspaceState = useEditorStore((state) => state.resetWorkspaceState);

  const versionsQuery = useModelVersions(modelId);
  const versionDetailQuery = useVersion(versionId);
  const versionArtifactsReady = versionDetailQuery.data?.version.status === 'ready';
  const compareCacheQuery = useCompareCache(versionId);
  const inspectionSnapshotsQuery = useInspectionSnapshots(versionId);
  const viewerQuery = useViewerManifest(versionId, versionArtifactsReady);
  const workbenchManifestQuery = useMeshLibWorkbenchManifest(versionId, versionArtifactsReady);
  const versionJobsQuery = useVersionJobs(versionId);
  const snapshotQuery = useManufacturability(versionId, versionArtifactsReady);
  const thicknessOverlayQuery = useThicknessOverlay(versionId, heatmapEnabled && !compareOverlayEnabled);
  const compareCacheTargets = useMemo(
    () => new Set((compareCacheQuery.data ?? []).map((entry) => entry.other_version_id)),
    [compareCacheQuery.data],
  );
  const compareOverlayReady =
    compareOverlayEnabled &&
    !!compareTargetVersionId &&
    (compareTargetVersionId ? compareCacheTargets.has(compareTargetVersionId) : false);
  const compareOverlayQuery = useCompareOverlay(versionId, compareTargetVersionId, compareOverlayReady);
  const compareSummaryQuery = useCompareSummary(
    versionId,
    compareTargetVersionId,
    !!compareTargetVersionId && compareCacheTargets.has(compareTargetVersionId),
  );
  const repairMutation = useRepairOperation();
  const resizeMutation = useResizeOperation();
  const hollowMutation = useHollowOperation();
  const thickenMutation = useThickenOperation();
  const brushReplayMutation = useBrushReplayOperation();
  const compareMutation = useCompareOperation();
  const exactBooleanMutation = useExactBooleanOperation();
  const voxelBooleanMutation = useVoxelBooleanOperation();
  const collisionMutation = useCollisionDetectOperation();
  const scoopMutation = useScoopOperation();
  const smoothMutation = useSmoothOperation();
  const decimateMutation = useDecimateOperation();
  const subdivideMutation = useSubdivideOperation();
  const makeDeloneMutation = useMakeDeloneOperation();
  const measureInspectMutation = useMeasureInspectOperation();
  const meshCutMeasureTopologyMutation = useMeshCutMeasureTopologyOperation();
  const gcodeParseMutation = useGcodeParsePathsOperation();
  const gcodeLoadSourceMutation = useGcodeLoadSourceOperation();
  const gcodeWriteSourceMutation = useGcodeWriteSourceOperation();
  const gcodeParseFilePathsMutation = useGcodeParseFilePathsOperation();
  const pointCloudIcpMutation = usePointCloudIcpOperation();
  const offsetContoursMutation = useOffsetContoursOperation();
  const distanceMapFromMeshMutation = useDistanceMapFromMeshOperation();
  const distanceMapContoursMutation = useDistanceMapContoursOperation();
  const distanceMapIsoLinesMutation = useDistanceMapIsoLinesOperation();
  const distanceMapMergeMutation = useDistanceMapMergeOperation();
  const distanceMapContourBooleanMutation = useDistanceMapContourBooleanOperation();
  const distanceMapFromTiffMutation = useDistanceMapFromTiffOperation();
  const distanceMapToTiffMutation = useDistanceMapToTiffOperation();
  const objectLinesFromContoursMutation = useObjectLinesFromContoursOperation();
  const objectLinesLoadMrLinesMutation = useObjectLinesLoadMrLinesOperation();
  const objectLinesLoadPlyMutation = useObjectLinesLoadPlyOperation();
  const objectLinesLoadPtsMutation = useObjectLinesLoadPtsOperation();
  const objectLinesLoadSvgMutation = useObjectLinesLoadSvgOperation();
  const objectLinesSaveDxfMutation = useObjectLinesSaveDxfOperation();
  const objectLinesSaveMrLinesMutation = useObjectLinesSaveMrLinesOperation();
  const objectLinesSavePlyMutation = useObjectLinesSavePlyOperation();
  const objectLinesSavePtsMutation = useObjectLinesSavePtsOperation();
  const objectLinesToContoursMutation = useObjectLinesToContoursOperation();
  const meshToVoxelsMutation = useMeshToVoxelsSdfOperation();
  const openRawVoxelsMutation = useOpenRawVoxelsOperation();
  const openVoxelsFromTiffMutation = useOpenVoxelsFromTiffOperation();
  const voxelBinaryOperationsMutation = useVoxelBinaryOperationsOperation();
  const voxelLineGraphMutation = useVoxelLineGraphOperation();
  const voxelActiveBoxMutation = useVoxelActiveBoxOperation();
  const voxelSliceMutation = useVoxelSliceOperation();
  const voxelPathMutation = useVoxelPathOperation();
  const voxelPathBuildFourMutation = useVoxelPathBuildFourOperation();
  const voxelSegmentationMutation = useVoxelSegmentationOperation();
  const voxelMaskToMeshMutation = useVoxelMaskToMeshOperation();
  const voxelToMeshSimpleMutation = useVoxelToMeshSimpleOperation();
  const voxelToMeshDualMutation = useVoxelToMeshDualOperation();
  const voxelToMeshSmartMutation = useVoxelToMeshSmartOperation();
  const voxelVolumeRenderDataMutation = useVoxelVolumeRenderDataOperation();
  const voxelVolumeRenderLutMutation = useVoxelVolumeRenderLutOperation();
  const voxelVolumeRenderRayMutation = useVoxelVolumeRenderRayOperation();
  const offsetMeshMutation = useOffsetMeshOperation();
  const shellMeshMutation = useShellMeshOperation();
  const thickenMeshMutation = useThickenMeshOperation();
  const weightedShellMutation = useWeightedShellOperation();
  const partialOffsetMutation = usePartialOffsetOperation();
  const offsetVertsMutation = useOffsetVertsOperation();
  const expandShrinkMutation = useExpandShrinkOperation();
  const shrinkExpandMutation = useShrinkExpandOperation();
  const makeMutation = useMakeManufacturableOperation();
  const createInspectionSnapshotMutation = useCreateInspectionSnapshot();
  const branchVersionMutation = useBranchVersion();

  const submitAndTrack = useCallback(async (promise: Promise<unknown>) => {
    const job = await promise as { id: string };
    setActiveJobId(job.id);
  }, []);

  const trackWorkbenchJob = useCallback((job: JobResponse) => {
    setActiveJobId(job.id);
  }, []);

  const activateCompletedJobVersion = useCallback((nextVersionId: string) => {
    if (modelId) {
      void queryClient.invalidateQueries({ queryKey: ['model-versions', modelId] });
    }
    if (versionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['manufacturability', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['compare-cache', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['inspection-snapshots', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['version-jobs', versionId] });
    }
    if (nextVersionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['manufacturability', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['compare-cache', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['inspection-snapshots', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['version-jobs', nextVersionId] });
    }
    setVersionId(nextVersionId);
    setActiveJobId(null);
  }, [modelId, queryClient, versionId]);

  const currentJob = useJobPolling(activeJobId, activateCompletedJobVersion);
  const jobEvents = useJobEventStream(activeJobId);
  const previousModelIdRef = useRef<string | null>(modelId);
  const handledTerminalJobIdRef = useRef<string | null>(null);

  useEffect(() => {
    const terminalStatus = jobEvents.terminalStatus;
    if (
      !activeJobId ||
      !terminalStatus ||
      terminalStatus.id !== activeJobId ||
      terminalStatus.status !== 'succeeded' ||
      !terminalStatus.version_id ||
      handledTerminalJobIdRef.current === terminalStatus.id
    ) {
      return;
    }
    handledTerminalJobIdRef.current = terminalStatus.id;
    activateCompletedJobVersion(terminalStatus.version_id);
  }, [activeJobId, activateCompletedJobVersion, jobEvents.terminalStatus]);

  const activateExactBooleanResult = useCallback((response: ExactBooleanResponse) => {
    setExactBooleanResult(response);
    setVersionId(response.version.id);
    setActiveJobId(null);
    if (modelId) {
      void queryClient.invalidateQueries({ queryKey: ['model-versions', modelId] });
    }
    if (versionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', versionId] });
    }
    void queryClient.invalidateQueries({ queryKey: ['version', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['manufacturability', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['version-jobs', response.version.id] });
  }, [modelId, queryClient, versionId]);

  const activateVoxelBooleanResult = useCallback((response: VoxelBooleanResponse) => {
    setVoxelBooleanResult(response);
    setVersionId(response.version.id);
    setActiveJobId(null);
    if (modelId) {
      void queryClient.invalidateQueries({ queryKey: ['model-versions', modelId] });
    }
    if (versionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', versionId] });
    }
    void queryClient.invalidateQueries({ queryKey: ['version', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['manufacturability', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['version-jobs', response.version.id] });
  }, [modelId, queryClient, versionId]);

  const activateOffsetShellResult = useCallback((response: OffsetShellMeshResponse) => {
    setOffsetShellResult(response);
    setVersionId(response.version.id);
    setActiveJobId(null);
    if (modelId) {
      void queryClient.invalidateQueries({ queryKey: ['model-versions', modelId] });
    }
    if (versionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', versionId] });
    }
    void queryClient.invalidateQueries({ queryKey: ['version', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['manufacturability', response.version.id] });
    void queryClient.invalidateQueries({ queryKey: ['version-jobs', response.version.id] });
  }, [modelId, queryClient, versionId]);

  useEffect(() => {
    if (previousModelIdRef.current === modelId) {
      return;
    }
    previousModelIdRef.current = modelId;
    resetWorkspaceState();
    setResizeAxisMode('auto');
    setManualResizeAxis(null);
    setUrlStateReady(false);
    urlSyncRef.current = null;
  }, [modelId, resetWorkspaceState]);

  useEffect(() => {
    if (!urlVersionId) {
      return;
    }
    setVersionId((current) => (current === urlVersionId ? current : urlVersionId));
  }, [urlVersionId]);

  useEffect(() => {
    if (urlJobId) {
      setActiveJobId(urlJobId);
    }
  }, [urlJobId]);

  useEffect(() => {
    if (!versionId || !compareTargetVersionId || compareTargetVersionId !== versionId) {
      return;
    }
    setCompareOverlayEnabled(false);
    setCompareTargetVersionId(null);
  }, [compareTargetVersionId, setCompareOverlayEnabled, setCompareTargetVersionId, versionId]);

  useEffect(() => {
    const decodeBoolean = (value: string | null, fallback: boolean) => {
      if (value === '1') return true;
      if (value === '0') return false;
      return fallback;
    };

    setWireframe(decodeBoolean(searchParams.get('wire'), false));
    setSectionEnabled(decodeBoolean(searchParams.get('section'), false));
    setHeatmapEnabled(decodeBoolean(searchParams.get('heatmap'), false));
    setRegionOverlayEnabled(decodeBoolean(searchParams.get('regions'), false));
    setCompareOverlayEnabled(decodeBoolean(searchParams.get('compare'), false));

    const planeValue = Number(searchParams.get('plane'));
    setSectionConstant(Number.isFinite(planeValue) ? planeValue : 0);

    const regionId = searchParams.get('region');
    setSelectedRegionId(regionId || null);
    const selectedIds = searchParams.get('regions_selected');
    setSelectedRegionIds(selectedIds ? selectedIds.split(',').filter(Boolean) : []);
    const axisMode = searchParams.get('axis_mode');
    setResizeAxisMode(axisMode === 'manual' ? 'manual' : 'auto');
    const axis = searchParams.get('axis');
    if (axis) {
      const values = axis.split(',').map(Number);
      if (values.length === 3 && values.every(Number.isFinite)) {
        setManualResizeAxis([values[0], values[1], values[2]]);
      } else {
        setManualResizeAxis(null);
      }
    } else {
      setManualResizeAxis(null);
    }

    const compareTarget = searchParams.get('compare_target');
    setCompareTargetVersionId(compareTarget || null);
    urlSyncRef.current = searchParams.toString();
    setUrlStateReady(true);
  }, [
    searchParams,
    setCompareOverlayEnabled,
    setCompareTargetVersionId,
    setHeatmapEnabled,
    setRegionOverlayEnabled,
    setSectionConstant,
    setSectionEnabled,
    setSelectedRegionId,
    setSelectedRegionIds,
    setWireframe,
  ]);

  useEffect(() => {
    if (!modelId || !versionId) {
      return;
    }
    if (!urlStateReady) {
      return;
    }

    const params = new URLSearchParams();
    params.set('model', modelId);
    params.set('version', versionId);
    if (activeJobId) params.set('job', activeJobId);
    if (wireframe) params.set('wire', '1');
    if (sectionEnabled) params.set('section', '1');
    if (sectionEnabled && Math.abs(sectionConstant) > 1e-6) params.set('plane', sectionConstant.toFixed(1));
    if (heatmapEnabled) params.set('heatmap', '1');
    if (regionOverlayEnabled) params.set('regions', '1');
    if (selectedRegionId) params.set('region', selectedRegionId);
    if (selectedRegionIds.length) params.set('regions_selected', selectedRegionIds.join(','));
    params.set('axis_mode', resizeAxisMode);
    if (manualResizeAxis) params.set('axis', manualResizeAxis.join(','));
    if (compareOverlayEnabled) params.set('compare', '1');
    if (compareTargetVersionId) params.set('compare_target', compareTargetVersionId);

    const next = params.toString();
    if (urlSyncRef.current === next) {
      return;
    }
    urlSyncRef.current = next;
    router.replace(`/viewer?${next}`, { scroll: false });
  }, [
    activeJobId,
    compareOverlayEnabled,
    compareTargetVersionId,
    heatmapEnabled,
    modelId,
    regionOverlayEnabled,
    router,
    resizeAxisMode,
    sectionConstant,
    sectionEnabled,
    selectedRegionId,
    selectedRegionIds,
    urlStateReady,
    versionId,
    wireframe,
    manualResizeAxis,
  ]);

  useEffect(() => {
    const regions = viewerQuery.data?.region_manifest ?? [];
    if (!regions.length) {
      return;
    }
    if (!selectedRegionId || !regions.some((region) => region.region_id === selectedRegionId)) {
      const preferred = regions.find((region) => region.allowed_operations.includes('scoop') && region.vertex_count > 0) ?? regions[0];
      setSelectedRegionId(preferred.region_id);
      setSelectedRegionIds([preferred.region_id]);
    }
  }, [selectedRegionId, setSelectedRegionId, setSelectedRegionIds, viewerQuery.data]);

  useEffect(() => {
    if (currentJob.data?.status !== 'succeeded' || currentJob.data.operation_type !== 'compare' || !versionId) {
      return;
    }
    void queryClient.invalidateQueries({ queryKey: ['compare-cache', versionId] });
    if (compareTargetVersionId) {
      void queryClient.invalidateQueries({ queryKey: ['compare-summary', versionId, compareTargetVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['compare-overlay', versionId, compareTargetVersionId] });
      setCompareOverlayEnabled(true);
    }
  }, [
    compareTargetVersionId,
    currentJob.data,
    queryClient,
    setCompareOverlayEnabled,
    versionId,
  ]);

  useEffect(() => {
    if (
      !compareOverlayEnabled ||
      !compareTargetVersionId ||
      !versionId ||
      compareTargetVersionId === versionId ||
      compareCacheTargets.has(compareTargetVersionId) ||
      !!activeJobId ||
      compareCacheQuery.isPending ||
      compareCacheQuery.isFetching ||
      compareMutation.isPending
    ) {
      return;
    }
    void submitAndTrack(compareMutation.mutateAsync({ versionId, params: { other_version_id: compareTargetVersionId } }));
  }, [
    activeJobId,
    compareCacheTargets,
    compareCacheQuery.isFetching,
    compareCacheQuery.isPending,
    compareMutation,
    compareMutation.isPending,
    compareOverlayEnabled,
    compareTargetVersionId,
    submitAndTrack,
    versionId,
  ]);

  const previewLowUrl = useMemo(() => {
    const path = viewerQuery.data?.preview_low_url;
    return path ? `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'}${path}` : null;
  }, [viewerQuery.data]);
  const previewHighUrl = useMemo(() => {
    const path = viewerQuery.data?.preview_high_url;
    return path ? `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'}${path}` : null;
  }, [viewerQuery.data]);
  const normalizedMeshUrl = useMemo(() => {
    const path = viewerQuery.data?.normalized_mesh_url;
    return path ? `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'}${path}` : null;
  }, [viewerQuery.data]);
  const regionArtifactUrl = useMemo(() => {
    const path = viewerQuery.data?.region_artifact_url;
    return path ? `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'}${path}` : null;
  }, [viewerQuery.data]);
  const textureArtifactUrl = useMemo(() => {
    const path = viewerQuery.data?.texture_artifact_url;
    return path ? `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000'}${path}` : null;
  }, [viewerQuery.data]);
  const textureArtifacts = useMemo<TextureArtifactManifest[]>(() => {
    const apiBase = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';
    return (viewerQuery.data?.texture_artifacts ?? []).map((texture) => ({
      ...texture,
      artifact_url:
        texture.artifact_url.startsWith('http://') || texture.artifact_url.startsWith('https://')
          ? texture.artifact_url
          : `${apiBase}${texture.artifact_url}`,
    }));
  }, [viewerQuery.data]);
  const scalarOverlay = compareOverlayEnabled ? compareOverlayQuery.data ?? null : heatmapEnabled ? thicknessOverlayQuery.data ?? null : null;
  const compareSummary =
    compareSummaryQuery.data ??
    (compareTargetVersionId
      ? (compareCacheQuery.data ?? []).find((entry) => entry.other_version_id === compareTargetVersionId)?.summary ?? null
      : null);
  const sectionAxis = useMemo<[number, number, number]>(() => {
    if (resizeAxisMode === 'manual' && manualResizeAxis) {
      return normalizeAxis(manualResizeAxis);
    }
    const detected = snapshotQuery.data?.dimensions.ring_axis ?? null;
    return normalizeAxis(detected);
  }, [manualResizeAxis, resizeAxisMode, snapshotQuery.data?.dimensions.ring_axis]);
  const sectionContourQuery = useSectionContour(
    versionId,
    sectionEnabled && versionArtifactsReady,
    sectionConstant,
    sectionAxis,
    selectedRegionIds,
  );
  const activeSectionContour = sectionContourQuery.data ?? null;
  const hasExportableSectionContour = Boolean(
    activeSectionContour?.segments.length &&
      activeSectionContour.projected_bounds_min &&
      activeSectionContour.projected_bounds_max,
  );
  const selectedRegion =
    (viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? []).find((region) => region.region_id === selectedRegionId) ?? null;
  const sectionPresets = useMemo(() => {
    const regions = viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? [];
    const presets = [
      { id: 'center', label: 'Centerline', description: 'Reset the section plane to the ring centerline.' },
    ];
    for (const regionId of ['inner_band', 'head', 'gem_seat', 'ornament_relief', 'outer_band'] as const) {
      const region = regions.find((entry) => entry.region_id === regionId && entry.centroid_mm);
      if (!region) continue;
      presets.push({
        id: region.region_id,
        label: region.label,
        description: `Snap to the ${region.label.toLowerCase()} section for focused inspection.`,
      });
    }
    return presets;
  }, [snapshotQuery.data?.regions, viewerQuery.data?.region_manifest]);

  const currentStlArtifact = versionDetailQuery.data?.artifacts.find((artifact) => artifact.artifact_type === 'manufacturing_stl') ?? null;
  const busy =
    repairMutation.isPending ||
    resizeMutation.isPending ||
    hollowMutation.isPending ||
    thickenMutation.isPending ||
    compareMutation.isPending ||
    exactBooleanMutation.isPending ||
    voxelBooleanMutation.isPending ||
    collisionMutation.isPending ||
    scoopMutation.isPending ||
    smoothMutation.isPending ||
    decimateMutation.isPending ||
    subdivideMutation.isPending ||
    makeDeloneMutation.isPending ||
    measureInspectMutation.isPending ||
    gcodeParseMutation.isPending ||
    pointCloudIcpMutation.isPending ||
    offsetContoursMutation.isPending ||
    distanceMapFromMeshMutation.isPending ||
    distanceMapContoursMutation.isPending ||
    distanceMapIsoLinesMutation.isPending ||
    distanceMapMergeMutation.isPending ||
    distanceMapContourBooleanMutation.isPending ||
    distanceMapFromTiffMutation.isPending ||
    distanceMapToTiffMutation.isPending ||
    objectLinesFromContoursMutation.isPending ||
    objectLinesLoadMrLinesMutation.isPending ||
    objectLinesLoadPlyMutation.isPending ||
    objectLinesLoadPtsMutation.isPending ||
    objectLinesLoadSvgMutation.isPending ||
    objectLinesSaveDxfMutation.isPending ||
    objectLinesSaveMrLinesMutation.isPending ||
    objectLinesSavePlyMutation.isPending ||
    objectLinesSavePtsMutation.isPending ||
    objectLinesToContoursMutation.isPending ||
    meshToVoxelsMutation.isPending ||
    openRawVoxelsMutation.isPending ||
    openVoxelsFromTiffMutation.isPending ||
    voxelVolumeRenderRayMutation.isPending ||
    offsetMeshMutation.isPending ||
    shellMeshMutation.isPending ||
    thickenMeshMutation.isPending ||
    weightedShellMutation.isPending ||
    partialOffsetMutation.isPending ||
    offsetVertsMutation.isPending ||
    expandShrinkMutation.isPending ||
    shrinkExpandMutation.isPending ||
    makeMutation.isPending ||
    (currentJob.data?.status === 'running') ||
    false;
  const activeToolLabel = useMemo(
    () => WORKSPACE_COMMANDS.find((command) => command.contextualToolId === activeTool)?.label ?? null,
    [activeTool],
  );
  const activeOverlays = useMemo(() => {
    const values: string[] = [];
    if (wireframe) values.push('Wireframe');
    if (sectionEnabled) values.push('Section');
    if (heatmapEnabled && !compareOverlayEnabled) values.push('Heatmap');
    if (regionOverlayEnabled) values.push('Regions');
    if (compareOverlayEnabled && compareOverlayReady) values.push('Compare');
    return values;
  }, [compareOverlayEnabled, compareOverlayReady, heatmapEnabled, regionOverlayEnabled, sectionEnabled, wireframe]);

  const terminalJobRecord = jobEvents.terminalStatus?.id === activeJobId ? jobEvents.terminalStatus : null;
  const activeJobStatus = terminalJobRecord?.status ?? currentJob.data?.status ?? null;
  const activeJobRecord = terminalJobRecord ?? currentJob.data ?? null;
  const activeJobFailure =
    activeJobStatus === 'failed'
      ? terminalJobRecord?.error_message ??
        terminalJobRecord?.error_code ??
        currentJob.data?.error_message ??
        currentJob.data?.error_code ??
        'The active job failed.'
      : null;
  const versionStatus = versionDetailQuery.data?.version.status ?? null;
  const viewerFailureMessage =
    activeJobFailure ??
    (versionStatus === 'failed'
      ? 'This version failed to prepare viewer artifacts. Check Activity for the failed job.'
      : viewerQuery.error instanceof Error
        ? viewerQuery.error.message
        : workbenchManifestQuery.error instanceof Error
          ? workbenchManifestQuery.error.message
          : snapshotQuery.error instanceof Error
            ? snapshotQuery.error.message
            : null);

  const onRepair = () => {
    if (!versionId) return;
    void submitAndTrack(repairMutation.mutateAsync(versionId));
  };

  const onResize = (request: ResizeRequestV2) => {
    if (!versionId) return;
    const axisAwareRequest: ResizeRequestV2 =
      resizeAxisMode === 'manual' && manualResizeAxis
        ? {
            ...request,
            axis_mode: 'manual',
            manual_axis: manualResizeAxis,
          }
        : {
            ...request,
            axis_mode: 'auto',
            manual_axis: undefined,
          };
    void submitAndTrack(resizeMutation.mutateAsync({ versionId, params: axisAwareRequest }));
  };

  const onHollow = (request: HollowRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(hollowMutation.mutateAsync({ versionId, params: request }));
  };

  const onThicken = (request: ThickenRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(thickenMutation.mutateAsync({ versionId, params: request }));
  };

  const onScoop = (request: ScoopRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(scoopMutation.mutateAsync({ versionId, params: request }));
  };

  const onSmooth = (request: SmoothRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(smoothMutation.mutateAsync({ versionId, params: request }));
  };

  const onDecimate = (request: DecimateRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(decimateMutation.mutateAsync({ versionId, params: request }));
  };

  const onSubdivide = (request: SubdivideRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(subdivideMutation.mutateAsync({ versionId, params: request }));
  };

  const onMakeDelone = (request: MakeDeloneRequestV2) => {
    if (!versionId) return;
    void submitAndTrack(makeDeloneMutation.mutateAsync({ versionId, params: request }));
  };

  const onMeasureInspect = (request: MeasureInspectRequest) => {
    if (!versionId) return;
    void measureInspectMutation.mutateAsync({ versionId, params: request }).then(setMeasureInspectResult);
  };

  const onGcodeParse = (request: GcodeParsePathsRequest) => {
    if (!versionId) return;
    void gcodeParseMutation.mutateAsync({ versionId, params: request }).then(setGcodeParseResult);
  };

  const onPointCloudIcp = (request: PointCloudIcpRequest) => {
    if (!versionId) return;
    void pointCloudIcpMutation.mutateAsync({ versionId, params: request }).then(setPointCloudIcpResult);
  };

  const onOffsetContours = (request: OffsetContoursRequest) => {
    if (!versionId) return;
    void offsetContoursMutation.mutateAsync({ versionId, params: request }).then(setOffsetContoursResult);
  };

  const onDistanceMapFromMesh = (request: DistanceMapFromMeshRequest) => {
    if (!versionId) return;
    void distanceMapFromMeshMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapFromMeshResult);
  };

  const onDistanceMapContours = (request: DistanceMapContoursRequest) => {
    if (!versionId) return;
    void distanceMapContoursMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapContoursResult);
  };

  const onDistanceMapIsoLines = (request: DistanceMapIsoLinesRequest) => {
    if (!versionId) return;
    void distanceMapIsoLinesMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapIsoLinesResult);
  };

  const onDistanceMapMerge = (request: DistanceMapMergeRequest) => {
    if (!versionId) return;
    void distanceMapMergeMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapMergeResult);
  };

  const onDistanceMapContourBoolean = (request: DistanceMapContourBooleanRequest) => {
    if (!versionId) return;
    void distanceMapContourBooleanMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapContourBooleanResult);
  };

  const onDistanceMapFromTiff = (request: DistanceMapTiffImportRequest) => {
    if (!versionId) return;
    void distanceMapFromTiffMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapTiffImportResult);
  };

  const onDistanceMapToTiff = (request: DistanceMapTiffExportRequest) => {
    if (!versionId) return;
    void distanceMapToTiffMutation
      .mutateAsync({ versionId, params: request })
      .then(setDistanceMapTiffExportResult);
  };

  const onObjectLinesFromContours = (request: ObjectLinesFromContoursRequest) => {
    if (!versionId) return;
    void objectLinesFromContoursMutation.mutateAsync({ versionId, params: request }).then(setObjectLinesResult);
  };

  const onObjectLinesLoadMrLines = (request: ObjectLinesBinaryLoadRequest) => {
    if (!versionId) return;
    void objectLinesLoadMrLinesMutation.mutateAsync({ versionId, params: request }).then(setObjectLinesResult);
  };

  const onObjectLinesSaveMrLines = (request: ObjectLinesBinaryExportRequest) => {
    if (!versionId) return;
    void objectLinesSaveMrLinesMutation
      .mutateAsync({ versionId, params: request })
      .then(setObjectLinesMrLinesExportResult);
  };

  const onObjectLinesLoadPly = (request: ObjectLinesBinaryLoadRequest) => {
    if (!versionId) return;
    void objectLinesLoadPlyMutation.mutateAsync({ versionId, params: request }).then(setObjectLinesResult);
  };

  const onObjectLinesSavePly = (request: ObjectLinesBinaryExportRequest) => {
    if (!versionId) return;
    void objectLinesSavePlyMutation
      .mutateAsync({ versionId, params: request })
      .then(setObjectLinesPlyExportResult);
  };

  const onObjectLinesLoadPts = (request: ObjectLinesPtsLoadRequest) => {
    if (!versionId) return;
    void objectLinesLoadPtsMutation.mutateAsync({ versionId, params: request }).then(setObjectLinesResult);
  };

  const onObjectLinesSavePts = (request: ObjectLinesTextExportRequest) => {
    if (!versionId) return;
    void objectLinesSavePtsMutation
      .mutateAsync({ versionId, params: request })
      .then(setObjectLinesPtsExportResult);
  };

  const onObjectLinesLoadSvg = (request: ObjectLinesSvgLoadRequest) => {
    if (!versionId) return;
    void objectLinesLoadSvgMutation.mutateAsync({ versionId, params: request }).then(setObjectLinesResult);
  };

  const onObjectLinesSaveDxf = (request: ObjectLinesTextExportRequest) => {
    if (!versionId) return;
    void objectLinesSaveDxfMutation
      .mutateAsync({ versionId, params: request })
      .then(setObjectLinesDxfExportResult);
  };

  const onObjectLinesToContours = (request: ObjectLinesToContoursRequest) => {
    if (!versionId) return;
    void objectLinesToContoursMutation
      .mutateAsync({ versionId, params: request })
      .then(setObjectLinesContoursResult);
  };

  const onMeshToVoxelsSdf = (request: MeshToVoxelsSdfRequest) => {
    if (!versionId) return;
    void meshToVoxelsMutation.mutateAsync({ versionId, params: request }).then(setMeshToVoxelsResult);
  };

  const onOpenRawVoxels = (request: VoxelRawLoadRequest) => {
    if (!versionId) return;
    void openRawVoxelsMutation.mutateAsync({ versionId, params: request }).then(setVoxelLoadResult);
  };

  const onOpenVoxelsFromTiff = (request: VoxelTiffLoadRequest) => {
    if (!versionId) return;
    void openVoxelsFromTiffMutation.mutateAsync({ versionId, params: request }).then(setVoxelLoadResult);
  };

  const onVoxelVolumeRenderRay = (request: VoxelVolumeRenderRayRequest) => {
    if (!versionId) return;
    void voxelVolumeRenderRayMutation
      .mutateAsync({ versionId, params: request })
      .then(setVoxelVolumeRenderRayResult);
  };

  const onOffsetMesh = (request: OffsetMeshRequest) => {
    if (!versionId) return;
    void offsetMeshMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onShellMesh = (request: ShellMeshRequest) => {
    if (!versionId) return;
    void shellMeshMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onThickenMesh = (request: ThickenMeshRequest) => {
    if (!versionId) return;
    void thickenMeshMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onWeightedShell = (request: WeightedShellRequest) => {
    if (!versionId) return;
    void weightedShellMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onPartialOffset = (request: PartialOffsetRequest) => {
    if (!versionId) return;
    void partialOffsetMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onOffsetVerts = (request: OffsetVertsRequest) => {
    if (!versionId) return;
    void offsetVertsMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onExpandShrink = (request: OffsetSmoothingRequest) => {
    if (!versionId) return;
    void expandShrinkMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onShrinkExpand = (request: OffsetSmoothingRequest) => {
    if (!versionId) return;
    void shrinkExpandMutation.mutateAsync({ versionId, params: request }).then(activateOffsetShellResult);
  };

  const onExactBoolean = (request: ExactBooleanRequest) => {
    if (!versionId) return;
    void exactBooleanMutation.mutateAsync({ versionId, params: request }).then(activateExactBooleanResult);
  };

  const onVoxelBoolean = (request: VoxelBooleanRequest) => {
    if (!versionId) return;
    void voxelBooleanMutation.mutateAsync({ versionId, params: request }).then(activateVoxelBooleanResult);
  };

  const onCollisionDetect = (request: CollisionDetectRequest) => {
    if (!versionId) return;
    void collisionMutation.mutateAsync({ versionId, params: request }).then(setCollisionResult);
  };

  const onRegionPick = (regionId: string, additive?: boolean) => {
    if (additive) {
      toggleSelectedRegionId(regionId);
      return;
    }
    setSelectedRegionId(regionId);
  };

  const onSnapToRegion = () => {
    if (selectedRegion?.centroid_mm) {
      setSectionEnabled(true);
      setSectionConstant(dot(selectedRegion.centroid_mm, sectionAxis));
    }
  };

  const onSnapToCenter = () => {
    setSectionEnabled(true);
    setSectionConstant(0);
  };

  const onExportSection = () => {
    if (!activeSectionContour) {
      return;
    }
    downloadSectionContourSvg(activeSectionContour, versionId ?? 'section');
  };

  const onApplySectionPreset = (presetId: string) => {
    if (presetId === 'center') {
      onSnapToCenter();
      return;
    }
    const regions = viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? [];
    const region = regions.find((entry) => entry.region_id === presetId && entry.centroid_mm);
    if (!region?.centroid_mm) {
      return;
    }
    setSelectedRegionId(region.region_id);
    setSectionEnabled(true);
    setSectionConstant(dot(region.centroid_mm, sectionAxis));
  };

  const currentInspectionSnapshotState = (name: string): InspectionSnapshotState => ({
    name,
    section_enabled: sectionEnabled,
    section_constant: sectionConstant,
    selected_region_id: selectedRegionId,
    selected_region_ids: selectedRegionIds,
    axis_mode: resizeAxisMode,
    manual_axis: manualResizeAxis,
    heatmap_enabled: heatmapEnabled,
    compare_enabled: compareOverlayEnabled,
    compare_target_version_id: compareTargetVersionId,
  });

  const onSaveInspection = (name: string) => {
    if (!versionId) return;
    createInspectionSnapshotMutation.mutate({
      versionId,
      params: currentInspectionSnapshotState(name),
    });
  };

  const onLoadInspection = (snapshot: InspectionSnapshotResponse) => {
    setSectionEnabled(snapshot.section_enabled);
    setSectionConstant(snapshot.section_constant);
    setSelectedRegionIds(snapshot.selected_region_ids);
    setSelectedRegionId(snapshot.selected_region_id);
    setResizeAxisMode(snapshot.axis_mode);
    setManualResizeAxis(snapshot.manual_axis);
    setHeatmapEnabled(snapshot.heatmap_enabled);
    setCompareOverlayEnabled(snapshot.compare_enabled);
    setCompareTargetVersionId(snapshot.compare_target_version_id);
  };

  const onMakeManufacturable = (request: MakeManufacturableRequest) => {
    if (!versionId) return;
    void submitAndTrack(makeMutation.mutateAsync({ versionId, params: request }));
  };

  const onRequestCompare = (otherVersionId: string | null) => {
    setCompareTargetVersionId(otherVersionId);
    if (!versionId || !otherVersionId || otherVersionId === versionId) {
      if (otherVersionId === versionId) {
        setCompareTargetVersionId(null);
      }
      setCompareOverlayEnabled(false);
      return;
    }
    if (compareCacheTargets.has(otherVersionId)) {
      setCompareOverlayEnabled(true);
      return;
    }
    setCompareOverlayEnabled(false);
    void submitAndTrack(compareMutation.mutateAsync({ versionId, params: { other_version_id: otherVersionId } }));
  };

  const onOpenVersion = (nextVersionId: string) => {
    setVersionId(nextVersionId);
    setActiveJobId(null);
    if (compareTargetVersionId === nextVersionId) {
      setCompareOverlayEnabled(false);
      setCompareTargetVersionId(null);
    }
  };

  const onBranchVersion = (sourceVersionId: string) => {
    void branchVersionMutation.mutateAsync({
      versionId: sourceVersionId,
      params: { operation_label: `Restore Branch from ${sourceVersionId}` },
    }).then((nextVersion) => {
      setVersionId(nextVersion.id);
      setActiveJobId(null);
    });
  };

  const onCompareVersion = (otherVersionId: string) => {
    onRequestCompare(otherVersionId);
    setReviewPane('compare');
    setRightDockTab('review');
  };

  const onOpenToolbarGroup = (group: ToolbarGroup) => {
    setActiveToolbarGroup(group);
    setOpenPopoverGroup(group);
  };

  const onCloseToolbarGroup = () => {
    setOpenPopoverGroup(null);
  };

  const onDownloadStl = () => {
    if (!currentStlArtifact) return;
    window.open(getArtifactUrl(currentStlArtifact.id), '_blank', 'noopener,noreferrer');
  };

  const getCommandAvailability = (commandId: WorkspaceCommandId) => {
    switch (commandId) {
      case 'download-stl':
        return { disabled: !currentStlArtifact, reason: currentStlArtifact ? undefined : 'No manufacturing STL is ready for this version.' };
      case 'export-section':
        return {
          disabled: !hasExportableSectionContour,
          reason: hasExportableSectionContour ? undefined : 'Enable a section with contour data before exporting.',
        };
      case 'thicken-region':
        return getSelectedRegionOperationAvailability(selectedRegion, 'thicken', 'thickening');
      case 'batch-thicken':
        return getBatchRegionOperationAvailability(
          viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? [],
          selectedRegionIds,
          'thicken',
          'Batch thickening',
        );
      case 'batch-smooth':
        return getBatchRegionOperationAvailability(
          viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? [],
          selectedRegionIds,
          'smooth',
          'Batch smoothing',
        );
      case 'scoop':
        return getScoopCommandAvailability(
          viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? [],
          selectedRegionId,
          toolDrafts.scoopDepth,
          toolDrafts.minThickness,
        );
      case 'compare-versions':
      case 'version-history':
      case 'restore-branch':
        return {
          disabled: (versionsQuery.data ?? []).length < 2,
          reason: (versionsQuery.data ?? []).length >= 2 ? undefined : 'Create at least one derived version to use review workflows.',
        };
      default:
        return { disabled: false };
    }
  };

  const onCommandSelect = (commandId: WorkspaceCommandId, invocation?: WorkbenchCommandInvocation) => {
    const runtimeBrushCommandId =
      commandId === 'runtime-thicken-brush' || commandId === 'runtime-scoop-brush' || commandId === 'runtime-smooth-brush'
        ? commandId
        : null;
    if (runtimeBrushCommandId) {
      if (!versionId || !shouldExecuteWorkbenchCommand(invocation)) {
        return;
      }
      const workbenchRequest = requestPayloadFromWorkbenchCommand(invocation);
      const request = brushReplayRequestFromWorkbenchPayload(workbenchRequest, runtimeBrushCommandId);
      if (!request) {
        return;
      }
      setActiveTool(runtimeBrushCommandId, 'modify');
      setRightDockTab('tool');
      void submitAndTrack(brushReplayMutation.mutateAsync({ versionId, params: request }));
      return;
    }

    if (
      commandId === 'voxel-binary-operations' ||
      commandId === 'voxel-slice' ||
      commandId === 'voxel-line-graph' ||
      commandId === 'voxel-active-box' ||
      commandId === 'voxel-path' ||
      commandId === 'voxel-path-build-four' ||
      commandId === 'voxel-segmentation' ||
      commandId === 'voxel-mask-to-mesh' ||
      commandId === 'voxel-to-mesh-simple' ||
      commandId === 'voxel-to-mesh-dual' ||
      commandId === 'voxel-to-mesh-smart' ||
      commandId === 'voxel-volume-render-data' ||
      commandId === 'voxel-volume-render-lut'
    ) {
      if (!versionId || !shouldExecuteWorkbenchCommand(invocation)) {
        return;
      }
      const workbenchRequest = requestPayloadFromWorkbenchCommand(invocation);
      if (commandId === 'voxel-binary-operations') {
        void voxelBinaryOperationsMutation.mutateAsync({
          versionId,
          params: voxelBinaryOperationsRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-slice') {
        void voxelSliceMutation.mutateAsync({
          versionId,
          params: voxelSliceRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-line-graph') {
        void voxelLineGraphMutation.mutateAsync({
          versionId,
          params: voxelLineGraphRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-active-box') {
        void voxelActiveBoxMutation.mutateAsync({
          versionId,
          params: voxelActiveBoxRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-path') {
        void voxelPathMutation.mutateAsync({
          versionId,
          params: voxelPathRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-path-build-four') {
        void voxelPathBuildFourMutation.mutateAsync({
          versionId,
          params: voxelPathBuildFourRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-segmentation') {
        void voxelSegmentationMutation.mutateAsync({
          versionId,
          params: voxelSegmentationRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-mask-to-mesh') {
        void voxelMaskToMeshMutation.mutateAsync({
          versionId,
          params: voxelMaskToMeshRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-to-mesh-simple') {
        void voxelToMeshSimpleMutation.mutateAsync({
          versionId,
          params: voxelToMeshSimpleRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-to-mesh-dual') {
        void voxelToMeshDualMutation.mutateAsync({
          versionId,
          params: voxelToMeshDualRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-to-mesh-smart') {
        void voxelToMeshSmartMutation.mutateAsync({
          versionId,
          params: voxelToMeshSmartRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      if (commandId === 'voxel-volume-render-data') {
        void voxelVolumeRenderDataMutation.mutateAsync({
          versionId,
          params: voxelVolumeRenderDataRequestFromWorkbenchPayload(workbenchRequest),
        });
        return;
      }
      void voxelVolumeRenderLutMutation.mutateAsync({
        versionId,
        params: voxelVolumeRenderLutRequestFromWorkbenchPayload(workbenchRequest),
      });
      return;
    }

    const definition = WORKSPACE_COMMANDS.find((command) => command.id === commandId);
    if (!definition) return;
    const availability = getCommandAvailability(commandId);
    const shouldExecuteWorkbenchRequest = shouldExecuteWorkbenchCommand(invocation);
    if (availability.disabled && !shouldExecuteWorkbenchRequest) {
      return;
    }
    const workbenchRequest = requestPayloadFromWorkbenchCommand(invocation);
    const workbenchState = statePayloadFromWorkbenchCommand(invocation);

    setActiveToolbarGroup(definition.group);

    if (shouldExecuteWorkbenchRequest && versionId) {
      switch (commandId) {
        case 'download-stl': {
          const downloadUrl = downloadUrlFromWorkbenchInvocation(invocation, currentStlArtifact ? getArtifactUrl(currentStlArtifact.id) : null);
          if (downloadUrl) {
            window.open(downloadUrl, '_blank', 'noopener,noreferrer');
          }
          return;
        }
        case 'export-section': {
          const sectionParams = sectionContourParamsFromWorkbenchPayload(
            workbenchState,
            sectionConstant,
            sectionAxis,
            selectedRegionIds,
          );
          setSectionEnabled(true);
          setSectionConstant(sectionParams.section_constant);
          setResizeAxisMode('manual');
          setManualResizeAxis(sectionParams.plane_axis);
          if (sectionParams.selected_region_ids.length > 0) {
            setSelectedRegionIds(sectionParams.selected_region_ids);
            setSelectedRegionId(sectionParams.selected_region_ids[0]);
          }
          void getSectionContour(versionId, sectionParams).then((contour) => {
            downloadSectionContourSvg(contour, versionId);
          });
          return;
        }
        case 'job-activity': {
          const jobId = jobIdFromWorkbenchPayload(workbenchState);
          if (jobId) {
            setActiveJobId(jobId);
          }
          setRightDockTab('activity');
          return;
        }
        case 'repair':
          void submitAndTrack(repairMutation.mutateAsync(versionId));
          return;
        case 'fit-size':
        case 'resize':
          void submitAndTrack(resizeMutation.mutateAsync({
            versionId,
            params: resizeRequestFromWorkbenchPayload(
              workbenchRequest,
              commandId === 'fit-size' ? toolDrafts.targetRingSize : toolDrafts.resizeTargetSize,
              resizeAxisMode,
              manualResizeAxis,
            ),
          }));
          return;
        case 'reduce-weight':
          void submitAndTrack(hollowMutation.mutateAsync({
            versionId,
            params: hollowRequestFromWorkbenchPayload(
              { ...workbenchRequest, mode: workbenchRequest.mode ?? 'target_weight' },
              selectedMaterial,
              toolDrafts.wallThickness,
              toolDrafts.targetWeight,
              toolDrafts.minThickness,
              false,
            ),
          }));
          return;
        case 'prepare-casting':
        case 'hollow-drains':
        case 'protected-hollow':
          void submitAndTrack(hollowMutation.mutateAsync({
            versionId,
            params: hollowRequestFromWorkbenchPayload(
              workbenchRequest,
              selectedMaterial,
              toolDrafts.wallThickness,
              toolDrafts.targetWeight,
              toolDrafts.minThickness,
              commandId !== 'protected-hollow',
            ),
          }));
          return;
        case 'make-manufacturable':
          void submitAndTrack(makeMutation.mutateAsync({
            versionId,
            params: {
              material: materialFromPayload(workbenchRequest, selectedMaterial),
              target_ring_size_us: numberFromPayload(
                workbenchRequest,
                ['target_ring_size_us', 'target_size_us', 'ring_size_us', 'ring_size'],
                toolDrafts.targetRingSize,
              ),
              target_weight_g: numberFromPayload(workbenchRequest, ['target_weight_g', 'target_weight'], toolDrafts.targetWeight),
              min_allowed_thickness_mm: numberFromPayload(
                workbenchRequest,
                ['min_allowed_thickness_mm', 'min_thickness_mm'],
                toolDrafts.minThickness,
              ),
            },
          }));
          return;
        case 'thicken-violations':
        case 'thicken-region':
        case 'batch-thicken': {
          const request = thickenRequestFromWorkbenchPayload(
            workbenchRequest,
            commandId === 'thicken-violations' ? 'violations_only' : commandId === 'batch-thicken' ? 'selected_regions' : 'selected_region',
            toolDrafts.thickenTarget,
            selectedRegionId,
            selectedRegionIds,
          );
          if (request.region_id) {
            setSelectedRegionId(request.region_id);
          }
          if (request.region_ids?.length) {
            setSelectedRegionIds(request.region_ids);
          }
          void submitAndTrack(thickenMutation.mutateAsync({ versionId, params: request }));
          return;
        }
        case 'scoop': {
          const regionId = stringFromPayload(workbenchRequest, ['region_id', 'region']) ?? selectedRegionId;
          if (!regionId) {
            break;
          }
          setSelectedRegionId(regionId);
          void submitAndTrack(scoopMutation.mutateAsync({
            versionId,
            params: {
              region_id: regionId,
              depth_mm: numberFromPayload(workbenchRequest, ['depth_mm', 'scoop_depth_mm'], toolDrafts.scoopDepth),
              falloff_mm: numberFromPayload(workbenchRequest, ['falloff_mm', 'brush_radius_mm'], toolDrafts.scoopFalloff),
              keep_min_thickness_mm: numberFromPayload(
                workbenchRequest,
                ['keep_min_thickness_mm', 'min_thickness_mm'],
                toolDrafts.minThickness,
              ),
            },
          }));
          return;
        }
        case 'smooth':
        case 'batch-smooth': {
          const request = smoothRequestFromWorkbenchPayload(
            workbenchRequest,
            toolDrafts.smoothIterations,
            toolDrafts.smoothStrength,
            selectedRegionId,
            commandId === 'batch-smooth' ? selectedRegionIds : [],
          );
          if (request.region_id) {
            setSelectedRegionId(request.region_id);
          }
          if (request.region_ids?.length) {
            setSelectedRegionIds(request.region_ids);
          }
          void submitAndTrack(smoothMutation.mutateAsync({ versionId, params: request }));
          return;
        }
        case 'decimate-mesh': {
          const request = decimateRequestFromWorkbenchPayload(
            workbenchRequest,
            toolDrafts.decimateStrategy,
            toolDrafts.decimateMaxError,
            toolDrafts.decimateTargetFaces,
            toolDrafts.decimateTargetPercent,
            toolDrafts.decimateMaxEdgeLen,
            toolDrafts.decimateMaxBoundaryShift,
            toolDrafts.decimateStabilizer,
            toolDrafts.decimateParallelAlgorithm,
            toolDrafts.decimateSubdivideParts,
            toolDrafts.decimateRegionFaces,
            toolDrafts.decimateNotFlippableEdges,
            toolDrafts.decimateCollapseNearNotFlippable,
            toolDrafts.decimateAngleWeightedDistToPlane,
            toolDrafts.decimateMaxDeletedVertices,
            toolDrafts.decimateMaxDeletedFaces,
            toolDrafts.decimateMaxTriangleAspectRatio,
            toolDrafts.decimateTouchNearBoundaryEdges,
            toolDrafts.decimateTouchBoundaryVerts,
            toolDrafts.decimateOptimizeVertexPos,
            toolDrafts.decimatePackMesh,
          );
          setActiveTool('decimate-mesh', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            decimateStrategy: request.strategy,
            decimateMaxError: request.max_error,
            decimateTargetFaces: request.target_face_count ?? 0,
            decimateTargetPercent: request.target_face_ratio ? request.target_face_ratio * 100 : 0,
            decimateMaxEdgeLen: request.max_edge_len ?? 0,
            decimateMaxBoundaryShift: request.max_bd_shift ?? 0,
            decimateStabilizer: request.stabilizer,
            decimateParallelAlgorithm: request.subdivide_parts > 1,
            decimateSubdivideParts: request.subdivide_parts,
            decimateRegionFaces: formatIntegerList(request.region_faces ?? []),
            decimateNotFlippableEdges: formatEdgePairs(request.not_flippable_edges ?? []),
            decimateCollapseNearNotFlippable: request.collapse_near_not_flippable,
            decimateAngleWeightedDistToPlane: request.angle_weighted_dist_to_plane,
            decimateMaxDeletedVertices: request.max_deleted_vertices,
            decimateMaxDeletedFaces: request.max_deleted_faces,
            decimateMaxTriangleAspectRatio: request.max_triangle_aspect_ratio,
            decimateTouchNearBoundaryEdges: request.touch_near_bd_edges,
            decimateTouchBoundaryVerts: request.touch_bd_verts,
            decimateOptimizeVertexPos: request.optimize_vertex_pos,
            decimatePackMesh: request.pack_mesh,
          });
          void submitAndTrack(decimateMutation.mutateAsync({ versionId, params: request }));
          return;
        }
        case 'subdivide-mesh': {
          const request = subdivideRequestFromWorkbenchPayload(
            workbenchRequest,
            toolDrafts.subdivideMaxEdgeLen,
            toolDrafts.subdivideMaxEdgeSplits,
            toolDrafts.subdivideBorder,
            toolDrafts.subdivideCurvaturePriority,
            toolDrafts.subdivideProjectOnOriginalMesh,
            toolDrafts.subdivideSmoothMode,
            toolDrafts.subdivideMinSharpDihedralAngle,
            toolDrafts.subdivideMaxTriAspectRatio,
            toolDrafts.subdivideMaxSplittableTriAspectRatio,
            toolDrafts.subdivideMaxDeviationAfterFlip,
            toolDrafts.subdivideMaxAngleChangeAfterFlip,
            toolDrafts.subdivideCriticalTriAspectRatioFlip,
            toolDrafts.subdivideRegionFaces,
            toolDrafts.subdivideNotFlippableEdges,
          );
          setActiveTool('subdivide-mesh', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            subdivideMaxEdgeLen: request.max_edge_len,
            subdivideMaxEdgeSplits: request.max_edge_splits,
            subdivideBorder: request.subdivide_border,
            subdivideCurvaturePriority: request.curvature_priority,
            subdivideProjectOnOriginalMesh: request.project_on_original_mesh,
            subdivideSmoothMode: request.smooth_mode,
            subdivideMinSharpDihedralAngle: request.min_sharp_dihedral_angle,
            subdivideMaxTriAspectRatio: request.max_tri_aspect_ratio,
            subdivideMaxSplittableTriAspectRatio: request.max_splittable_tri_aspect_ratio ?? 0,
            subdivideMaxDeviationAfterFlip: request.max_deviation_after_flip ?? 0,
            subdivideMaxAngleChangeAfterFlip: request.max_angle_change_after_flip ?? 0,
            subdivideCriticalTriAspectRatioFlip: request.critical_tri_aspect_ratio_flip ?? 0,
            subdivideRegionFaces: formatIntegerList(request.region_faces ?? []),
            subdivideNotFlippableEdges: formatEdgePairs(request.not_flippable_edges ?? []),
          });
          void submitAndTrack(subdivideMutation.mutateAsync({ versionId, params: request }));
          return;
        }
        case 'make-delone': {
          const request = makeDeloneRequestFromWorkbenchPayload(
            workbenchRequest,
            toolDrafts.makeDeloneNumIters,
            toolDrafts.makeDeloneMaxDeviationAfterFlip,
            toolDrafts.makeDeloneMaxAngleChange,
            toolDrafts.makeDeloneCriticalTriAspectRatio,
            toolDrafts.makeDeloneRegionFaces,
            toolDrafts.makeDeloneNotFlippableEdges,
            toolDrafts.makeDeloneVertRegion,
          );
          setActiveTool('make-delone', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            makeDeloneNumIters: request.num_iters,
            makeDeloneMaxDeviationAfterFlip: request.max_deviation_after_flip ?? 0,
            makeDeloneMaxAngleChange: request.max_angle_change ?? 0,
            makeDeloneCriticalTriAspectRatio: request.critical_tri_aspect_ratio ?? 0,
            makeDeloneRegionFaces: formatIntegerList(request.region_faces ?? []),
            makeDeloneNotFlippableEdges: formatEdgePairs(request.not_flippable_edges ?? []),
            makeDeloneVertRegion: formatIntegerList(request.vert_region ?? []),
          });
          void submitAndTrack(makeDeloneMutation.mutateAsync({ versionId, params: request }));
          return;
        }
        case 'offset-mesh': {
          const request = offsetMeshRequestFromWorkbenchPayload(workbenchRequest, {
            offsetMm: toolDrafts.offsetDistanceMm,
            voxelSizeMm: toolDrafts.offsetVoxelSizeMm,
            paddingMm: toolDrafts.offsetPaddingMm,
            refine: toolDrafts.offsetRefine,
          });
          setActiveTool('offset-mesh', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            offsetDistanceMm: request.offset_mm,
            offsetVoxelSizeMm: request.voxel_size_mm,
            offsetPaddingMm: request.padding_mm ?? toolDrafts.offsetPaddingMm,
            offsetRefine: Boolean(request.refine),
          });
          void offsetMeshMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'shell-mesh': {
          const request = shellMeshRequestFromWorkbenchPayload(workbenchRequest, {
            wallThicknessMm: toolDrafts.shellWallThicknessMm,
            voxelSizeMm: toolDrafts.shellVoxelSizeMm,
            paddingMm: toolDrafts.shellPaddingMm,
            refine: toolDrafts.shellRefine,
          });
          setActiveTool('shell-mesh', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            shellWallThicknessMm: request.wall_thickness_mm,
            shellVoxelSizeMm: request.voxel_size_mm,
            shellPaddingMm: request.padding_mm ?? toolDrafts.shellPaddingMm,
            shellRefine: Boolean(request.refine),
          });
          void shellMeshMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'thicken-mesh': {
          const request = thickenMeshRequestFromWorkbenchPayload(workbenchRequest, {
            thicknessMm: toolDrafts.thickenMeshThicknessMm,
            voxelSizeMm: toolDrafts.thickenMeshVoxelSizeMm,
            paddingMm: toolDrafts.thickenMeshPaddingMm,
            refine: toolDrafts.thickenMeshRefine,
          });
          setActiveTool('thicken-mesh', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            thickenMeshThicknessMm: request.thickness_mm,
            thickenMeshVoxelSizeMm: request.voxel_size_mm,
            thickenMeshPaddingMm: request.padding_mm ?? toolDrafts.thickenMeshPaddingMm,
            thickenMeshRefine: Boolean(request.refine),
          });
          void thickenMeshMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'weighted-shell': {
          const fallbackRegionIds = selectedRegionIds.length > 0 ? selectedRegionIds : selectedRegionId ? [selectedRegionId] : [];
          const request = weightedShellRequestFromWorkbenchPayload(workbenchRequest, {
            offsetMm: toolDrafts.weightedShellOffsetMm,
            regionWeightMm: toolDrafts.weightedShellRegionWeightMm,
            interpolationMm: toolDrafts.weightedShellInterpolationMm,
            voxelSizeMm: toolDrafts.weightedShellVoxelSizeMm,
            paddingMm: toolDrafts.weightedShellPaddingMm,
            refine: toolDrafts.weightedShellRefine,
            regionIds: fallbackRegionIds,
          });
          setActiveTool('weighted-shell', 'modify');
          setRightDockTab('tool');
          if (request.region_weights.length > 0) {
            setSelectedRegionIds(request.region_weights.map((entry) => entry.region_id));
            setSelectedRegionId(request.region_weights[0].region_id);
          }
          updateToolDrafts({
            weightedShellOffsetMm: request.offset_mm,
            weightedShellRegionWeightMm: request.region_weights[0]?.weight_mm ?? toolDrafts.weightedShellRegionWeightMm,
            weightedShellInterpolationMm: request.interpolation_distance_mm ?? toolDrafts.weightedShellInterpolationMm,
            weightedShellVoxelSizeMm: request.voxel_size_mm,
            weightedShellPaddingMm: request.padding_mm ?? toolDrafts.weightedShellPaddingMm,
            weightedShellRefine: Boolean(request.refine),
          });
          void weightedShellMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'partial-offset': {
          const fallbackRegionIds = selectedRegionIds.length > 0 ? selectedRegionIds : selectedRegionId ? [selectedRegionId] : [];
          const request = partialOffsetRequestFromWorkbenchPayload(workbenchRequest, {
            offsetMm: toolDrafts.partialOffsetDistanceMm,
            voxelSizeMm: toolDrafts.partialOffsetVoxelSizeMm,
            paddingMm: toolDrafts.partialOffsetPaddingMm,
            refine: toolDrafts.partialOffsetRefine,
            regionIds: fallbackRegionIds,
          });
          setActiveTool('partial-offset', 'modify');
          setRightDockTab('tool');
          if (request.region_ids.length > 0) {
            setSelectedRegionIds(request.region_ids);
            setSelectedRegionId(request.region_ids[0]);
          }
          updateToolDrafts({
            partialOffsetDistanceMm: request.offset_mm,
            partialOffsetVoxelSizeMm: request.voxel_size_mm,
            partialOffsetPaddingMm: request.padding_mm ?? toolDrafts.partialOffsetPaddingMm,
            partialOffsetRefine: Boolean(request.refine),
          });
          void partialOffsetMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'offset-verts': {
          const fallbackRegionIds = selectedRegionIds.length > 0 ? selectedRegionIds : selectedRegionId ? [selectedRegionId] : [];
          const request = offsetVertsRequestFromWorkbenchPayload(workbenchRequest, {
            offsetMm: toolDrafts.offsetVertsDistanceMm,
            regionIds: fallbackRegionIds,
          });
          setActiveTool('offset-verts', 'modify');
          setRightDockTab('tool');
          if (request.region_ids.length > 0) {
            setSelectedRegionIds(request.region_ids);
            setSelectedRegionId(request.region_ids[0]);
          }
          updateToolDrafts({
            offsetVertsDistanceMm: request.offset_mm,
          });
          void offsetVertsMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'expand-shrink': {
          const request = offsetSmoothingRequestFromWorkbenchPayload(workbenchRequest, {
            distanceMm: toolDrafts.expandShrinkDistanceMm,
            voxelSizeMm: toolDrafts.expandShrinkVoxelSizeMm,
            paddingMm: toolDrafts.expandShrinkPaddingMm,
            refine: toolDrafts.expandShrinkRefine,
          });
          setActiveTool('expand-shrink', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            expandShrinkDistanceMm: request.distance_mm,
            expandShrinkVoxelSizeMm: request.voxel_size_mm,
            expandShrinkPaddingMm: request.padding_mm ?? toolDrafts.expandShrinkPaddingMm,
            expandShrinkRefine: Boolean(request.refine),
          });
          void expandShrinkMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'shrink-expand': {
          const request = offsetSmoothingRequestFromWorkbenchPayload(workbenchRequest, {
            distanceMm: toolDrafts.shrinkExpandDistanceMm,
            voxelSizeMm: toolDrafts.shrinkExpandVoxelSizeMm,
            paddingMm: toolDrafts.shrinkExpandPaddingMm,
            refine: toolDrafts.shrinkExpandRefine,
          });
          setActiveTool('shrink-expand', 'modify');
          setRightDockTab('tool');
          updateToolDrafts({
            shrinkExpandDistanceMm: request.distance_mm,
            shrinkExpandVoxelSizeMm: request.voxel_size_mm,
            shrinkExpandPaddingMm: request.padding_mm ?? toolDrafts.shrinkExpandPaddingMm,
            shrinkExpandRefine: Boolean(request.refine),
          });
          void shrinkExpandMutation
            .mutateAsync({ versionId, params: request })
            .then(activateOffsetShellResult);
          return;
        }
        case 'wireframe': {
          setWireframe(booleanFromPayload(workbenchState, ['enabled', 'wireframe_enabled', 'show'], true));
          setActiveTool('wireframe', 'inspect');
          setRightDockTab('tool');
          return;
        }
        case 'compare-versions': {
          if (shouldDisableCompareFromWorkbenchPayload(workbenchState)) {
            onRequestCompare(null);
            setReviewPane('compare');
            setRightDockTab('review');
            return;
          }
          const otherVersionId = compareVersionIdFromWorkbenchPayload(workbenchState);
          if (otherVersionId) {
            onCompareVersion(otherVersionId);
            return;
          }
          break;
        }
        case 'version-history': {
          const historyVersionId = versionHistoryVersionIdFromWorkbenchPayload(workbenchState);
          if (historyVersionId) {
            onOpenVersion(historyVersionId);
          }
          setReviewPane('history');
          setRightDockTab('review');
          return;
        }
        case 'restore-branch': {
          const sourceVersionId = sourceVersionIdFromWorkbenchPayload(workbenchState, versionId);
          void branchVersionMutation.mutateAsync({
            versionId: sourceVersionId,
            params: {
              operation_label: stringFromPayload(workbenchState, ['operation_label', 'label']) ?? `Restore Branch from ${sourceVersionId}`,
            },
          }).then((nextVersion) => {
            setVersionId(nextVersion.id);
            setActiveJobId(null);
            setReviewPane('history');
            setRightDockTab('review');
          });
          return;
        }
        case 'section': {
          const selectedRegionIdsFromWorkbench = stringListFromPayload(
            workbenchState,
            ['selected_region_ids', 'region_ids', 'regions_selected', 'regions'],
          );
          const selectedRegionIdFromWorkbench = stringFromPayload(workbenchState, ['selected_region_id', 'region_id', 'region']);
          const sectionAxisFromWorkbench = vectorFromPayloadKeys(workbenchState, ['plane_axis', 'section_axis', 'axis', 'manual_axis']);
          if (sectionAxisFromWorkbench) {
            setResizeAxisMode('manual');
            setManualResizeAxis(normalizeAxis(sectionAxisFromWorkbench));
          }
          setSectionEnabled(booleanFromPayload(workbenchState, ['enabled', 'section_enabled', 'show'], true));
          setSectionConstant(sectionPlaneConstantFromWorkbenchPayload(workbenchState, sectionConstant, sectionAxis));
          if (selectedRegionIdsFromWorkbench.length > 0) {
            setSelectedRegionIds(selectedRegionIdsFromWorkbench);
          }
          if (selectedRegionIdFromWorkbench || selectedRegionIdsFromWorkbench.length > 0) {
            setSelectedRegionId(selectedRegionIdFromWorkbench ?? selectedRegionIdsFromWorkbench[0] ?? selectedRegionId);
          }
          setActiveTool('section', 'inspect');
          setRightDockTab('tool');
          return;
        }
        case 'heatmap':
          if (booleanFromPayload(workbenchState, ['enabled', 'heatmap_enabled', 'show'], true)) {
            setCompareOverlayEnabled(false);
          }
          setHeatmapEnabled(booleanFromPayload(workbenchState, ['enabled', 'heatmap_enabled', 'show'], true));
          setActiveTool('heatmap', 'inspect');
          setRightDockTab('tool');
          return;
        case 'regions': {
          const selectedRegionIdsFromWorkbench = stringListFromPayload(
            workbenchState,
            ['selected_region_ids', 'region_ids', 'regions_selected', 'regions'],
          );
          const selectedRegionIdFromWorkbench = stringFromPayload(workbenchState, ['selected_region_id', 'region_id', 'region']);
          setRegionOverlayEnabled(booleanFromPayload(workbenchState, ['enabled', 'region_overlay_enabled', 'show'], true));
          if (selectedRegionIdsFromWorkbench.length > 0) {
            setSelectedRegionIds(selectedRegionIdsFromWorkbench);
          }
          if (selectedRegionIdFromWorkbench || selectedRegionIdsFromWorkbench.length > 0) {
            setSelectedRegionId(selectedRegionIdFromWorkbench ?? selectedRegionIdsFromWorkbench[0] ?? selectedRegionId);
          }
          setActiveTool('regions', 'inspect');
          setRightDockTab('tool');
          return;
        }
        case 'measure-inspect': {
          setActiveTool('measure-inspect', 'inspect');
          setRightDockTab('tool');
          void measureInspectMutation
            .mutateAsync({ versionId, params: measureInspectRequestFromWorkbenchPayload(workbenchRequest) })
            .then(setMeasureInspectResult);
          return;
        }
        case 'mesh-cut-measure-path': {
          setActiveTool('mesh-cut-measure-path', 'inspect');
          setRightDockTab('tool');
          void meshCutMeasureTopologyMutation.mutateAsync({
            versionId,
            params: meshCutMeasureTopologyRequestFromWorkbenchPayload(workbenchRequest),
          });
          return;
        }
        case 'offset-contours': {
          const request = offsetContoursRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('offset-contours', 'inspect');
          setRightDockTab('tool');
          void offsetContoursMutation
            .mutateAsync({ versionId, params: request })
            .then(setOffsetContoursResult);
          return;
        }
        case 'distance-map-contours': {
          const request = distanceMapContoursRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('distance-map-contours', 'inspect');
          setRightDockTab('tool');
          void distanceMapContoursMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapContoursResult);
          return;
        }
        case 'distance-map-from-mesh': {
          const request = distanceMapFromMeshRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('distance-map-from-mesh', 'inspect');
          setRightDockTab('tool');
          void distanceMapFromMeshMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapFromMeshResult);
          return;
        }
        case 'distance-map-iso-lines': {
          const request = distanceMapIsoLinesRequestFromWorkbenchPayload(
            workbenchRequest,
            distanceMapMergeResult ?? distanceMapTiffImportResult ?? distanceMapFromMeshResult ?? distanceMapContoursResult,
          );
          setActiveTool('distance-map-iso-lines', 'inspect');
          setRightDockTab('tool');
          void distanceMapIsoLinesMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapIsoLinesResult);
          return;
        }
        case 'distance-map-merge': {
          const request = distanceMapMergeRequestFromWorkbenchPayload(
            workbenchRequest,
            distanceMapTiffImportResult ?? distanceMapFromMeshResult ?? distanceMapContoursResult,
          );
          setActiveTool('distance-map-merge', 'inspect');
          setRightDockTab('tool');
          void distanceMapMergeMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapMergeResult);
          return;
        }
        case 'distance-map-contour-boolean': {
          const request = distanceMapContourBooleanRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('distance-map-contour-boolean', 'inspect');
          setRightDockTab('tool');
          void distanceMapContourBooleanMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapContourBooleanResult);
          return;
        }
        case 'distance-map-from-tiff': {
          const request = distanceMapTiffImportRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('distance-map-from-tiff', 'inspect');
          setRightDockTab('tool');
          void distanceMapFromTiffMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapTiffImportResult);
          return;
        }
        case 'distance-map-to-tiff': {
          const request = distanceMapTiffExportRequestFromWorkbenchPayload(
            workbenchRequest,
            distanceMapTiffImportResult ?? distanceMapMergeResult ?? distanceMapFromMeshResult ?? distanceMapContoursResult,
          );
          setActiveTool('distance-map-to-tiff', 'inspect');
          setRightDockTab('tool');
          void distanceMapToTiffMutation
            .mutateAsync({ versionId, params: request })
            .then(setDistanceMapTiffExportResult);
          return;
        }
        case 'object-lines-from-contours': {
          const request = objectLinesFromContoursRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('object-lines-from-contours', 'inspect');
          setRightDockTab('tool');
          void objectLinesFromContoursMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesResult);
          return;
        }
        case 'object-lines-load-mrlines': {
          const request = objectLinesMrLinesLoadRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('object-lines-load-mrlines', 'inspect');
          setRightDockTab('tool');
          void objectLinesLoadMrLinesMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesResult);
          return;
        }
        case 'object-lines-save-mrlines': {
          const request = objectLinesMrLinesSaveRequestFromWorkbenchPayload(
            workbenchRequest,
            objectLinesResult?.object_lines,
          );
          setActiveTool('object-lines-save-mrlines', 'inspect');
          setRightDockTab('tool');
          void objectLinesSaveMrLinesMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesMrLinesExportResult);
          return;
        }
        case 'object-lines-load-ply': {
          const request = objectLinesPlyLoadRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('object-lines-load-ply', 'inspect');
          setRightDockTab('tool');
          void objectLinesLoadPlyMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesResult);
          return;
        }
        case 'object-lines-save-ply': {
          const request = objectLinesPlySaveRequestFromWorkbenchPayload(
            workbenchRequest,
            objectLinesResult?.object_lines,
          );
          setActiveTool('object-lines-save-ply', 'inspect');
          setRightDockTab('tool');
          void objectLinesSavePlyMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesPlyExportResult);
          return;
        }
        case 'object-lines-load-pts': {
          const request = objectLinesPtsLoadRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('object-lines-load-pts', 'inspect');
          setRightDockTab('tool');
          void objectLinesLoadPtsMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesResult);
          return;
        }
        case 'object-lines-save-pts': {
          const request = objectLinesPtsSaveRequestFromWorkbenchPayload(
            workbenchRequest,
            objectLinesResult?.object_lines,
          );
          setActiveTool('object-lines-save-pts', 'inspect');
          setRightDockTab('tool');
          void objectLinesSavePtsMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesPtsExportResult);
          return;
        }
        case 'object-lines-load-svg': {
          const request = objectLinesSvgLoadRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('object-lines-load-svg', 'inspect');
          setRightDockTab('tool');
          void objectLinesLoadSvgMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesResult);
          return;
        }
        case 'object-lines-save-dxf': {
          const request = objectLinesDxfSaveRequestFromWorkbenchPayload(
            workbenchRequest,
            objectLinesResult?.object_lines,
          );
          setActiveTool('object-lines-save-dxf', 'inspect');
          setRightDockTab('tool');
          void objectLinesSaveDxfMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesDxfExportResult);
          return;
        }
        case 'object-lines-to-contours': {
          const request = objectLinesToContoursRequestFromWorkbenchPayload(
            workbenchRequest,
            objectLinesResult?.object_lines,
          );
          setActiveTool('object-lines-to-contours', 'inspect');
          setRightDockTab('tool');
          void objectLinesToContoursMutation
            .mutateAsync({ versionId, params: request })
            .then(setObjectLinesContoursResult);
          return;
        }
        case 'point-cloud-icp': {
          const request = pointCloudIcpRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('point-cloud-icp', 'inspect');
          setRightDockTab('tool');
          void pointCloudIcpMutation
            .mutateAsync({ versionId, params: request })
            .then(setPointCloudIcpResult);
          return;
        }
        case 'gcode-parse-paths': {
          const request = gcodeRequestFromWorkbenchPayload(workbenchRequest, toolDrafts.gcodeSource);
          setActiveTool('gcode-parse-paths', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({ gcodeSource: request.source });
          void gcodeParseMutation
            .mutateAsync({ versionId, params: request })
            .then(setGcodeParseResult);
          return;
        }
        case 'gcode-load-source': {
          const request = gcodeLoadSourceRequestFromWorkbenchPayload(workbenchRequest, toolDrafts.gcodeSource);
          setActiveTool('gcode-parse-paths', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({ gcodeSource: request.source });
          void gcodeLoadSourceMutation.mutateAsync({ versionId, params: request });
          return;
        }
        case 'gcode-write-source': {
          const request = gcodeWriteSourceRequestFromWorkbenchPayload(workbenchRequest, toolDrafts.gcodeSource);
          setActiveTool('gcode-parse-paths', 'inspect');
          setRightDockTab('tool');
          void gcodeWriteSourceMutation.mutateAsync({ versionId, params: request });
          return;
        }
        case 'gcode-parse-file-paths': {
          const request = gcodeParseFilePathsRequestFromWorkbenchPayload(workbenchRequest, toolDrafts.gcodeSource);
          setActiveTool('gcode-parse-paths', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({ gcodeSource: request.source });
          void gcodeParseFilePathsMutation
            .mutateAsync({ versionId, params: request })
            .then(setGcodeParseResult);
          return;
        }
        case 'open-raw-voxels': {
          const request = openRawVoxelsRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('open-raw-voxels', 'inspect');
          setRightDockTab('tool');
          void openRawVoxelsMutation
            .mutateAsync({ versionId, params: request })
            .then(setVoxelLoadResult);
          return;
        }
        case 'open-voxels-from-tiff': {
          const request = openVoxelsFromTiffRequestFromWorkbenchPayload(workbenchRequest);
          setActiveTool('open-voxels-from-tiff', 'inspect');
          setRightDockTab('tool');
          void openVoxelsFromTiffMutation
            .mutateAsync({ versionId, params: request })
            .then(setVoxelLoadResult);
          return;
        }
        case 'mesh-to-voxels-sdf': {
          const request = meshToVoxelsRequestFromWorkbenchPayload(workbenchRequest, {
            voxelSizeMm: toolDrafts.voxelSizeMm,
            voxelSurfaceOffsetVoxels: toolDrafts.voxelSurfaceOffsetVoxels,
            voxelMode: toolDrafts.voxelMode,
            voxelExtractSurface: toolDrafts.voxelExtractSurface,
          });
          setActiveTool('mesh-to-voxels-sdf', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({
            voxelSizeMm: request.voxel_size_mm,
            voxelSurfaceOffsetVoxels: request.surface_offset_voxels,
            voxelMode: request.mode,
            voxelExtractSurface: request.extract_surface,
          });
          void meshToVoxelsMutation
            .mutateAsync({ versionId, params: request })
              .then(setMeshToVoxelsResult);
          return;
        }
        case 'voxel-volume-render-ray': {
          const request = voxelVolumeRenderRayRequestFromWorkbenchPayload(workbenchRequest, {
            rayStart: [toolDrafts.volumeRayStartX, toolDrafts.volumeRayStartY, toolDrafts.volumeRayStartZ],
            rayDirection: [
              toolDrafts.volumeRayDirectionX,
              toolDrafts.volumeRayDirectionY,
              toolDrafts.volumeRayDirectionZ,
            ],
            samplingStep: toolDrafts.volumeRaySamplingStep,
            alphaLimit: toolDrafts.volumeRayAlphaLimit,
            maxSteps: toolDrafts.volumeRayMaxSteps,
          });
          setActiveTool('voxel-volume-render-ray', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({
            volumeRayStartX: request.ray_start[0],
            volumeRayStartY: request.ray_start[1],
            volumeRayStartZ: request.ray_start[2],
            volumeRayDirectionX: request.ray_direction[0],
            volumeRayDirectionY: request.ray_direction[1],
            volumeRayDirectionZ: request.ray_direction[2],
            volumeRaySamplingStep: request.sampling_step,
            volumeRayAlphaLimit: request.alpha_limit ?? toolDrafts.volumeRayAlphaLimit,
            volumeRayMaxSteps: request.max_steps ?? toolDrafts.volumeRayMaxSteps,
          });
          void voxelVolumeRenderRayMutation
            .mutateAsync({ versionId, params: request })
            .then(setVoxelVolumeRenderRayResult);
          return;
        }
        case 'exact-boolean': {
          const request = exactBooleanRequestFromWorkbenchPayload(workbenchRequest, {
            otherVersionId: toolDrafts.booleanTargetVersionId || compareTargetVersionId || '',
            operation: toolDrafts.booleanOperation,
          });
          setActiveTool('exact-boolean', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({
            booleanTargetVersionId: request.other_version_id,
            booleanOperation: request.operation,
          });
          if (request.other_version_id) {
            void exactBooleanMutation
              .mutateAsync({ versionId, params: request })
              .then(activateExactBooleanResult);
          }
          return;
        }
        case 'voxel-boolean': {
          const request = voxelBooleanRequestFromWorkbenchPayload(workbenchRequest, {
            otherVersionId: toolDrafts.voxelBooleanTargetVersionId || compareTargetVersionId || '',
            operation: toolDrafts.voxelBooleanOperation,
            voxelSizeMm: toolDrafts.voxelBooleanSizeMm,
            paddingMm: toolDrafts.voxelBooleanPaddingMm,
            refine: toolDrafts.voxelBooleanRefine,
          });
          setActiveTool('voxel-boolean', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({
            voxelBooleanTargetVersionId: request.other_version_id,
            voxelBooleanOperation: request.operation,
            voxelBooleanSizeMm: request.voxel_size_mm,
            voxelBooleanPaddingMm: request.padding_mm ?? toolDrafts.voxelBooleanPaddingMm,
            voxelBooleanRefine: Boolean(request.refine),
          });
          if (request.other_version_id) {
            void voxelBooleanMutation
              .mutateAsync({ versionId, params: request })
              .then(activateVoxelBooleanResult);
          }
          return;
        }
        case 'collision-detect': {
          const request = collisionRequestFromWorkbenchPayload(workbenchRequest, {
            otherVersionId: toolDrafts.collisionTargetVersionId || compareTargetVersionId || '',
            firstIntersectionOnly: toolDrafts.collisionFirstOnly,
            maxPairs: toolDrafts.collisionMaxPairs,
          });
          setActiveTool('collision-detect', 'inspect');
          setRightDockTab('tool');
          updateToolDrafts({
            collisionTargetVersionId: request.other_version_id,
            collisionFirstOnly: request.first_intersection_only,
            collisionMaxPairs: request.max_pairs ?? toolDrafts.collisionMaxPairs,
          });
          if (request.other_version_id) {
            void collisionMutation
              .mutateAsync({ versionId, params: request })
              .then(setCollisionResult);
          }
          return;
        }
        case 'snapshots': {
          const snapshotName = stringFromPayload(workbenchState, ['snapshot_name', 'name', 'label']);
          const snapshotAction = stringFromPayload(workbenchState, ['snapshot_action', 'action', 'mode', 'operation']);
          const snapshotToLoad = findInspectionSnapshotForWorkbenchPayload(
            workbenchState,
            inspectionSnapshotsQuery.data ?? [],
          );
          const wantsLoad = snapshotAction === 'load' || snapshotAction === 'restore' || snapshotAction === 'open';
          const wantsSave = snapshotAction === 'save' || snapshotAction === 'create' || snapshotAction === 'persist';
          if ((wantsLoad || snapshotToLoad) && snapshotToLoad) {
            onLoadInspection(snapshotToLoad);
            setActiveTool('snapshots', 'inspect');
            setRightDockTab('tool');
            return;
          }
          if (wantsSave || (!wantsLoad && snapshotName)) {
            createInspectionSnapshotMutation.mutate({
              versionId,
              params: inspectionSnapshotStateFromWorkbenchPayload(
                workbenchState,
                currentInspectionSnapshotState(snapshotName ?? 'MeshLib Workbench Snapshot'),
              ),
            });
            setActiveTool('snapshots', 'inspect');
            setRightDockTab('tool');
            return;
          }
          setActiveTool('snapshots', 'inspect');
          setRightDockTab('tool');
          return;
        }
        default:
          break;
      }
    }

    switch (commandId) {
      case 'upload-new':
        router.push('/');
        return;
      case 'download-stl':
        onDownloadStl();
        return;
      case 'export-section':
        onExportSection();
        return;
      case 'compare-versions':
        setRightDockTab('review');
        setReviewPane('compare');
        return;
      case 'version-history':
      case 'restore-branch':
        setRightDockTab('review');
        setReviewPane('history');
        return;
      case 'job-activity':
        setRightDockTab('activity');
        return;
      case 'wireframe':
        setWireframe(activeTool === 'wireframe' ? !wireframe : true);
        break;
      case 'section':
        if (!sectionEnabled) {
          setSectionEnabled(true);
        }
        break;
      case 'heatmap':
        if (compareOverlayEnabled) {
          setCompareOverlayEnabled(false);
        }
        setHeatmapEnabled(activeTool === 'heatmap' ? !heatmapEnabled : true);
        break;
      case 'regions':
        setRegionOverlayEnabled(activeTool === 'regions' ? !regionOverlayEnabled : true);
        break;
      default:
        break;
    }

    if (definition.contextualToolId) {
      setActiveTool(definition.contextualToolId, definition.group);
      setRightDockTab('tool');
    }
  };

  const onWorkbenchHostCommand = (command: WorkbenchHostCommandPayload) => {
    onCommandSelect(command.commandId, {
      payload: command.payload,
      options: command.options,
      endpointUrl: command.endpointUrl,
      endpointUrlKey: command.endpointUrlKey,
      rustBacked: command.rustBacked,
      sdkOperations: command.sdkOperations,
    });
  };

  const renderDockTab = (tab: RightDockTab) => {
    switch (tab) {
      case 'tool':
        return (
          <ToolInspector
            activeTool={activeTool}
            drafts={toolDrafts}
            busy={busy}
            selectedMaterial={selectedMaterial}
            onMaterialChange={setSelectedMaterial}
            updateDrafts={updateToolDrafts}
            selectedRegion={selectedRegion}
            selectedRegionIds={selectedRegionIds}
            regions={viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? []}
            wireframe={wireframe}
            sectionEnabled={sectionEnabled}
            sectionConstant={sectionConstant}
            heatmapEnabled={heatmapEnabled}
            regionOverlayEnabled={regionOverlayEnabled}
            overlay={scalarOverlay}
            sectionContour={activeSectionContour}
            measureInspectResult={measureInspectResult}
            gcodeParseResult={gcodeParseResult}
            pointCloudIcpResult={pointCloudIcpResult}
            offsetContoursResult={offsetContoursResult}
            distanceMapContoursResult={distanceMapContoursResult}
            distanceMapIsoLinesResult={distanceMapIsoLinesResult}
            distanceMapMergeResult={distanceMapMergeResult}
            distanceMapContourBooleanResult={distanceMapContourBooleanResult}
            objectLinesResult={objectLinesResult}
            objectLinesContoursResult={objectLinesContoursResult}
            meshToVoxelsResult={meshToVoxelsResult}
            voxelVolumeRenderRayResult={voxelVolumeRenderRayResult}
            offsetShellResult={offsetShellResult}
            exactBooleanResult={exactBooleanResult}
            voxelBooleanResult={voxelBooleanResult}
            collisionResult={collisionResult}
            sectionPresets={sectionPresets}
            savedSnapshots={inspectionSnapshotsQuery.data ?? []}
            onRepair={onRepair}
            onResize={onResize}
            onHollow={onHollow}
            onThicken={onThicken}
            onScoop={onScoop}
            onSmooth={onSmooth}
            onDecimate={onDecimate}
            onSubdivide={onSubdivide}
            onMakeDelone={onMakeDelone}
            onMeasureInspect={onMeasureInspect}
            onGcodeParse={onGcodeParse}
            onPointCloudIcp={onPointCloudIcp}
            onOffsetContours={onOffsetContours}
            distanceMapFromMeshResult={distanceMapFromMeshResult}
            onDistanceMapFromMesh={onDistanceMapFromMesh}
            onDistanceMapContours={onDistanceMapContours}
            onDistanceMapIsoLines={onDistanceMapIsoLines}
            onDistanceMapMerge={onDistanceMapMerge}
            onDistanceMapContourBoolean={onDistanceMapContourBoolean}
            distanceMapTiffImportResult={distanceMapTiffImportResult}
            distanceMapTiffExportResult={distanceMapTiffExportResult}
            onDistanceMapFromTiff={onDistanceMapFromTiff}
            onDistanceMapToTiff={onDistanceMapToTiff}
            onObjectLinesFromContours={onObjectLinesFromContours}
            objectLinesMrLinesExportResult={objectLinesMrLinesExportResult}
            objectLinesPlyExportResult={objectLinesPlyExportResult}
            objectLinesPtsExportResult={objectLinesPtsExportResult}
            objectLinesDxfExportResult={objectLinesDxfExportResult}
            onObjectLinesLoadMrLines={onObjectLinesLoadMrLines}
            onObjectLinesSaveMrLines={onObjectLinesSaveMrLines}
            onObjectLinesLoadPly={onObjectLinesLoadPly}
            onObjectLinesSavePly={onObjectLinesSavePly}
            onObjectLinesLoadPts={onObjectLinesLoadPts}
            onObjectLinesSavePts={onObjectLinesSavePts}
            onObjectLinesLoadSvg={onObjectLinesLoadSvg}
            onObjectLinesSaveDxf={onObjectLinesSaveDxf}
            onObjectLinesToContours={onObjectLinesToContours}
            onMeshToVoxelsSdf={onMeshToVoxelsSdf}
            voxelLoadResult={voxelLoadResult}
            onOpenRawVoxels={onOpenRawVoxels}
            onOpenVoxelsFromTiff={onOpenVoxelsFromTiff}
            onVoxelVolumeRenderRay={onVoxelVolumeRenderRay}
            onOffsetMesh={onOffsetMesh}
            onShellMesh={onShellMesh}
            onThickenMesh={onThickenMesh}
            onWeightedShell={onWeightedShell}
            onPartialOffset={onPartialOffset}
            onOffsetVerts={onOffsetVerts}
            onExpandShrink={onExpandShrink}
            onShrinkExpand={onShrinkExpand}
            onExactBoolean={onExactBoolean}
            onVoxelBoolean={onVoxelBoolean}
            onCollisionDetect={onCollisionDetect}
            onMakeManufacturable={onMakeManufacturable}
            onWireframeToggle={() => setWireframe(!wireframe)}
            onSectionToggle={() => setSectionEnabled(!sectionEnabled)}
            onHeatmapToggle={() => setHeatmapEnabled(!heatmapEnabled)}
            onRegionOverlayToggle={() => setRegionOverlayEnabled(!regionOverlayEnabled)}
            onSectionConstantChange={setSectionConstant}
            onRegionSelect={setSelectedRegionId}
            onRegionToggle={toggleSelectedRegionId}
            onSnapToRegion={onSnapToRegion}
            onSnapToCenter={onSnapToCenter}
            onApplySectionPreset={onApplySectionPreset}
            onExportSection={onExportSection}
            onSaveSnapshot={onSaveInspection}
            onLoadSnapshot={onLoadInspection}
          />
        );
      case 'review':
        return (
          <ReviewInspector
            reviewPane={reviewPane}
            onReviewPaneChange={setReviewPane}
            versions={versionsQuery.data ?? []}
            currentVersionId={versionId!}
            compareTargetVersionId={compareTargetVersionId}
            compareEnabled={compareOverlayEnabled}
            compareReady={compareOverlayReady}
            onCompareToggle={() => {
              if (!compareOverlayEnabled && !compareOverlayReady) {
                return;
              }
              setCompareOverlayEnabled(!compareOverlayEnabled);
            }}
            onCompareTargetChange={onRequestCompare}
            compareSummary={compareSummary}
            cacheEntries={compareCacheQuery.data ?? []}
            onOpenVersion={onOpenVersion}
            onBranchVersion={onBranchVersion}
            onCompareVersion={onCompareVersion}
            busy={busy || branchVersionMutation.isPending}
          />
        );
      case 'activity':
        return (
          <JobActivityPanel
            events={jobEvents.events}
            job={currentJob.data ?? jobEvents.terminalStatus}
            jobHistory={versionJobsQuery.data ?? []}
          />
        );
      case 'model':
      default:
        return (
          <ModelInspector
            snapshot={snapshotQuery.data ?? null}
            selectedMaterial={selectedMaterial}
            onMaterialChange={setSelectedMaterial}
            axisMode={resizeAxisMode}
            manualAxis={manualResizeAxis}
            onAxisModeChange={setResizeAxisMode}
            onManualAxisChange={setManualResizeAxis}
          />
        );
    }
  };

  if (!modelId || !versionId) {
    return (
      <div className="min-h-screen bg-zinc-950 text-zinc-100 flex items-center justify-center">
        <div className="rounded-2xl border border-zinc-800 bg-zinc-900/80 px-8 py-6">
          <p className="text-sm text-zinc-300">No model version is loaded.</p>
        </div>
      </div>
    );
  }

  return (
    <MeshLibWorkbenchHost
      manifest={workbenchManifestQuery.data ?? null}
      onJobSubmitted={trackWorkbenchJob}
      onWorkspaceCommand={onWorkbenchHostCommand}
    >
      <main className="flex h-screen flex-col overflow-hidden bg-[radial-gradient(circle_at_top,_#1b1b1e,_#09090b_60%)] text-zinc-100">
      <header className="flex shrink-0 items-center justify-between border-b border-zinc-800 bg-zinc-950/80 px-6 py-4 backdrop-blur">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-zinc-500">MeshInspector Production</p>
          <h1 className="text-lg font-semibold text-white">Manufacturing Workspace</h1>
        </div>
        <div className="flex items-center gap-3">
          {activeJobRecord && (
            <div
              className={`rounded-full px-3 py-1 text-xs ${
                activeJobStatus === 'failed'
                  ? 'border border-rose-500/30 bg-rose-500/10 text-rose-200'
                  : 'border border-amber-500/30 bg-amber-500/10 text-amber-200'
              }`}
            >
              {activeJobRecord.operation_type} {activeJobStatus}
              {typeof activeJobRecord.progress_pct === 'number' ? ` ${activeJobRecord.progress_pct}%` : ''}
            </div>
          )}
          <div className="rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs text-zinc-400">
            {activeToolLabel ?? activeToolbarGroup ?? 'model shell'}
          </div>
        </div>
      </header>
      <CommandBar
        activeTool={activeTool}
        openPopoverGroup={openPopoverGroup}
        onGroupOpen={onOpenToolbarGroup}
        onGroupClose={onCloseToolbarGroup}
        onCommandSelect={onCommandSelect}
        getCommandAvailability={getCommandAvailability}
      />

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <section className="relative min-h-0 min-w-0 flex-1 overflow-hidden">
          {previewLowUrl ? (
            <>
              <ViewerEngine
                lowUrl={previewLowUrl}
                highUrl={previewHighUrl}
                wireframe={wireframe}
                sectionEnabled={sectionEnabled}
                sectionConstant={sectionConstant}
                sectionAxis={sectionAxis}
                sectionContour={activeSectionContour}
                normalizedMeshUrl={normalizedMeshUrl}
                regionArtifactUrl={regionArtifactUrl}
                regionOverlayEnabled={regionOverlayEnabled}
                selectedRegionId={selectedRegionId}
                selectedRegionIds={selectedRegionIds}
                scalarOverlay={scalarOverlay}
                textureArtifactUrl={textureArtifactUrl}
                textureMetadata={viewerQuery.data?.texture_metadata ?? {}}
                textureArtifacts={textureArtifacts}
                texturePerFace={viewerQuery.data?.texture_per_face ?? []}
                onRegionPick={onRegionPick}
              />
              <ViewerMetricsHud snapshot={snapshotQuery.data ?? null} material={selectedMaterial} />
            </>
          ) : viewerFailureMessage ? (
            <div className="flex h-full items-center justify-center px-8">
              <div className="max-w-lg rounded-2xl border border-rose-900/60 bg-rose-950/30 px-6 py-5 text-center">
                <p className="text-xs uppercase tracking-[0.22em] text-rose-300/80">Viewer Unavailable</p>
                <p className="mt-3 text-sm text-rose-100">{viewerFailureMessage}</p>
              </div>
            </div>
          ) : (
            <div className="flex h-full items-center justify-center text-zinc-500">Preparing viewer artifacts...</div>
          )}
        </section>

        <aside className="flex h-full w-[360px] shrink-0 flex-col border-l border-zinc-800 bg-zinc-950/75 max-[1439px]:w-[320px]">
          <div className="border-b border-zinc-800 p-2">
            <div className="grid grid-cols-4 gap-2">
              {([
                ['tool', 'Tool'],
                ['model', 'Model'],
                ['review', 'Review'],
                ['activity', 'Activity'],
              ] as Array<[RightDockTab, string]>).map(([tab, label]) => (
                <button
                  key={tab}
                  onClick={() => setRightDockTab(tab)}
                  className={`rounded-xl px-2 py-2 text-sm transition-colors ${
                    rightDockTab === tab
                      ? 'bg-zinc-100 text-zinc-950'
                      : 'bg-zinc-900 text-zinc-300 hover:bg-zinc-800'
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto p-4">{renderDockTab(rightDockTab)}</div>
        </aside>
      </div>

      <StatusStrip
        currentVersionId={versionId!}
        activeToolLabel={activeToolLabel}
        material={selectedMaterial}
        selectedRegionCount={selectedRegionIds.length}
        overlays={activeOverlays}
        job={activeJobRecord}
      />
      </main>
    </MeshLibWorkbenchHost>
  );
}

function ViewerMetricsHud({
  snapshot,
  material,
}: {
  snapshot: ManufacturabilitySnapshot | null;
  material: MaterialType;
}) {
  if (!snapshot) {
    return null;
  }

  const { bbox_mm, estimated_ring_size_us, inner_diameter_mm, head_height_mm } = snapshot.dimensions;
  const materialWeight = snapshot.material_weight[material];

  return (
    <div className="pointer-events-none absolute left-4 top-4 z-20 max-w-xs rounded-2xl border border-zinc-800/90 bg-zinc-950/85 px-4 py-3 shadow-[0_18px_48px_rgba(0,0,0,0.35)] backdrop-blur">
      <p className="text-[10px] uppercase tracking-[0.24em] text-zinc-500">Viewport Metrics</p>
      <div className="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-sm text-zinc-200">
        <Metric label="X" value={`${bbox_mm[0].toFixed(2)} mm`} />
        <Metric label="Y" value={`${bbox_mm[1].toFixed(2)} mm`} />
        <Metric label="Z" value={`${bbox_mm[2].toFixed(2)} mm`} />
        <Metric label="Min T" value={snapshot.thickness.min_mm != null ? `${snapshot.thickness.min_mm.toFixed(2)} mm` : 'n/a'} />
        <Metric label="Ring US" value={estimated_ring_size_us != null ? estimated_ring_size_us.toFixed(2) : 'n/a'} />
        <Metric label="Inner ID" value={inner_diameter_mm != null ? `${inner_diameter_mm.toFixed(2)} mm` : 'n/a'} />
        <Metric label="Head H" value={head_height_mm != null ? `${head_height_mm.toFixed(2)} mm` : 'n/a'} />
        <Metric label="Weight" value={materialWeight ? `${materialWeight.weight_g.toFixed(2)} g` : 'n/a'} />
      </div>
      <p className="mt-3 text-[11px] text-zinc-500">
        Axis gizmo: bottom-left. Units: millimeters.
      </p>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[10px] uppercase tracking-[0.18em] text-zinc-500">{label}</p>
      <p className="mt-1 font-medium text-zinc-100">{value}</p>
    </div>
  );
}

export default function ViewerPage() {
  return (
    <Suspense fallback={<div className="min-h-screen bg-zinc-950" />}>
      <ViewerPageContent />
    </Suspense>
  );
}

function getSelectedRegionOperationAvailability(
  selectedRegion: RegionManifestEntry | null,
  operation: string,
  label: string,
): CommandAvailability {
  if (!selectedRegion) {
    return { disabled: true, reason: 'Select a primary region first.' };
  }
  if (!selectedRegion.allowed_operations.includes(operation)) {
    return {
      disabled: true,
      reason: `${selectedRegion.label} does not allow ${label}.`,
    };
  }
  return { disabled: false };
}

function getBatchRegionOperationAvailability(
  regions: RegionManifestEntry[],
  selectedRegionIds: string[],
  operation: string,
  label: string,
): CommandAvailability {
  if (selectedRegionIds.length < 2) {
    return { disabled: true, reason: 'Batch commands require at least 2 selected regions.' };
  }
  const selectedRegions = selectedRegionIds
    .map((regionId) => regions.find((region) => region.region_id === regionId) ?? null)
    .filter((region): region is RegionManifestEntry => region !== null);
  if (selectedRegions.length !== selectedRegionIds.length) {
    return { disabled: true, reason: 'One or more selected regions is no longer available.' };
  }
  const blockedRegion = selectedRegions.find((region) => !region.allowed_operations.includes(operation));
  if (blockedRegion) {
    return { disabled: true, reason: `${blockedRegion.label} does not allow ${label.toLowerCase()}.` };
  }
  return { disabled: false };
}

function getScoopCommandAvailability(
  regions: RegionManifestEntry[],
  selectedRegionId: string | null,
  scoopDepth: number,
  keepMinThickness: number,
) {
  const requiredThickness = scoopDepth + keepMinThickness;
  const selected = regions.find((region) => region.region_id === selectedRegionId) ?? null;
  const candidates = regions.filter((region) => region.allowed_operations?.includes('scoop') && region.vertex_count > 0);

  if (selected?.allowed_operations?.includes('scoop')) {
    if (selected.min_thickness_mm == null || selected.min_thickness_mm >= requiredThickness) {
      return { disabled: false };
    }
    return {
      disabled: true,
      reason:
        `Selected region ${selected.label} is too thin for the current scoop depth and minimum thickness. ` +
        'Thicken it first or reduce scoop depth.',
    };
  }

  if (candidates.some((region) => region.min_thickness_mm == null || region.min_thickness_mm >= requiredThickness)) {
    return { disabled: false };
  }

  if (candidates.length > 0) {
    return {
      disabled: true,
      reason: 'No scoop-safe region can support the current scoop depth and minimum thickness.',
    };
  }

  return {
    disabled: true,
    reason: 'No scoop-safe region is available on this mesh.',
  };
}
