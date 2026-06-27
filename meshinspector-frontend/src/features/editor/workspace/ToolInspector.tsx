'use client';

import type { ReactNode } from 'react';
import type {
  CollisionDetectRequest,
  CollisionDetectResponse,
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
  GcodeParsePathsRequest,
  GcodeParsePathsResponse,
  HollowRequestV2,
  InspectionSnapshotResponse,
  IsoLineSegmentsResponse,
  MakeDeloneRequestV2,
  MakeManufacturableRequest,
  MaterialType,
  MeasureInspectRequest,
  MeasureInspectResponse,
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
  ScalarOverlayResponse,
  ScoopRequestV2,
  SectionContourPayload,
  ShellMeshRequest,
  SmoothRequestV2,
  SubdivideRequestV2,
  ThickenMeshRequest,
  ThickenRequestV2,
  VoxelBooleanRequest,
  VoxelBooleanResponse,
  VoxelRawLoadRequest,
  VoxelTiffLoadRequest,
  VoxelVolumeLoadResponse,
  VoxelVolumeRenderRayRequest,
  VoxelVolumeRenderRayResponse,
  WeightedShellRequest,
} from '@/lib/api/types';
import { MATERIALS } from '@/lib/constants';
import type { ContextToolId, ToolDrafts } from './types';

type SectionPreset = {
  id: string;
  label: string;
  description: string;
};

const ICP_REFERENCE_POINTS: Array<[number, number, number]> = [
  [0, 0, 0],
  [10, 0, 0],
  [0, 10, 0],
  [0, 0, 10],
  [8, 8, 8],
];

const ICP_FLOATING_POINTS: Array<[number, number, number]> = ICP_REFERENCE_POINTS.map(
  ([x, y, z]) => [x + 0.25, y - 0.1, z + 0.05],
);

const OFFSET_CONTOUR_POINTS: Array<Array<[number, number, number]>> = [
  [
    [0, 0, 0],
    [2, 0, 0],
    [2, 2, 0],
    [0, 2, 0],
  ],
];

const DISTANCE_MAP_CONTOUR_POINTS: Array<Array<[number, number]>> = [
  [
    [0, 0],
    [2, 0],
    [2, 2],
    [0, 2],
    [0, 0],
  ],
];
const DISTANCE_MAP_CONTOUR_POINTS_B: Array<Array<[number, number]>> = [
  [
    [1, 0],
    [3, 0],
    [3, 2],
    [1, 2],
    [1, 0],
  ],
];

const DISTANCE_MAP_VALUES = [
  [-1, 1],
  [-1, 1],
];
const DISTANCE_MAP_TIFF_VALUES = [
  [1, 2],
  [3, 4],
];
const DISTANCE_MAP_TIFF_SAMPLE_BASE64 =
  'SUkqAL8AAAABAAAAAQAAAAEAAAABAAAALTMuNDAyODIzNDY2Mzg1Mjg4NmUzOAAAAAAAAAAEQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAkQAAAAAAAAAAAAAAAAAAAEEAAAAAAAAAAAAAAAAAAADRAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPA/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADwPwAAgD8AAABAAABAQAAAgEAQAAABBAABAAAAAgAAAAEBBAABAAAAAgAAAAIBAwABAAAAIAAAAAMBAwABAAAAAQAAAAYBAwABAAAAAAAAABEBBAABAAAArwAAABUBAwABAAAAAQAAABYBBAABAAAASOgBABcBBAABAAAAEAAAABoBBQABAAAACAAAABsBBQABAAAAEAAAACgBAwABAAAAAQAAAD0BAwABAAAAAQAAAFMBAwABAAAAAwAAANiFDAAQAAAALwAAAIGkAgAXAAAAGAAAAAAAAAA=';
const RAW_VOXELS_BASE64 = 'AAAAgP//AEA=';
const TIFF_VOXEL_SLICE_10_BASE64 =
  'SUkqAAgAAAAKAAABBAABAAAAAgAAAAEBBAABAAAAAQAAAAIBAwABAAAAIAAAAAMBAwABAAAAAQAAAAYBAwABAAAAAQAAABEBBAABAAAAhgAAABYBBAABAAAAAQAAABcBBAABAAAACAAAABwBAwABAAAAAQAAAFMBAwABAAAAAwAAAAAAAAAAACBBAAAwQQ==';
const TIFF_VOXEL_SLICE_02_BASE64 =
  'SUkqAAgAAAAKAAABBAABAAAAAgAAAAEBBAABAAAAAQAAAAIBAwABAAAAIAAAAAMBAwABAAAAAQAAAAYBAwABAAAAAQAAABEBBAABAAAAhgAAABYBBAABAAAAAQAAABcBBAABAAAACAAAABwBAwABAAAAAQAAAFMBAwABAAAAAwAAAAAAAAAAAABAAABAQA==';
const DISTANCE_MAP_INVALID_VALUE = -3.4028234663852886e38;
const DISTANCE_MAP_MERGE_RIGHT_VALUES = [
  [3, 5],
  [DISTANCE_MAP_INVALID_VALUE, 6],
];

const OBJECT_LINES_CONTOUR_POINTS: Array<Array<[number, number, number]>> = [
  [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
  ],
];

const OBJECT_LINES_PAYLOAD: Record<string, unknown> = {
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
const OBJECT_LINES_PTS_SOURCE =
  'BEGIN_Polyline\n' +
  '0 0 0\n' +
  '1.25 0 0\n' +
  '1.25 1.5 0\n' +
  'END_Polyline\n' +
  'BEGIN_Polyline\n' +
  '2 -1 0.5\n' +
  '3 -1 0.5\n' +
  'END_Polyline\n';
const OBJECT_LINES_SVG_SOURCE =
  '<svg xmlns="http://www.w3.org/2000/svg">' +
  '<line x1="1" y1="2" x2="4" y2="6" />' +
  '<polyline points="0,0 2,0 2,2" />' +
  '</svg>';
const OBJECT_LINES_MRLINES_BASE64 =
  'AgAAAAAAAAAAAAAAAQAAAAEAAAACAAAAAAAAAAEAAAADAAAAAgAAAAAAAAAAAAAAAAAAAAAAgD8AAABAAABAQA==';
const OBJECT_LINES_PLY_BASE64 =
  'cGx5CmZvcm1hdCBiaW5hcnlfbGl0dGxlX2VuZGlhbiAxLjAKY29tbWVudCBNZXNoSW5zcGVjdG9yLmNvbQplbGVtZW50IHZlcnRleCAyCnByb3BlcnR5IGZsb2F0IHgKcHJvcGVydHkgZmxvYXQgeQpwcm9wZXJ0eSBmbG9hdCB6CmVsZW1lbnQgZWRnZSAxCnByb3BlcnR5IGludCB2ZXJ0ZXgxCnByb3BlcnR5IGludCB2ZXJ0ZXgyCmVuZF9oZWFkZXIKAAAAAAAAAAAAAAAAAACAPwAAAEAAAEBAAAAAAAEAAAA=';

export default function ToolInspector({
  activeTool,
  drafts,
  busy,
  selectedMaterial,
  onMaterialChange,
  updateDrafts,
  selectedRegion,
  selectedRegionIds,
  regions,
  wireframe,
  sectionEnabled,
  sectionConstant,
  heatmapEnabled,
  regionOverlayEnabled,
  overlay,
  sectionContour,
  measureInspectResult,
  gcodeParseResult,
  pointCloudIcpResult,
  offsetContoursResult,
  distanceMapFromMeshResult,
  distanceMapContoursResult,
  distanceMapIsoLinesResult,
  distanceMapMergeResult,
  distanceMapContourBooleanResult,
  distanceMapTiffImportResult,
  distanceMapTiffExportResult,
  objectLinesResult,
  objectLinesContoursResult,
  objectLinesMrLinesExportResult,
  objectLinesPlyExportResult,
  objectLinesPtsExportResult,
  objectLinesDxfExportResult,
  meshToVoxelsResult,
  voxelLoadResult,
  voxelVolumeRenderRayResult,
  offsetShellResult,
  exactBooleanResult,
  voxelBooleanResult,
  collisionResult,
  sectionPresets,
  savedSnapshots,
  onRepair,
  onResize,
  onHollow,
  onThicken,
  onScoop,
  onSmooth,
  onDecimate,
  onSubdivide,
  onMakeDelone,
  onMeasureInspect,
  onGcodeParse,
  onPointCloudIcp,
  onOffsetContours,
  onDistanceMapFromMesh,
  onDistanceMapContours,
  onDistanceMapIsoLines,
  onDistanceMapMerge,
  onDistanceMapContourBoolean,
  onDistanceMapFromTiff,
  onDistanceMapToTiff,
  onObjectLinesFromContours,
  onObjectLinesLoadMrLines,
  onObjectLinesSaveMrLines,
  onObjectLinesLoadPly,
  onObjectLinesSavePly,
  onObjectLinesLoadPts,
  onObjectLinesSavePts,
  onObjectLinesLoadSvg,
  onObjectLinesSaveDxf,
  onObjectLinesToContours,
  onMeshToVoxelsSdf,
  onOpenRawVoxels,
  onOpenVoxelsFromTiff,
  onVoxelVolumeRenderRay,
  onOffsetMesh,
  onShellMesh,
  onThickenMesh,
  onWeightedShell,
  onPartialOffset,
  onOffsetVerts,
  onExpandShrink,
  onShrinkExpand,
  onExactBoolean,
  onVoxelBoolean,
  onCollisionDetect,
  onMakeManufacturable,
  onWireframeToggle,
  onSectionToggle,
  onHeatmapToggle,
  onRegionOverlayToggle,
  onSectionConstantChange,
  onRegionSelect,
  onRegionToggle,
  onSnapToRegion,
  onSnapToCenter,
  onApplySectionPreset,
  onExportSection,
  onSaveSnapshot,
  onLoadSnapshot,
}: {
  activeTool: ContextToolId | null;
  drafts: ToolDrafts;
  busy: boolean;
  selectedMaterial: MaterialType;
  onMaterialChange: (value: MaterialType) => void;
  updateDrafts: (value: Partial<ToolDrafts>) => void;
  selectedRegion: RegionManifestEntry | null;
  selectedRegionIds: string[];
  regions: RegionManifestEntry[];
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  heatmapEnabled: boolean;
  regionOverlayEnabled: boolean;
  overlay: ScalarOverlayResponse | null;
  sectionContour: SectionContourPayload | null;
  measureInspectResult: MeasureInspectResponse | null;
  gcodeParseResult: GcodeParsePathsResponse | null;
  pointCloudIcpResult: PointCloudIcpResponse | null;
  offsetContoursResult: OffsetContoursResponse | null;
  distanceMapFromMeshResult: DistanceMapResponse | null;
  distanceMapContoursResult: DistanceMapResponse | null;
  distanceMapIsoLinesResult: IsoLineSegmentsResponse | null;
  distanceMapMergeResult: DistanceMapResponse | null;
  distanceMapContourBooleanResult: IsoLineSegmentsResponse | null;
  distanceMapTiffImportResult: DistanceMapResponse | null;
  distanceMapTiffExportResult: DistanceMapTiffExportResponse | null;
  objectLinesResult: ObjectLinesResponse | null;
  objectLinesContoursResult: ObjectLinesToContoursResponse | null;
  objectLinesMrLinesExportResult: ObjectLinesBinaryExportResponse | null;
  objectLinesPlyExportResult: ObjectLinesBinaryExportResponse | null;
  objectLinesPtsExportResult: ObjectLinesTextExportResponse | null;
  objectLinesDxfExportResult: ObjectLinesTextExportResponse | null;
  meshToVoxelsResult: MeshToVoxelsSdfResponse | null;
  voxelLoadResult: VoxelVolumeLoadResponse | null;
  voxelVolumeRenderRayResult: VoxelVolumeRenderRayResponse | null;
  offsetShellResult: OffsetShellMeshResponse | null;
  exactBooleanResult: ExactBooleanResponse | null;
  voxelBooleanResult: VoxelBooleanResponse | null;
  collisionResult: CollisionDetectResponse | null;
  sectionPresets: SectionPreset[];
  savedSnapshots: InspectionSnapshotResponse[];
  onRepair: () => void;
  onResize: (request: ResizeRequestV2) => void;
  onHollow: (request: HollowRequestV2) => void;
  onThicken: (request: ThickenRequestV2) => void;
  onScoop: (request: ScoopRequestV2) => void;
  onSmooth: (request: SmoothRequestV2) => void;
  onDecimate: (request: DecimateRequestV2) => void;
  onSubdivide: (request: SubdivideRequestV2) => void;
  onMakeDelone: (request: MakeDeloneRequestV2) => void;
  onMeasureInspect: (request: MeasureInspectRequest) => void;
  onGcodeParse: (request: GcodeParsePathsRequest) => void;
  onPointCloudIcp: (request: PointCloudIcpRequest) => void;
  onOffsetContours: (request: OffsetContoursRequest) => void;
  onDistanceMapFromMesh: (request: DistanceMapFromMeshRequest) => void;
  onDistanceMapContours: (request: DistanceMapContoursRequest) => void;
  onDistanceMapIsoLines: (request: DistanceMapIsoLinesRequest) => void;
  onDistanceMapMerge: (request: DistanceMapMergeRequest) => void;
  onDistanceMapContourBoolean: (request: DistanceMapContourBooleanRequest) => void;
  onDistanceMapFromTiff: (request: DistanceMapTiffImportRequest) => void;
  onDistanceMapToTiff: (request: DistanceMapTiffExportRequest) => void;
  onObjectLinesFromContours: (request: ObjectLinesFromContoursRequest) => void;
  onObjectLinesLoadMrLines: (request: ObjectLinesBinaryLoadRequest) => void;
  onObjectLinesSaveMrLines: (request: ObjectLinesBinaryExportRequest) => void;
  onObjectLinesLoadPly: (request: ObjectLinesBinaryLoadRequest) => void;
  onObjectLinesSavePly: (request: ObjectLinesBinaryExportRequest) => void;
  onObjectLinesLoadPts: (request: ObjectLinesPtsLoadRequest) => void;
  onObjectLinesSavePts: (request: ObjectLinesTextExportRequest) => void;
  onObjectLinesLoadSvg: (request: ObjectLinesSvgLoadRequest) => void;
  onObjectLinesSaveDxf: (request: ObjectLinesTextExportRequest) => void;
  onObjectLinesToContours: (request: ObjectLinesToContoursRequest) => void;
  onMeshToVoxelsSdf: (request: MeshToVoxelsSdfRequest) => void;
  onOpenRawVoxels: (request: VoxelRawLoadRequest) => void;
  onOpenVoxelsFromTiff: (request: VoxelTiffLoadRequest) => void;
  onVoxelVolumeRenderRay: (request: VoxelVolumeRenderRayRequest) => void;
  onOffsetMesh: (request: OffsetMeshRequest) => void;
  onShellMesh: (request: ShellMeshRequest) => void;
  onThickenMesh: (request: ThickenMeshRequest) => void;
  onWeightedShell: (request: WeightedShellRequest) => void;
  onPartialOffset: (request: PartialOffsetRequest) => void;
  onOffsetVerts: (request: OffsetVertsRequest) => void;
  onExpandShrink: (request: OffsetSmoothingRequest) => void;
  onShrinkExpand: (request: OffsetSmoothingRequest) => void;
  onExactBoolean: (request: ExactBooleanRequest) => void;
  onVoxelBoolean: (request: VoxelBooleanRequest) => void;
  onCollisionDetect: (request: CollisionDetectRequest) => void;
  onMakeManufacturable: (request: MakeManufacturableRequest) => void;
  onWireframeToggle: () => void;
  onSectionToggle: () => void;
  onHeatmapToggle: () => void;
  onRegionOverlayToggle: () => void;
  onSectionConstantChange: (value: number) => void;
  onRegionSelect: (regionId: string) => void;
  onRegionToggle: (regionId: string) => void;
  onSnapToRegion: () => void;
  onSnapToCenter: () => void;
  onApplySectionPreset: (presetId: string) => void;
  onExportSection: () => void;
  onSaveSnapshot: (name: string) => void;
  onLoadSnapshot: (snapshot: InspectionSnapshotResponse) => void;
}) {
  const batchRegions = regions.filter((region) => selectedRegionIds.includes(region.region_id));
  const weightedShellRegionIds = selectedRegionIds.length > 0 ? selectedRegionIds : selectedRegion ? [selectedRegion.region_id] : [];
  const partialOffsetRegionIds = weightedShellRegionIds;
  const offsetVertsRegionIds = weightedShellRegionIds;
  const scoopEligibility = getScoopEligibility(regions, selectedRegion, drafts.scoopDepth, drafts.minThickness);
  const scoopRegion = scoopEligibility.region;

  if (!activeTool) {
    return (
      <EmptyState
        title="No Tool Selected"
        body="Choose a command from the top toolbar. Geometry tools open here with persistent settings so repeated operations stay in the same place."
      />
    );
  }

  switch (activeTool) {
    case 'repair':
      return (
        <ToolCard
          eyebrow="Prepare"
          title="Auto Repair"
          description="Heal holes, degeneracies, and manufacturability blockers before editing."
          footer={
            <ActionFooter
              busy={busy}
              disabled={false}
              label="Create Repaired Version"
              onClick={onRepair}
            />
          }
        />
      );
    case 'fit-size':
      return (
        <ToolCard
          eyebrow="Prepare"
          title="Fit To Size"
          description="Resize to a target ring size while preserving ornament-heavy regions."
        >
          <NumberField
            label="Target Ring Size"
            value={drafts.targetRingSize}
            min={3}
            max={15}
            step={0.5}
            onChange={(value) => updateDrafts({ targetRingSize: value })}
          />
          <MaterialField material={selectedMaterial} onMaterialChange={onMaterialChange} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Create Sized Version"
            onClick={() => onResize({ target_ring_size_us: drafts.targetRingSize, axis_mode: 'auto', preserve_head: true })}
          />
        </ToolCard>
      );
    case 'reduce-weight':
      return (
        <ToolCard
          eyebrow="Prepare"
          title="Reduce Weight"
          description="Target a weight class using protected hollowing while keeping detailed regions thicker."
        >
          <MaterialField material={selectedMaterial} onMaterialChange={onMaterialChange} />
          <InlineToggle
            label="Full Resolution Batch"
            enabled={drafts.hollowFullResolution}
            onClick={() => updateDrafts({ hollowFullResolution: !drafts.hollowFullResolution })}
          />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Target Weight"
              value={drafts.targetWeight}
              min={0.5}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ targetWeight: value })}
            />
            <NumberField
              label="Min Thickness"
              value={drafts.minThickness}
              min={0.2}
              max={5}
              step={0.05}
              onChange={(value) => updateDrafts({ minThickness: value })}
            />
          </div>
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Create Weight-Reduced Version"
            onClick={() =>
              onHollow({
                mode: 'target_weight',
                processing_mode: drafts.hollowFullResolution ? 'full_resolution' : 'interactive',
                material: selectedMaterial,
                target_weight_g: drafts.targetWeight,
                min_allowed_thickness_mm: drafts.minThickness,
                protect_regions: ['head', 'gem_seat', 'ornament_relief'],
                add_drain_holes: false,
              })
            }
          />
        </ToolCard>
      );
    case 'prepare-casting':
      return (
        <ToolCard
          eyebrow="Prepare"
          title="Prepare For Casting"
          description="Build a protected hollow shell and add drain holes through the inner band."
        >
          <MaterialField material={selectedMaterial} onMaterialChange={onMaterialChange} />
          <InlineToggle
            label="Full Resolution Batch"
            enabled={drafts.hollowFullResolution}
            onClick={() => updateDrafts({ hollowFullResolution: !drafts.hollowFullResolution })}
          />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Wall Thickness"
              value={drafts.wallThickness}
              min={0.3}
              max={5}
              step={0.05}
              onChange={(value) => updateDrafts({ wallThickness: value })}
            />
            <NumberField
              label="Min Thickness"
              value={drafts.minThickness}
              min={0.2}
              max={5}
              step={0.05}
              onChange={(value) => updateDrafts({ minThickness: value })}
            />
          </div>
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Create Castable Version"
            onClick={() =>
              onHollow({
                mode: 'fixed_thickness',
                processing_mode: drafts.hollowFullResolution ? 'full_resolution' : 'interactive',
                material: selectedMaterial,
                wall_thickness_mm: drafts.wallThickness,
                min_allowed_thickness_mm: drafts.minThickness,
                protect_regions: ['head', 'gem_seat', 'ornament_relief'],
                add_drain_holes: true,
              })
            }
          />
        </ToolCard>
      );
    case 'make-manufacturable':
      return (
        <ToolCard
          eyebrow="Prepare"
          title="Make Manufacturable"
          description="Run the guided pipeline: repair, size, optimize, and validate."
        >
          <MaterialField material={selectedMaterial} onMaterialChange={onMaterialChange} />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Target Size"
              value={drafts.targetRingSize}
              min={3}
              max={15}
              step={0.5}
              onChange={(value) => updateDrafts({ targetRingSize: value })}
            />
            <NumberField
              label="Target Weight"
              value={drafts.targetWeight}
              min={0.5}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ targetWeight: value })}
            />
          </div>
          <NumberField
            label="Min Thickness"
            value={drafts.minThickness}
            min={0.2}
            max={5}
            step={0.05}
            onChange={(value) => updateDrafts({ minThickness: value })}
          />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Run Manufacturing Pipeline"
            onClick={() =>
              onMakeManufacturable({
                material: selectedMaterial,
                target_ring_size_us: drafts.targetRingSize,
                target_weight_g: drafts.targetWeight,
                min_allowed_thickness_mm: drafts.minThickness,
              })
            }
          />
        </ToolCard>
      );
    case 'resize':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Resize"
          description="Create a resized version while preserving ornament-heavy regions."
        >
          <NumberField
            label="Resize To"
            value={drafts.resizeTargetSize}
            min={3}
            max={15}
            step={0.5}
            onChange={(value) => updateDrafts({ resizeTargetSize: value })}
          />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Create Resized Version"
            onClick={() => onResize({ target_ring_size_us: drafts.resizeTargetSize, axis_mode: 'auto', preserve_head: true })}
          />
        </ToolCard>
      );
    case 'protected-hollow':
      return (
        <HollowTool
          title="Protected Hollow"
          busy={busy}
          selectedMaterial={selectedMaterial}
          wallThickness={drafts.wallThickness}
          minThickness={drafts.minThickness}
          fullResolution={drafts.hollowFullResolution}
          onMaterialChange={onMaterialChange}
          onDraftChange={updateDrafts}
          onApply={() =>
            onHollow({
              mode: 'fixed_thickness',
              processing_mode: drafts.hollowFullResolution ? 'full_resolution' : 'interactive',
              material: selectedMaterial,
              wall_thickness_mm: drafts.wallThickness,
              min_allowed_thickness_mm: drafts.minThickness,
              protect_regions: ['head', 'gem_seat', 'ornament_relief'],
              add_drain_holes: false,
            })
          }
          actionLabel="Create Hollow Version"
        />
      );
    case 'hollow-drains':
      return (
        <HollowTool
          title="Hollow + Drains"
          busy={busy}
          selectedMaterial={selectedMaterial}
          wallThickness={drafts.wallThickness}
          minThickness={drafts.minThickness}
          fullResolution={drafts.hollowFullResolution}
          onMaterialChange={onMaterialChange}
          onDraftChange={updateDrafts}
          onApply={() =>
            onHollow({
              mode: 'fixed_thickness',
              processing_mode: drafts.hollowFullResolution ? 'full_resolution' : 'interactive',
              material: selectedMaterial,
              wall_thickness_mm: drafts.wallThickness,
              min_allowed_thickness_mm: drafts.minThickness,
              protect_regions: ['head', 'gem_seat', 'ornament_relief'],
              add_drain_holes: true,
            })
          }
          actionLabel="Create Hollow + Drain Version"
        />
      );
    case 'thicken-violations':
      return (
        <ToolCard eyebrow="Modify" title="Thicken Violations" description="Only thicken unsafe regions below the minimum target.">
          <NumberField
            label="Thicken To"
            value={drafts.thickenTarget}
            min={0.3}
            max={5}
            step={0.05}
            onChange={(value) => updateDrafts({ thickenTarget: value })}
          />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Create Thickened Version"
            onClick={() =>
              onThicken({
                mode: 'violations_only',
                min_target_thickness_mm: drafts.thickenTarget,
                smoothing_pass: true,
              })
            }
          />
        </ToolCard>
      );
    case 'thicken-region': {
      const reason = getSelectedRegionOperationReason(selectedRegion, 'thicken', 'thickening');
      return (
        <ToolCard eyebrow="Modify" title="Thicken Region" description="Apply a localized thickening pass to the primary region.">
          <RegionSummary region={selectedRegion} fallback="No primary region selected." />
          <NumberField
            label="Thicken To"
            value={drafts.thickenTarget}
            min={0.3}
            max={5}
            step={0.05}
            onChange={(value) => updateDrafts({ thickenTarget: value })}
          />
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <ActionFooter
            busy={busy}
            disabled={!!reason}
            disabledReason={reason ?? undefined}
            label="Create Region Thickening"
            onClick={() =>
              selectedRegion &&
              onThicken({
                mode: 'selected_region',
                region_id: selectedRegion.region_id,
                min_target_thickness_mm: drafts.thickenTarget,
                smoothing_pass: true,
              })
            }
          />
        </ToolCard>
      );
    }
    case 'batch-thicken': {
      const reason = getBatchRegionOperationReason(batchRegions, selectedRegionIds, 'thicken', 'batch thickening');
      return (
        <ToolCard eyebrow="Modify" title="Batch Thicken" description="Apply one localized thickening pass across all selected regions.">
          <NumberField
            label="Thicken To"
            value={drafts.thickenTarget}
            min={0.3}
            max={5}
            step={0.05}
            onChange={(value) => updateDrafts({ thickenTarget: value })}
          />
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <ActionFooter
            busy={busy}
            disabled={!!reason}
            disabledReason={reason ?? undefined}
            label="Create Batch Thickening"
            onClick={() =>
              onThicken({
                mode: 'selected_regions',
                region_ids: batchRegions.map((region) => region.region_id),
                min_target_thickness_mm: drafts.thickenTarget,
                smoothing_pass: true,
              })
            }
          />
        </ToolCard>
      );
    }
    case 'scoop': {
      const reason = scoopEligibility.reason;
      return (
        <ToolCard eyebrow="Modify" title="Scoop" description="Carve a controlled recess into a scoop-safe region while enforcing minimum thickness.">
          <RegionSummary region={scoopRegion} fallback="No scoop-safe region available." />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Depth"
              value={drafts.scoopDepth}
              min={0.05}
              max={5}
              step={0.05}
              onChange={(value) => updateDrafts({ scoopDepth: value })}
            />
            <NumberField
              label="Falloff"
              value={drafts.scoopFalloff}
              min={0.1}
              max={10}
              step={0.1}
              onChange={(value) => updateDrafts({ scoopFalloff: value })}
            />
          </div>
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <ActionFooter
            busy={busy}
            disabled={!!reason}
            disabledReason={reason ?? undefined}
            label="Create Scoop Version"
            onClick={() =>
              scoopRegion &&
              onScoop({
                region_id: scoopRegion.region_id,
                depth_mm: drafts.scoopDepth,
                falloff_mm: drafts.scoopFalloff,
                keep_min_thickness_mm: drafts.minThickness,
              })
            }
          />
        </ToolCard>
      );
    }
    case 'smooth': {
      const reason = selectedRegion ? getSelectedRegionOperationReason(selectedRegion, 'smooth', 'smoothing') : null;
      return (
        <ToolCard
          eyebrow="Modify"
          title="Smooth"
          description="Smooth the current primary region or the entire model when no primary region is selected."
        >
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Iterations"
              value={drafts.smoothIterations}
              min={1}
              max={50}
              step={1}
              onChange={(value) => updateDrafts({ smoothIterations: value })}
            />
            <NumberField
              label="Strength"
              value={drafts.smoothStrength}
              min={0.01}
              max={1}
              step={0.01}
              onChange={(value) => updateDrafts({ smoothStrength: value })}
            />
          </div>
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <ActionFooter
            busy={busy}
            disabled={!!reason}
            disabledReason={reason ?? undefined}
            label={selectedRegion ? `Smooth ${selectedRegion.label}` : 'Smooth Entire Surface'}
            onClick={() =>
              onSmooth({
                region_id: selectedRegion?.region_id,
                iterations: drafts.smoothIterations,
                strength: drafts.smoothStrength,
                global_mode: !selectedRegion,
              })
            }
          />
        </ToolCard>
      );
    }
    case 'batch-smooth': {
      const reason = getBatchRegionOperationReason(batchRegions, selectedRegionIds, 'smooth', 'batch smoothing');
      return (
        <ToolCard eyebrow="Modify" title="Batch Smooth" description="Smooth all selected regions with one localized pass.">
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Iterations"
              value={drafts.smoothIterations}
              min={1}
              max={50}
              step={1}
              onChange={(value) => updateDrafts({ smoothIterations: value })}
            />
            <NumberField
              label="Strength"
              value={drafts.smoothStrength}
              min={0.01}
              max={1}
              step={0.01}
              onChange={(value) => updateDrafts({ smoothStrength: value })}
            />
          </div>
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <ActionFooter
            busy={busy}
            disabled={!!reason}
            disabledReason={reason ?? undefined}
            label="Create Batch Smooth Version"
            onClick={() =>
              onSmooth({
                region_ids: batchRegions.map((region) => region.region_id),
                iterations: drafts.smoothIterations,
                strength: drafts.smoothStrength,
                global_mode: false,
              })
            }
          />
        </ToolCard>
      );
    }
    case 'decimate-mesh':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Decimate Mesh"
          description="Simplify meshes with Rust-backed MeshLib DecimateStrategy::MinimizeError QEM or ShortestEdgeFirst."
        >
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Strategy
            <select
              value={drafts.decimateStrategy}
              onChange={(event) =>
                updateDrafts({ decimateStrategy: event.target.value as DecimateRequestV2['strategy'] })
              }
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 outline-none"
            >
              <option value="minimize_error">Minimize Error</option>
              <option value="shortest_edge_first">Shortest Edge First</option>
            </select>
          </label>
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Max Error"
              value={drafts.decimateMaxError}
              min={0}
              max={25}
              step={0.01}
              onChange={(value) => updateDrafts({ decimateMaxError: value })}
            />
            <NumberField
              label="Target Faces"
              value={drafts.decimateTargetFaces}
              min={0}
              max={10000000}
              step={1}
              onChange={(value) => updateDrafts({ decimateTargetFaces: Math.max(0, Math.round(value)) })}
            />
            <NumberField
              label="Target %"
              value={drafts.decimateTargetPercent}
              min={0}
              max={100}
              step={1}
              onChange={(value) => updateDrafts({ decimateTargetPercent: Math.max(0, Math.min(100, value)) })}
            />
            <NumberField
              label="Max Edge Len"
              value={drafts.decimateMaxEdgeLen}
              min={0}
              max={100}
              step={0.01}
              onChange={(value) => updateDrafts({ decimateMaxEdgeLen: value })}
            />
            <NumberField
              label="Max Boundary Shift"
              value={drafts.decimateMaxBoundaryShift}
              min={0}
              max={100}
              step={0.01}
              onChange={(value) => updateDrafts({ decimateMaxBoundaryShift: value })}
            />
            <NumberField
              label="Stabilizer"
              value={drafts.decimateStabilizer}
              min={0}
              max={1}
              step={0.001}
              onChange={(value) => updateDrafts({ decimateStabilizer: Math.max(0, value) })}
            />
          </div>
          <InlineToggle
            label="Parallel Algorithm"
            enabled={drafts.decimateParallelAlgorithm}
            onClick={() => updateDrafts({ decimateParallelAlgorithm: !drafts.decimateParallelAlgorithm })}
          />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Max Verts"
              value={drafts.decimateMaxDeletedVertices}
              min={1}
              max={100000}
              step={1}
              onChange={(value) => updateDrafts({ decimateMaxDeletedVertices: Math.max(1, Math.round(value)) })}
            />
            <NumberField
              label="Max Faces"
              value={drafts.decimateMaxDeletedFaces}
              min={1}
              max={200000}
              step={1}
              onChange={(value) => updateDrafts({ decimateMaxDeletedFaces: Math.max(1, Math.round(value)) })}
            />
            <NumberField
              label="Subdivide Parts"
              value={drafts.decimateSubdivideParts}
              min={1}
              max={128}
              step={1}
              onChange={(value) => updateDrafts({ decimateSubdivideParts: Math.max(1, Math.round(value)) })}
            />
          </div>
          <NumberField
            label="Max Tri Aspect"
            value={drafts.decimateMaxTriangleAspectRatio}
            min={1}
            max={100}
            step={0.1}
            onChange={(value) => updateDrafts({ decimateMaxTriangleAspectRatio: Math.max(1, value) })}
          />
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Region Faces
            <input
              value={drafts.decimateRegionFaces}
              onChange={(event) => updateDrafts({ decimateRegionFaces: event.target.value })}
              placeholder="0, 4, 12"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Not Flippable Edges
            <input
              value={drafts.decimateNotFlippableEdges}
              onChange={(event) => updateDrafts({ decimateNotFlippableEdges: event.target.value })}
              placeholder="1-3, 2-4"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <InlineToggle
            label="Collapse Near Protected"
            enabled={drafts.decimateCollapseNearNotFlippable}
            onClick={() => updateDrafts({ decimateCollapseNearNotFlippable: !drafts.decimateCollapseNearNotFlippable })}
          />
          <InlineToggle
            label="Angle Weighted Planes"
            enabled={drafts.decimateAngleWeightedDistToPlane}
            onClick={() =>
              updateDrafts({ decimateAngleWeightedDistToPlane: !drafts.decimateAngleWeightedDistToPlane })
            }
          />
          <InlineToggle
            label="Touch Boundary Edges"
            enabled={drafts.decimateTouchNearBoundaryEdges}
            onClick={() =>
              updateDrafts({ decimateTouchNearBoundaryEdges: !drafts.decimateTouchNearBoundaryEdges })
            }
          />
          <InlineToggle
            label="Touch Boundary Verts"
            enabled={drafts.decimateTouchBoundaryVerts}
            onClick={() => updateDrafts({ decimateTouchBoundaryVerts: !drafts.decimateTouchBoundaryVerts })}
          />
          <InlineToggle
            label="Optimize Vertex"
            enabled={drafts.decimateOptimizeVertexPos}
            onClick={() => updateDrafts({ decimateOptimizeVertexPos: !drafts.decimateOptimizeVertexPos })}
          />
          <InlineToggle
            label="Pack Mesh"
            enabled={drafts.decimatePackMesh}
            onClick={() => updateDrafts({ decimatePackMesh: !drafts.decimatePackMesh })}
          />
          <ActionFooter
            busy={busy}
            disabled={drafts.decimateMaxError < 0}
            label="Create Decimated Version"
            onClick={() => {
              const targetFaceCount =
                drafts.decimateTargetFaces > 0 ? Math.max(1, Math.round(drafts.decimateTargetFaces)) : null;
              const targetFaceRatio =
                targetFaceCount === null && drafts.decimateTargetPercent > 0
                  ? Math.min(1, drafts.decimateTargetPercent / 100)
                  : null;
              onDecimate({
                strategy: drafts.decimateStrategy,
                max_error: drafts.decimateMaxError,
                target_face_count: targetFaceCount,
                target_face_ratio: targetFaceRatio,
                max_edge_len: drafts.decimateMaxEdgeLen > 0 ? drafts.decimateMaxEdgeLen : null,
                max_bd_shift: drafts.decimateMaxBoundaryShift > 0 ? drafts.decimateMaxBoundaryShift : null,
                stabilizer: Math.max(0, drafts.decimateStabilizer),
                subdivide_parts: drafts.decimateParallelAlgorithm
                  ? Math.max(2, Math.round(drafts.decimateSubdivideParts))
                  : 1,
                decimate_between_parts: true,
                region_faces: parseIndexList(drafts.decimateRegionFaces),
                not_flippable_edges: parseEdgePairs(drafts.decimateNotFlippableEdges),
                collapse_near_not_flippable: drafts.decimateCollapseNearNotFlippable,
                angle_weighted_dist_to_plane: drafts.decimateAngleWeightedDistToPlane,
                max_deleted_vertices: Math.max(1, Math.round(drafts.decimateMaxDeletedVertices)),
                max_deleted_faces: Math.max(1, Math.round(drafts.decimateMaxDeletedFaces)),
                max_triangle_aspect_ratio: Math.max(1, drafts.decimateMaxTriangleAspectRatio),
                touch_near_bd_edges: drafts.decimateTouchNearBoundaryEdges,
                touch_bd_verts: drafts.decimateTouchBoundaryVerts,
                optimize_vertex_pos: drafts.decimateOptimizeVertexPos,
                pack_mesh: drafts.decimatePackMesh,
              });
            }}
          />
        </ToolCard>
      );
    case 'subdivide-mesh':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Subdivide Mesh"
          description="Split long edges with MeshLib-style subdivision, projection, and smooth-mode settings."
        >
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Max Edge Len"
              value={drafts.subdivideMaxEdgeLen}
              min={0.001}
              max={25}
              step={0.01}
              onChange={(value) => updateDrafts({ subdivideMaxEdgeLen: value })}
            />
            <NumberField
              label="Max Splits"
              value={drafts.subdivideMaxEdgeSplits}
              min={1}
              max={100000}
              step={1}
              onChange={(value) => updateDrafts({ subdivideMaxEdgeSplits: Math.max(1, Math.round(value)) })}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Curvature"
              value={drafts.subdivideCurvaturePriority}
              min={0}
              max={20}
              step={0.1}
              onChange={(value) => updateDrafts({ subdivideCurvaturePriority: value })}
            />
            <NumberField
              label="Sharp Angle"
              value={drafts.subdivideMinSharpDihedralAngle}
              min={0.001}
              max={3.1416}
              step={0.01}
              onChange={(value) => updateDrafts({ subdivideMinSharpDihedralAngle: value })}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Tri Aspect"
              value={drafts.subdivideMaxTriAspectRatio}
              min={0}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ subdivideMaxTriAspectRatio: value })}
            />
            <NumberField
              label="Split Aspect"
              value={drafts.subdivideMaxSplittableTriAspectRatio}
              min={0}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ subdivideMaxSplittableTriAspectRatio: value })}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Flip Deviation"
              value={drafts.subdivideMaxDeviationAfterFlip}
              min={0}
              max={25}
              step={0.01}
              onChange={(value) => updateDrafts({ subdivideMaxDeviationAfterFlip: value })}
            />
            <NumberField
              label="Flip Angle"
              value={drafts.subdivideMaxAngleChangeAfterFlip}
              min={0}
              max={6.2832}
              step={0.01}
              onChange={(value) => updateDrafts({ subdivideMaxAngleChangeAfterFlip: value })}
            />
            <NumberField
              label="Critical Aspect"
              value={drafts.subdivideCriticalTriAspectRatioFlip}
              min={0}
              max={1000}
              step={0.1}
              onChange={(value) => updateDrafts({ subdivideCriticalTriAspectRatioFlip: value })}
            />
          </div>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Region Faces
            <input
              value={drafts.subdivideRegionFaces}
              onChange={(event) => updateDrafts({ subdivideRegionFaces: event.target.value })}
              placeholder="0, 4, 12"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Not Flippable Edges
            <input
              value={drafts.subdivideNotFlippableEdges}
              onChange={(event) => updateDrafts({ subdivideNotFlippableEdges: event.target.value })}
              placeholder="1-3, 2-4"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <InlineToggle
            label="Subdivide Border"
            enabled={drafts.subdivideBorder}
            onClick={() => updateDrafts({ subdivideBorder: !drafts.subdivideBorder })}
          />
          <InlineToggle
            label="Project Original"
            enabled={drafts.subdivideProjectOnOriginalMesh}
            onClick={() => updateDrafts({ subdivideProjectOnOriginalMesh: !drafts.subdivideProjectOnOriginalMesh })}
          />
          <InlineToggle
            label="Smooth Mode"
            enabled={drafts.subdivideSmoothMode}
            onClick={() => updateDrafts({ subdivideSmoothMode: !drafts.subdivideSmoothMode })}
          />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Create Subdivided Version"
            onClick={() =>
              onSubdivide({
                max_edge_len: drafts.subdivideMaxEdgeLen,
                max_edge_splits: Math.max(1, Math.round(drafts.subdivideMaxEdgeSplits)),
                subdivide_border: drafts.subdivideBorder,
                curvature_priority: drafts.subdivideCurvaturePriority,
                project_on_original_mesh: drafts.subdivideProjectOnOriginalMesh,
                smooth_mode: drafts.subdivideSmoothMode,
                min_sharp_dihedral_angle: drafts.subdivideMinSharpDihedralAngle,
                max_tri_aspect_ratio: drafts.subdivideMaxTriAspectRatio,
                max_splittable_tri_aspect_ratio:
                  drafts.subdivideMaxSplittableTriAspectRatio > 0 ? drafts.subdivideMaxSplittableTriAspectRatio : null,
                max_deviation_after_flip:
                  drafts.subdivideMaxDeviationAfterFlip > 0 ? drafts.subdivideMaxDeviationAfterFlip : null,
                max_angle_change_after_flip:
                  drafts.subdivideMaxAngleChangeAfterFlip > 0 ? drafts.subdivideMaxAngleChangeAfterFlip : null,
                critical_tri_aspect_ratio_flip:
                  drafts.subdivideCriticalTriAspectRatioFlip > 0 ? drafts.subdivideCriticalTriAspectRatioFlip : null,
                region_faces: parseIndexList(drafts.subdivideRegionFaces),
                not_flippable_edges: parseEdgePairs(drafts.subdivideNotFlippableEdges),
              })
            }
          />
        </ToolCard>
      );
    case 'make-delone':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Make Delone"
          description="Flip local mesh diagonals with MeshLib MR::makeDeloneEdgeFlips semantics."
        >
          <NumberField
            label="Iterations"
            value={drafts.makeDeloneNumIters}
            min={1}
            max={1000}
            step={1}
            onChange={(value) => updateDrafts({ makeDeloneNumIters: Math.max(1, Math.round(value)) })}
          />
          <NumberField
            label="Max Deviation After Flip"
            value={drafts.makeDeloneMaxDeviationAfterFlip}
            min={0}
            max={1000}
            step={0.01}
            onChange={(value) => updateDrafts({ makeDeloneMaxDeviationAfterFlip: Math.max(0, value) })}
          />
          <NumberField
            label="Max Angle Change"
            value={drafts.makeDeloneMaxAngleChange}
            min={0}
            max={6.283185307179586}
            step={0.01}
            onChange={(value) => updateDrafts({ makeDeloneMaxAngleChange: Math.max(0, value) })}
          />
          <NumberField
            label="Critical Tri Aspect Ratio"
            value={drafts.makeDeloneCriticalTriAspectRatio}
            min={0}
            max={1000}
            step={0.1}
            onChange={(value) => updateDrafts({ makeDeloneCriticalTriAspectRatio: Math.max(0, value) })}
          />
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Region Faces
            <input
              value={drafts.makeDeloneRegionFaces}
              onChange={(event) => updateDrafts({ makeDeloneRegionFaces: event.target.value })}
              placeholder="0, 4, 12"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Not Flippable Edges
            <input
              value={drafts.makeDeloneNotFlippableEdges}
              onChange={(event) => updateDrafts({ makeDeloneNotFlippableEdges: event.target.value })}
              placeholder="0-2, 4-7"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Vertex Region
            <input
              value={drafts.makeDeloneVertRegion}
              onChange={(event) => updateDrafts({ makeDeloneVertRegion: event.target.value })}
              placeholder="1, 3, 8"
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <ActionFooter
            busy={busy}
            disabled={drafts.makeDeloneNumIters < 1}
            label="Create Delone Version"
            onClick={() =>
              onMakeDelone({
                num_iters: Math.max(1, Math.round(drafts.makeDeloneNumIters)),
                max_deviation_after_flip:
                  drafts.makeDeloneMaxDeviationAfterFlip > 0 ? drafts.makeDeloneMaxDeviationAfterFlip : null,
                max_angle_change: drafts.makeDeloneMaxAngleChange > 0 ? drafts.makeDeloneMaxAngleChange : null,
                critical_tri_aspect_ratio:
                  drafts.makeDeloneCriticalTriAspectRatio > 0 ? drafts.makeDeloneCriticalTriAspectRatio : null,
                region_faces: parseIndexList(drafts.makeDeloneRegionFaces),
                not_flippable_edges: parseEdgePairs(drafts.makeDeloneNotFlippableEdges),
                vert_region: parseIndexList(drafts.makeDeloneVertRegion),
              })
            }
          />
        </ToolCard>
      );
    case 'offset-mesh':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Offset Mesh"
          description="Create an entire-model MeshLib generalOffsetMesh-style child version through the Rust voxel offset kernel."
        >
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Offset"
              value={drafts.offsetDistanceMm}
              min={-20}
              max={20}
              step={0.01}
              onChange={(value) => updateDrafts({ offsetDistanceMm: value })}
            />
            <NumberField
              label="Voxel Size"
              value={drafts.offsetVoxelSizeMm}
              min={0.01}
              max={10}
              step={0.05}
              onChange={(value) => updateDrafts({ offsetVoxelSizeMm: value })}
            />
          </div>
          <NumberField
            label="Padding"
            value={drafts.offsetPaddingMm}
            min={0.01}
            max={50}
            step={0.1}
            onChange={(value) => updateDrafts({ offsetPaddingMm: value })}
          />
          <InlineToggle
            label="Refine Surface"
            enabled={drafts.offsetRefine}
            onClick={() => updateDrafts({ offsetRefine: !drafts.offsetRefine })}
          />
          <OffsetShellReadout result={offsetShellResult} />
          <ActionFooter
            busy={busy}
            disabled={drafts.offsetDistanceMm === 0 || drafts.offsetVoxelSizeMm <= 0 || drafts.offsetPaddingMm <= 0}
            label="Create Offset Version"
            onClick={() =>
              onOffsetMesh({
                offset_mm: drafts.offsetDistanceMm,
                voxel_size_mm: drafts.offsetVoxelSizeMm,
                padding_mm: drafts.offsetPaddingMm,
                refine: drafts.offsetRefine,
              })
            }
          />
        </ToolCard>
      );
    case 'shell-mesh':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Shell Mesh"
          description="Create an official Offset tool shell-mode child version through the Rust voxel shell kernel."
        >
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Wall Thickness"
              value={drafts.shellWallThicknessMm}
              min={0.01}
              max={20}
              step={0.01}
              onChange={(value) => updateDrafts({ shellWallThicknessMm: value })}
            />
            <NumberField
              label="Voxel Size"
              value={drafts.shellVoxelSizeMm}
              min={0.01}
              max={10}
              step={0.05}
              onChange={(value) => updateDrafts({ shellVoxelSizeMm: value })}
            />
          </div>
          <NumberField
            label="Padding"
            value={drafts.shellPaddingMm}
            min={0.01}
            max={50}
            step={0.1}
            onChange={(value) => updateDrafts({ shellPaddingMm: value })}
          />
          <InlineToggle
            label="Refine Surface"
            enabled={drafts.shellRefine}
            onClick={() => updateDrafts({ shellRefine: !drafts.shellRefine })}
          />
          <OffsetShellReadout result={offsetShellResult} />
          <ActionFooter
            busy={busy}
            disabled={drafts.shellWallThicknessMm <= 0 || drafts.shellVoxelSizeMm <= 0 || drafts.shellPaddingMm <= 0}
            label="Create Shell Version"
            onClick={() =>
              onShellMesh({
                wall_thickness_mm: drafts.shellWallThicknessMm,
                voxel_size_mm: drafts.shellVoxelSizeMm,
                padding_mm: drafts.shellPaddingMm,
                refine: drafts.shellRefine,
              })
            }
          />
        </ToolCard>
      );
    case 'thicken-mesh':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Thickening"
          description="Create an official MeshLib thickenMesh child version through the Rust voxel thickening kernel."
        >
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Thickness"
              value={drafts.thickenMeshThicknessMm}
              min={-20}
              max={20}
              step={0.01}
              onChange={(value) => updateDrafts({ thickenMeshThicknessMm: value })}
            />
            <NumberField
              label="Voxel Size"
              value={drafts.thickenMeshVoxelSizeMm}
              min={0.01}
              max={10}
              step={0.05}
              onChange={(value) => updateDrafts({ thickenMeshVoxelSizeMm: value })}
            />
          </div>
          <NumberField
            label="Padding"
            value={drafts.thickenMeshPaddingMm}
            min={0.01}
            max={50}
            step={0.1}
            onChange={(value) => updateDrafts({ thickenMeshPaddingMm: value })}
          />
          <InlineToggle
            label="Refine Surface"
            enabled={drafts.thickenMeshRefine}
            onClick={() => updateDrafts({ thickenMeshRefine: !drafts.thickenMeshRefine })}
          />
          <OffsetShellReadout result={offsetShellResult} />
          <ActionFooter
            busy={busy}
            disabled={
              drafts.thickenMeshThicknessMm === 0 ||
              drafts.thickenMeshVoxelSizeMm <= 0 ||
              drafts.thickenMeshPaddingMm <= 0
            }
            label="Create Thickening Version"
            onClick={() =>
              onThickenMesh({
                thickness_mm: drafts.thickenMeshThicknessMm,
                voxel_size_mm: drafts.thickenMeshVoxelSizeMm,
                padding_mm: drafts.thickenMeshPaddingMm,
                refine: drafts.thickenMeshRefine,
              })
            }
          />
        </ToolCard>
      );
    case 'weighted-shell':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Weighted Shell"
          description="Create a MeshLib WeightedShell child version with additive offsets on selected regions."
        >
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Base Offset"
              value={drafts.weightedShellOffsetMm}
              min={-20}
              max={20}
              step={0.01}
              onChange={(value) => updateDrafts({ weightedShellOffsetMm: value })}
            />
            <NumberField
              label="Region Weight"
              value={drafts.weightedShellRegionWeightMm}
              min={-20}
              max={20}
              step={0.01}
              onChange={(value) => updateDrafts({ weightedShellRegionWeightMm: value })}
            />
            <NumberField
              label="Interpolation"
              value={drafts.weightedShellInterpolationMm}
              min={0}
              max={50}
              step={0.1}
              onChange={(value) => updateDrafts({ weightedShellInterpolationMm: value })}
            />
            <NumberField
              label="Voxel Size"
              value={drafts.weightedShellVoxelSizeMm}
              min={0.01}
              max={10}
              step={0.05}
              onChange={(value) => updateDrafts({ weightedShellVoxelSizeMm: value })}
            />
          </div>
          <NumberField
            label="Padding"
            value={drafts.weightedShellPaddingMm}
            min={0.01}
            max={50}
            step={0.1}
            onChange={(value) => updateDrafts({ weightedShellPaddingMm: value })}
          />
          <InlineToggle
            label="Refine Surface"
            enabled={drafts.weightedShellRefine}
            onClick={() => updateDrafts({ weightedShellRefine: !drafts.weightedShellRefine })}
          />
          <OffsetShellReadout result={offsetShellResult} />
          <ActionFooter
            busy={busy}
            disabled={
              weightedShellRegionIds.length === 0 ||
              drafts.weightedShellRegionWeightMm === 0 ||
              drafts.weightedShellVoxelSizeMm <= 0 ||
              drafts.weightedShellPaddingMm <= 0 ||
              drafts.weightedShellInterpolationMm < 0
            }
            disabledReason={weightedShellRegionIds.length === 0 ? 'Select at least one region.' : undefined}
            label="Create Weighted Shell"
            onClick={() =>
              onWeightedShell({
                offset_mm: drafts.weightedShellOffsetMm,
                region_weights: weightedShellRegionIds.map((regionId) => ({
                  region_id: regionId,
                  weight_mm: drafts.weightedShellRegionWeightMm,
                })),
                voxel_size_mm: drafts.weightedShellVoxelSizeMm,
                padding_mm: drafts.weightedShellPaddingMm,
                interpolation_distance_mm: drafts.weightedShellInterpolationMm,
                refine: drafts.weightedShellRefine,
              })
            }
          />
        </ToolCard>
      );
    case 'partial-offset':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Partial Offset"
          description="Offset selected regions with MeshLib partialOffsetMesh unsigned-part semantics and union the result with the source mesh."
        >
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label="Offset"
              value={drafts.partialOffsetDistanceMm}
              min={0.01}
              max={20}
              step={0.01}
              onChange={(value) => updateDrafts({ partialOffsetDistanceMm: value })}
            />
            <NumberField
              label="Voxel Size"
              value={drafts.partialOffsetVoxelSizeMm}
              min={0.01}
              max={10}
              step={0.05}
              onChange={(value) => updateDrafts({ partialOffsetVoxelSizeMm: value })}
            />
          </div>
          <NumberField
            label="Padding"
            value={drafts.partialOffsetPaddingMm}
            min={0.01}
            max={50}
            step={0.1}
            onChange={(value) => updateDrafts({ partialOffsetPaddingMm: value })}
          />
          <InlineToggle
            label="Refine Surface"
            enabled={drafts.partialOffsetRefine}
            onClick={() => updateDrafts({ partialOffsetRefine: !drafts.partialOffsetRefine })}
          />
          <OffsetShellReadout result={offsetShellResult} />
          <ActionFooter
            busy={busy}
            disabled={
              partialOffsetRegionIds.length === 0 ||
              drafts.partialOffsetDistanceMm <= 0 ||
              drafts.partialOffsetVoxelSizeMm <= 0 ||
              drafts.partialOffsetPaddingMm <= 0
            }
            disabledReason={partialOffsetRegionIds.length === 0 ? 'Select at least one region.' : undefined}
            label="Create Partial Offset"
            onClick={() =>
              onPartialOffset({
                offset_mm: drafts.partialOffsetDistanceMm,
                region_ids: partialOffsetRegionIds,
                voxel_size_mm: drafts.partialOffsetVoxelSizeMm,
                padding_mm: drafts.partialOffsetPaddingMm,
                refine: drafts.partialOffsetRefine,
              })
            }
          />
        </ToolCard>
      );
    case 'offset-verts':
      return (
        <ToolCard
          eyebrow="Modify"
          title="Offset Verts"
          description="Shift vertices along MeshLib pseudonormals with per-vertex offset metrics."
        >
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <NumberField
            label="Offset"
            value={drafts.offsetVertsDistanceMm}
            min={-20}
            max={20}
            step={0.01}
            onChange={(value) => updateDrafts({ offsetVertsDistanceMm: value })}
          />
          <OffsetShellReadout result={offsetShellResult} />
          <ActionFooter
            busy={busy}
            disabled={drafts.offsetVertsDistanceMm === 0}
            label="Create Offset Verts"
            onClick={() =>
              onOffsetVerts({
                offset_mm: drafts.offsetVertsDistanceMm,
                region_ids: offsetVertsRegionIds,
              })
            }
          />
        </ToolCard>
      );
    case 'expand-shrink':
      return (
        <OffsetSmoothingTool
          title="Expand/Shrink"
          description="Smooth concave features by offsetting outward and then inward with the Rust voxel offset kernel."
          busy={busy}
          distance={drafts.expandShrinkDistanceMm}
          voxelSize={drafts.expandShrinkVoxelSizeMm}
          padding={drafts.expandShrinkPaddingMm}
          refine={drafts.expandShrinkRefine}
          result={offsetShellResult}
          actionLabel="Create Expand/Shrink Version"
          onDistanceChange={(value) => updateDrafts({ expandShrinkDistanceMm: value })}
          onVoxelSizeChange={(value) => updateDrafts({ expandShrinkVoxelSizeMm: value })}
          onPaddingChange={(value) => updateDrafts({ expandShrinkPaddingMm: value })}
          onRefineToggle={() => updateDrafts({ expandShrinkRefine: !drafts.expandShrinkRefine })}
          onApply={() =>
            onExpandShrink({
              distance_mm: drafts.expandShrinkDistanceMm,
              voxel_size_mm: drafts.expandShrinkVoxelSizeMm,
              padding_mm: drafts.expandShrinkPaddingMm,
              refine: drafts.expandShrinkRefine,
            })
          }
        />
      );
    case 'shrink-expand':
      return (
        <OffsetSmoothingTool
          title="Shrink/Expand"
          description="Smooth convex features by offsetting inward and then outward with the Rust voxel offset kernel."
          busy={busy}
          distance={drafts.shrinkExpandDistanceMm}
          voxelSize={drafts.shrinkExpandVoxelSizeMm}
          padding={drafts.shrinkExpandPaddingMm}
          refine={drafts.shrinkExpandRefine}
          result={offsetShellResult}
          actionLabel="Create Shrink/Expand Version"
          onDistanceChange={(value) => updateDrafts({ shrinkExpandDistanceMm: value })}
          onVoxelSizeChange={(value) => updateDrafts({ shrinkExpandVoxelSizeMm: value })}
          onPaddingChange={(value) => updateDrafts({ shrinkExpandPaddingMm: value })}
          onRefineToggle={() => updateDrafts({ shrinkExpandRefine: !drafts.shrinkExpandRefine })}
          onApply={() =>
            onShrinkExpand({
              distance_mm: drafts.shrinkExpandDistanceMm,
              voxel_size_mm: drafts.shrinkExpandVoxelSizeMm,
              padding_mm: drafts.shrinkExpandPaddingMm,
              refine: drafts.shrinkExpandRefine,
            })
          }
        />
      );
    case 'section':
      return (
        <ToolCard eyebrow="Inspect" title="Section" description="Slice the model along the active axis, inspect contour dimensions, and export SVG.">
          <InlineToggle label="Section Plane" enabled={sectionEnabled} onClick={onSectionToggle} />
          <div>
            <div className="flex items-center justify-between gap-3 text-sm text-zinc-300">
              <span>Plane Offset</span>
              <span>{sectionEnabled ? `${sectionConstant.toFixed(1)} mm` : 'Off'}</span>
            </div>
            <input
              type="range"
              min={-40}
              max={40}
              step={0.5}
              value={sectionConstant}
              onChange={(event) => onSectionConstantChange(Number(event.target.value))}
              className="mt-3 w-full accent-amber-400"
            />
          </div>
          <div className="flex flex-wrap gap-2">
            <SecondaryButton label="Snap Center" onClick={onSnapToCenter} />
            <SecondaryButton label="Snap Region" onClick={onSnapToRegion} disabled={!selectedRegion?.centroid_mm} />
            <SecondaryButton
              label="Export SVG"
              onClick={onExportSection}
              disabled={!sectionEnabled || !sectionContour?.segments.length || !sectionContour.projected_bounds_min || !sectionContour.projected_bounds_max}
            />
          </div>
          {sectionPresets.length ? (
            <div className="space-y-2">
              <p className="text-[11px] uppercase tracking-[0.18em] text-zinc-500">Presets</p>
              {sectionPresets.map((preset) => (
                <button
                  key={preset.id}
                  onClick={() => onApplySectionPreset(preset.id)}
                  className="w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-left hover:bg-zinc-900"
                >
                  <p className="text-sm text-zinc-100">{preset.label}</p>
                  <p className="mt-1 text-xs text-zinc-500">{preset.description}</p>
                </button>
              ))}
            </div>
          ) : null}
          {sectionContour ? <SectionReadout sectionContour={sectionContour} /> : null}
          <ActionFooter
            busy={busy}
            disabled={false}
            label={sectionEnabled ? 'Disable Section' : 'Enable Section'}
            onClick={onSectionToggle}
          />
        </ToolCard>
      );
    case 'heatmap':
      return (
        <ToolCard eyebrow="Inspect" title="Heatmap" description="Visualize wall thickness as a scalar overlay in the viewport.">
          <InlineToggle label="Thickness Overlay" enabled={heatmapEnabled} onClick={onHeatmapToggle} />
          <OverlayReadout overlay={overlay} fallback="Enable the heatmap to inspect scalar ranges." />
          <ActionFooter
            busy={busy}
            disabled={false}
            label={heatmapEnabled ? 'Hide Heatmap' : 'Show Heatmap'}
            onClick={onHeatmapToggle}
          />
        </ToolCard>
      );
    case 'regions':
      return (
        <ToolCard eyebrow="Inspect" title="Regions" description="Review region coverage, thickness, and operation eligibility.">
          <InlineToggle label="Region Overlay" enabled={regionOverlayEnabled} onClick={onRegionOverlayToggle} />
          <RegionPicker
            regions={regions}
            selectedRegionIds={selectedRegionIds}
            selectedRegion={selectedRegion}
            onRegionSelect={onRegionSelect}
            onRegionToggle={onRegionToggle}
          />
          <RegionSummary region={selectedRegion} fallback="Select a region from the list or by clicking the mesh." />
          <ActionFooter
            busy={busy}
            disabled={false}
            label={regionOverlayEnabled ? 'Hide Regions' : 'Show Regions'}
            onClick={onRegionOverlayToggle}
          />
        </ToolCard>
      );
    case 'mesh-cut-measure-path':
    case 'measure-inspect': {
      const regionPoint = selectedRegion?.centroid_mm ?? null;
      const probePoint: [number, number, number] = regionPoint ?? [0, 0, 0];
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Measure / Inspect"
          description="Probe closest-surface distance, point pairs, and local thickness through the Rust-backed SDK."
        >
          <RegionSummary region={selectedRegion} fallback="No primary region selected for centroid probing." />
          <div className="flex flex-wrap gap-2">
            <SecondaryButton
              label="Probe Region"
              onClick={() => {
                if (!regionPoint) return;
                onMeasureInspect({
                  points: [regionPoint],
                  point_pairs: [],
                  include_local_thickness: true,
                });
              }}
              disabled={!regionPoint}
            />
            <SecondaryButton
              label="Probe Origin"
              onClick={() =>
                onMeasureInspect({
                  points: [[0, 0, 0]],
                  point_pairs: [],
                  include_local_thickness: true,
                })
              }
            />
          </div>
          <MeasureInspectReadout result={measureInspectResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label={regionPoint ? 'Measure Selected Region' : 'Measure Origin'}
            onClick={() =>
              onMeasureInspect({
                points: [probePoint],
                point_pairs: [],
                include_local_thickness: true,
              })
            }
          />
        </ToolCard>
      );
    }
    case 'gcode-parse-paths':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="G-code Path Parser"
          description="Parse CNC source into MeshLib GcodeProcessor-style path segments through the Rust-backed SDK."
        >
          <textarea
            value={drafts.gcodeSource}
            onChange={(event) => updateDrafts({ gcodeSource: event.target.value })}
            spellCheck={false}
            className="min-h-40 w-full resize-y rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 font-mono text-xs leading-5 text-zinc-100 outline-none placeholder:text-zinc-500"
            placeholder="G90&#10;G21&#10;G1 X10 Y0 Z5 F1200"
          />
          <GcodeParseReadout result={gcodeParseResult} />
          <ActionFooter
            busy={busy}
            disabled={!drafts.gcodeSource.trim()}
            label="Parse Toolpath"
            onClick={() =>
              onGcodeParse({
                source: drafts.gcodeSource,
                machine_settings: null,
              })
            }
          />
        </ToolCard>
      );
    case 'offset-contours':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Offset Contours"
          description="Apply MeshLib MROffsetContours closed signed round, sharp max-angle with 3D Z restore/relaxation, fixed and variable positive/inward/shell origin maps, default 3D signed/shell Z restore/relaxation, signed variable round/sharp, signed variable shell round/sharp, signed closed shell, open round/cut end, and open variable offsets through the Rust line kernel."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Mode" value="Type::Offset" />
            <Readout label="Corners" value="Round" />
            <Readout label="Precision" value="20 deg" />
            <Readout label="Source" value="Rust SDK" />
          </div>
          <OffsetContoursReadout result={offsetContoursResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Run Contour Offset"
            onClick={() =>
              onOffsetContours({
                contours: OFFSET_CONTOUR_POINTS,
                offset: 0.25,
                mode: 'offset',
                end_type: 'round',
                corner_type: 'round',
                include_origins: true,
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-from-mesh':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Mesh Distance Map"
          description="Compute MeshLib computeDistanceMap-style samples from the active mesh through the Rust ray kernel."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Grid" value="2 x 2" />
            <Readout label="Frame" value="XY plane" />
            <Readout label="Ray" value="+Z" />
            <Readout label="Source" value="Current mesh" />
          </div>
          <DistanceMapReadout result={distanceMapFromMeshResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Compute Mesh Map"
            onClick={() =>
              onDistanceMapFromMesh({
                width: 2,
                height: 2,
                origin: [0, 0, 0],
                x_range: [2, 0, 0],
                y_range: [0, 2, 0],
                direction: [0, 0, 1],
                epsilon: 1e-8,
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-contours':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Contour Distance Map"
          description="Compute MeshLib-style contour distance-map samples through the Rust pixel-center kernel."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Grid" value="3 x 3" />
            <Readout label="Sign" value="Closed contours" />
            <Readout label="Pixel" value="1.00 mm" />
            <Readout label="Source" value="Rust SDK" />
          </div>
          <DistanceMapReadout result={distanceMapContoursResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Compute Distance Map"
            onClick={() =>
              onDistanceMapContours({
                contours: DISTANCE_MAP_CONTOUR_POINTS,
                width: 3,
                height: 3,
                origin: [0, 0],
                pixel_size: [1, 1],
                signed: true,
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-iso-lines':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Distance Map Iso-Lines"
          description="Extract MeshLib-style iso-line segments from a distance map through the Rust marching-squares kernel."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Iso" value="0.00" />
            <Readout label="Source" value={distanceMapContoursResult ? 'Last map' : 'Sample map'} />
            <Readout label="Grid" value={distanceMapContoursResult ? `${distanceMapContoursResult.width} x ${distanceMapContoursResult.height}` : '2 x 2'} />
            <Readout label="Kernel" value="Rust SDK" />
          </div>
          <IsoLineSegmentsReadout result={distanceMapIsoLinesResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Extract Iso-Lines"
            onClick={() =>
              onDistanceMapIsoLines({
                width: distanceMapContoursResult?.width ?? 2,
                height: distanceMapContoursResult?.height ?? 2,
                origin: distanceMapContoursResult?.origin ?? [0, 0],
                pixel_size: distanceMapContoursResult?.pixel_size ?? [1, 1],
                values: distanceMapContoursResult?.values ?? DISTANCE_MAP_VALUES,
                valid_count: distanceMapContoursResult?.valid_count ?? 4,
                min_value: distanceMapContoursResult?.min_value ?? -1,
                max_value: distanceMapContoursResult?.max_value ?? 1,
                model_transform: distanceMapContoursResult?.model_transform ?? null,
                unit: distanceMapContoursResult?.unit ?? 'mm',
                iso_value: 0,
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-merge':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Distance Map Merge"
          description="Merge DistanceMap samples with MeshLib min, max, or subtraction invalid-cell semantics through Rust."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Mode" value="Maximum" />
            <Readout label="Left" value={distanceMapContoursResult ? 'Last map' : 'Sample map'} />
            <Readout label="Right" value="Sample map" />
            <Readout label="Kernel" value="Rust SDK" />
          </div>
          <DistanceMapReadout result={distanceMapMergeResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Merge Distance Maps"
            onClick={() =>
              onDistanceMapMerge({
                left: {
                  width: distanceMapContoursResult?.width ?? 2,
                  height: distanceMapContoursResult?.height ?? 2,
                  origin: distanceMapContoursResult?.origin ?? [0, 0],
                  pixel_size: distanceMapContoursResult?.pixel_size ?? [1, 1],
                  values: distanceMapContoursResult?.values ?? DISTANCE_MAP_VALUES,
                  valid_count: distanceMapContoursResult?.valid_count ?? 4,
                  min_value: distanceMapContoursResult?.min_value ?? -1,
                  max_value: distanceMapContoursResult?.max_value ?? 1,
                  model_transform: distanceMapContoursResult?.model_transform ?? null,
                  unit: distanceMapContoursResult?.unit ?? 'mm',
                },
                right: {
                  width: 2,
                  height: 2,
                  origin: [0, 0],
                  pixel_size: [1, 1],
                  values: DISTANCE_MAP_MERGE_RIGHT_VALUES,
                  valid_count: 3,
                  min_value: 3,
                  max_value: 6,
                  unit: 'mm',
                },
                mode: 'max',
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-contour-boolean':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Contour Boolean"
          description="Compose closed contour shapes with MeshLib union, intersection, or subtraction through Rust signed-distance maps."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Mode" value="Intersection" />
            <Readout label="Grid" value="6 x 5" />
            <Readout label="Iso" value="0.00" />
            <Readout label="Kernel" value="Rust SDK" />
          </div>
          <IsoLineSegmentsReadout result={distanceMapContourBooleanResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Boolean Contours"
            onClick={() =>
              onDistanceMapContourBoolean({
                contours_a: DISTANCE_MAP_CONTOUR_POINTS,
                contours_b: DISTANCE_MAP_CONTOUR_POINTS_B,
                mode: 'intersection',
                width: 6,
                height: 5,
                origin: [-1, -1],
                pixel_size: [1, 1],
                iso_value: 0,
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-from-tiff':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="TIFF Distance Map Import"
          description="Load MeshLib GeoTIFF distance-map samples through the Rust TIFF importer."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Format" value="TIFF" />
            <Readout label="Sample" value="2 x 2" />
            <Readout label="Transform" value="Model tag" />
            <Readout label="Kernel" value="Rust SDK" />
          </div>
          <DistanceMapReadout result={distanceMapTiffImportResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Import TIFF Map"
            onClick={() =>
              onDistanceMapFromTiff({
                file_name: 'height-field.tiff',
                contents_base64: DISTANCE_MAP_TIFF_SAMPLE_BASE64,
              })
            }
          />
        </ToolCard>
      );
    case 'distance-map-to-tiff': {
      const exportSource =
        distanceMapTiffImportResult ??
        distanceMapMergeResult ??
        distanceMapFromMeshResult ??
        distanceMapContoursResult;
      return (
        <ToolCard
          eyebrow="Inspect"
          title="TIFF Distance Map Export"
          description="Write MeshLib-style TIFF distance-map samples through the Rust TIFF exporter."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Format" value="TIFF" />
            <Readout label="Source" value={exportSource ? 'Last map' : 'Sample map'} />
            <Readout label="NoData" value="MeshLib" />
            <Readout label="Kernel" value="Rust SDK" />
          </div>
          <TiffExportReadout result={distanceMapTiffExportResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Export TIFF Map"
            onClick={() =>
              onDistanceMapToTiff({
                file_name: 'exported-height-field.tiff',
                width: exportSource?.width ?? 2,
                height: exportSource?.height ?? 2,
                origin: exportSource?.origin ?? [10, 20],
                pixel_size: exportSource?.pixel_size ?? [2.5, 4],
                values: exportSource?.values ?? DISTANCE_MAP_TIFF_VALUES,
                valid_count: exportSource?.valid_count ?? 4,
                min_value: exportSource?.min_value ?? 1,
                max_value: exportSource?.max_value ?? 4,
                model_transform: exportSource?.model_transform ?? null,
                unit: exportSource?.unit ?? 'mm',
              })
            }
          />
        </ToolCard>
      );
    }
    case 'object-lines-from-contours':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines From Contours"
          description="Build MeshLib ObjectLines scene JSON from contour polylines through the Rust PolylineTopology path."
        >
          <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Object" value="ObjectLines" />
            <Readout label="Holder" value="LinesHolder" />
            <Readout label="Polyline" value="3 points" />
            <Readout label="Source" value="Rust SDK" />
          </div>
          <ObjectLinesReadout result={objectLinesResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Build ObjectLines"
            onClick={() =>
              onObjectLinesFromContours({
                contours: OBJECT_LINES_CONTOUR_POINTS,
                line_width: 1.5,
                show_points: 1,
                smooth_connections: 0,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-load-mrlines':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Load MrLines"
          description="Load MeshLib binary MrLines topology and Vector3f points through the Rust LinesLoad::fromMrLines path."
        >
          <ObjectLinesReadout result={objectLinesResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Load MrLines"
            onClick={() =>
              onObjectLinesLoadMrLines({
                file_name: 'object-lines.mrlines',
                contents_base64: OBJECT_LINES_MRLINES_BASE64,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-save-mrlines':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Save MrLines"
          description="Export MeshLib binary MrLines topology and point payloads through the Rust LinesSave::toMrLines path."
        >
          <ObjectLinesBinaryExportReadout result={objectLinesMrLinesExportResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Save MrLines"
            onClick={() =>
              onObjectLinesSaveMrLines({
                file_name: 'object-lines.mrlines',
                object_lines: objectLinesResult?.object_lines ?? OBJECT_LINES_PAYLOAD,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-load-ply':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Load PLY"
          description="Load MeshLib PLY vertex and edge payloads through the Rust LinesLoad::fromPly path."
        >
          <ObjectLinesReadout result={objectLinesResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Load PLY"
            onClick={() =>
              onObjectLinesLoadPly({
                file_name: 'object-lines.ply',
                contents_base64: OBJECT_LINES_PLY_BASE64,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-save-ply':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Save PLY"
          description="Export MeshLib binary little-endian PLY vertex, optional color, and edge payloads through the Rust LinesSave::toPly path."
        >
          <ObjectLinesBinaryExportReadout result={objectLinesPlyExportResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Save PLY"
            onClick={() =>
              onObjectLinesSavePly({
                file_name: 'object-lines.ply',
                object_lines: objectLinesResult?.object_lines ?? OBJECT_LINES_PAYLOAD,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-load-pts':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Load PTS"
          description="Load MeshLib BEGIN_Polyline PTS text through the Rust LinesLoad::fromPts path."
        >
          <ObjectLinesReadout result={objectLinesResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Load PTS"
            onClick={() =>
              onObjectLinesLoadPts({
                file_name: 'object-lines.pts',
                source: OBJECT_LINES_PTS_SOURCE,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-load-svg':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Load SVG"
          description="Load MeshLib SVG lines and polylines through the Rust LinesLoad::fromSvg path."
        >
          <ObjectLinesReadout result={objectLinesResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Load SVG"
            onClick={() =>
              onObjectLinesLoadSvg({
                file_name: 'object-lines.svg',
                source: OBJECT_LINES_SVG_SOURCE,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-save-pts':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Save PTS"
          description="Export MeshLib ObjectLines scene JSON through the Rust LinesSave::toPts path."
        >
          <ObjectLinesTextExportReadout result={objectLinesPtsExportResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Save PTS"
            onClick={() =>
              onObjectLinesSavePts({
                file_name: 'object-lines.pts',
                object_lines: objectLinesResult?.object_lines ?? OBJECT_LINES_PAYLOAD,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-save-dxf':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines Save DXF"
          description="Export MeshLib ObjectLines scene JSON through the Rust LinesSave::toDxf path."
        >
          <ObjectLinesTextExportReadout result={objectLinesDxfExportResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Save DXF"
            onClick={() =>
              onObjectLinesSaveDxf({
                file_name: 'object-lines.dxf',
                object_lines: objectLinesResult?.object_lines ?? OBJECT_LINES_PAYLOAD,
              })
            }
          />
        </ToolCard>
      );
    case 'object-lines-to-contours':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="ObjectLines To Contours"
          description="Restore contour polylines from MeshLib ObjectLines scene JSON through the Rust PolylineTopology traversal."
        >
          <ObjectLinesContoursReadout result={objectLinesContoursResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Restore Contours"
            onClick={() =>
              onObjectLinesToContours({
                object_lines: objectLinesResult?.object_lines ?? OBJECT_LINES_PAYLOAD,
              })
            }
          />
        </ToolCard>
      );
    case 'point-cloud-icp':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Point Cloud / ICP"
          description="Run MeshLib-style pairwise point-cloud ICP registration through the Rust-backed SDK."
        >
          <PointCloudIcpReadout result={pointCloudIcpResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Run ICP"
            onClick={() =>
              onPointCloudIcp({
                floating_points: ICP_FLOATING_POINTS,
                reference_points: ICP_REFERENCE_POINTS,
                method: 'point_to_point',
                mode: 'translation',
                max_iterations: 20,
                tolerance: 1e-8,
              })
            }
          />
        </ToolCard>
      );
    case 'open-raw-voxels':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Open RAW Voxels"
          description="Load MeshLib VoxelsLoad::fromRaw RAW voxel payloads through the Rust-backed SDK."
        >
          <VoxelVolumeLoadReadout result={voxelLoadResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Open RAW"
            onClick={() =>
              onOpenRawVoxels({
                file_name: 'explicit.raw',
                contents_base64: RAW_VOXELS_BASE64,
                dimensions: [2, 2, 1],
                voxel_size: [0.5, 1.0, 2.0],
                scalar_type: 'uint16',
              })
            }
          />
        </ToolCard>
      );
    case 'open-voxels-from-tiff':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Open Voxels From TIFF"
          description="Load MeshLib VoxelsLoad::loadTiffDir TIFF slice stacks through the Rust-backed SDK."
        >
          <VoxelVolumeLoadReadout result={voxelLoadResult} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label="Open TIFF Stack"
            onClick={() =>
              onOpenVoxelsFromTiff({
                files: {
                  'slice_10.tiff': TIFF_VOXEL_SLICE_10_BASE64,
                  'slice_02.tiff': TIFF_VOXEL_SLICE_02_BASE64,
                },
                voxel_size: [0.5, 0.25, 2.0],
              })
            }
          />
        </ToolCard>
      );
    case 'mesh-to-voxels-sdf':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Mesh to Voxels / SDF"
          description="Convert the active mesh through the Rust-backed SDK using MeshLib meshToLevelSet or meshToDistanceField-style settings."
        >
          <NumberField
            label="Voxel Size"
            value={drafts.voxelSizeMm}
            min={0.01}
            max={25}
            step={0.1}
            onChange={(value) => updateDrafts({ voxelSizeMm: value })}
          />
          <NumberField
            label="Surface Offset"
            value={drafts.voxelSurfaceOffsetVoxels}
            min={0.1}
            max={20}
            step={0.5}
            onChange={(value) => updateDrafts({ voxelSurfaceOffsetVoxels: value })}
          />
          <div className="grid grid-cols-2 gap-2">
            <SecondaryButton
              label="Signed"
              onClick={() => updateDrafts({ voxelMode: 'signed' })}
              active={drafts.voxelMode === 'signed'}
            />
            <SecondaryButton
              label="Unsigned"
              onClick={() => updateDrafts({ voxelMode: 'unsigned' })}
              active={drafts.voxelMode === 'unsigned'}
            />
          </div>
          <InlineToggle
            label="Extract Iso-surface"
            enabled={drafts.voxelExtractSurface}
            onClick={() => updateDrafts({ voxelExtractSurface: !drafts.voxelExtractSurface })}
          />
          <MeshToVoxelsReadout result={meshToVoxelsResult} />
          <ActionFooter
            busy={busy}
            disabled={drafts.voxelSizeMm <= 0 || drafts.voxelSurfaceOffsetVoxels <= 0}
            label="Convert Mesh"
            onClick={() =>
              onMeshToVoxelsSdf({
                voxel_size_mm: drafts.voxelSizeMm,
                surface_offset_voxels: drafts.voxelSurfaceOffsetVoxels,
                mode: drafts.voxelMode,
                iso_value: 0,
                extract_surface: drafts.voxelExtractSurface,
              })
            }
          />
        </ToolCard>
      );
    case 'voxel-volume-render-ray':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Volume Ray"
          description="Composite a ray through a MeshLib-style voxel volume using the Rust-backed renderer."
        >
          <div className="grid grid-cols-3 gap-2">
            <NumberField
              label="Start X"
              value={drafts.volumeRayStartX}
              min={-100}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ volumeRayStartX: value })}
            />
            <NumberField
              label="Start Y"
              value={drafts.volumeRayStartY}
              min={-100}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ volumeRayStartY: value })}
            />
            <NumberField
              label="Start Z"
              value={drafts.volumeRayStartZ}
              min={-100}
              max={100}
              step={0.1}
              onChange={(value) => updateDrafts({ volumeRayStartZ: value })}
            />
          </div>
          <div className="grid grid-cols-3 gap-2">
            <NumberField
              label="Dir X"
              value={drafts.volumeRayDirectionX}
              min={-1}
              max={1}
              step={0.1}
              onChange={(value) => updateDrafts({ volumeRayDirectionX: value })}
            />
            <NumberField
              label="Dir Y"
              value={drafts.volumeRayDirectionY}
              min={-1}
              max={1}
              step={0.1}
              onChange={(value) => updateDrafts({ volumeRayDirectionY: value })}
            />
            <NumberField
              label="Dir Z"
              value={drafts.volumeRayDirectionZ}
              min={-1}
              max={1}
              step={0.1}
              onChange={(value) => updateDrafts({ volumeRayDirectionZ: value })}
            />
          </div>
          <NumberField
            label="Sampling Step"
            value={drafts.volumeRaySamplingStep}
            min={0.01}
            max={10}
            step={0.01}
            onChange={(value) => updateDrafts({ volumeRaySamplingStep: value })}
          />
          <NumberField
            label="Alpha Limit"
            value={drafts.volumeRayAlphaLimit}
            min={0}
            max={255}
            step={1}
            onChange={(value) => updateDrafts({ volumeRayAlphaLimit: Math.round(value) })}
          />
          <NumberField
            label="Max Steps"
            value={drafts.volumeRayMaxSteps}
            min={1}
            max={4096}
            step={1}
            onChange={(value) => updateDrafts({ volumeRayMaxSteps: Math.max(1, Math.round(value)) })}
          />
          <VoxelVolumeRenderRayReadout result={voxelVolumeRenderRayResult} />
          <ActionFooter
            busy={busy}
            disabled={
              drafts.volumeRaySamplingStep <= 0 ||
              drafts.volumeRayMaxSteps < 1 ||
              (drafts.volumeRayDirectionX === 0 &&
                drafts.volumeRayDirectionY === 0 &&
                drafts.volumeRayDirectionZ === 0)
            }
            label="Cast Ray"
            onClick={() =>
              onVoxelVolumeRenderRay({
                values: [0, 0.2, 0.4, 0.6, 0.8, 1, 0.5, 0.1],
                shape: [2, 2, 2],
                voxel_size: [1, 1, 1],
                min_corner: [0, 0, 0],
                ray_start: [drafts.volumeRayStartX, drafts.volumeRayStartY, drafts.volumeRayStartZ],
                ray_direction: [
                  drafts.volumeRayDirectionX,
                  drafts.volumeRayDirectionY,
                  drafts.volumeRayDirectionZ,
                ],
                sampling_step: drafts.volumeRaySamplingStep,
                min_value: 0,
                max_value: 1,
                lut_type: 'rainbow',
                alpha_type: 'constant',
                alpha_limit: Math.max(0, Math.min(255, Math.round(drafts.volumeRayAlphaLimit))),
                shading_mode: 'none',
                max_steps: Math.max(1, Math.round(drafts.volumeRayMaxSteps)),
              })
            }
          />
        </ToolCard>
      );
    case 'exact-boolean':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Exact Boolean"
          description="Run MeshLib MR::boolean-style exact mesh operations between the active version and another ready version."
        >
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Target Version
            <input
              value={drafts.booleanTargetVersionId}
              onChange={(event) => updateDrafts({ booleanTargetVersionId: event.target.value.trim() })}
              placeholder="ver_..."
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Operation
            <select
              value={drafts.booleanOperation}
              onChange={(event) =>
                updateDrafts({ booleanOperation: event.target.value as ExactBooleanRequest['operation'] })
              }
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 outline-none"
            >
              <option value="difference">Difference A-B</option>
              <option value="difference_ba">Difference B-A</option>
              <option value="union">Union</option>
              <option value="intersection">Intersection</option>
              <option value="inside_a">Inside A</option>
              <option value="inside_b">Inside B</option>
              <option value="outside_a">Outside A</option>
              <option value="outside_b">Outside B</option>
            </select>
          </label>
          <ExactBooleanReadout result={exactBooleanResult} />
          <ActionFooter
            busy={busy}
            disabled={!drafts.booleanTargetVersionId}
            label="Run Boolean"
            onClick={() =>
              onExactBoolean({
                other_version_id: drafts.booleanTargetVersionId,
                operation: drafts.booleanOperation,
                epsilon: 1e-8,
              })
            }
          />
        </ToolCard>
      );
    case 'voxel-boolean':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Voxel Boolean"
          description="Run MeshLib MRVoxels-style voxel mesh operations between the active version and another ready version."
        >
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Target Version
            <input
              value={drafts.voxelBooleanTargetVersionId}
              onChange={(event) => updateDrafts({ voxelBooleanTargetVersionId: event.target.value.trim() })}
              placeholder="ver_..."
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Operation
            <select
              value={drafts.voxelBooleanOperation}
              onChange={(event) =>
                updateDrafts({ voxelBooleanOperation: event.target.value as VoxelBooleanRequest['operation'] })
              }
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 outline-none"
            >
              <option value="union">Union</option>
              <option value="intersection">Intersection</option>
              <option value="difference">Difference A-B</option>
            </select>
          </label>
          <NumberField
            label="Voxel Size"
            value={drafts.voxelBooleanSizeMm}
            min={0.05}
            max={5}
            step={0.05}
            onChange={(value) => updateDrafts({ voxelBooleanSizeMm: value })}
          />
          <NumberField
            label="Padding"
            value={drafts.voxelBooleanPaddingMm}
            min={0.05}
            max={20}
            step={0.1}
            onChange={(value) => updateDrafts({ voxelBooleanPaddingMm: value })}
          />
          <InlineToggle
            label="Refine Surface"
            enabled={drafts.voxelBooleanRefine}
            onClick={() => updateDrafts({ voxelBooleanRefine: !drafts.voxelBooleanRefine })}
          />
          <VoxelBooleanReadout result={voxelBooleanResult} />
          <ActionFooter
            busy={busy}
            disabled={
              !drafts.voxelBooleanTargetVersionId ||
              drafts.voxelBooleanSizeMm <= 0 ||
              drafts.voxelBooleanPaddingMm <= 0
            }
            label="Run Voxel Boolean"
            onClick={() =>
              onVoxelBoolean({
                other_version_id: drafts.voxelBooleanTargetVersionId,
                operation: drafts.voxelBooleanOperation,
                voxel_size_mm: drafts.voxelBooleanSizeMm,
                padding_mm: drafts.voxelBooleanPaddingMm,
                refine: drafts.voxelBooleanRefine,
              })
            }
          />
        </ToolCard>
      );
    case 'collision-detect':
      return (
        <ToolCard
          eyebrow="Inspect"
          title="Collision Detection"
          description="Find MeshLib findCollidingTriangles-style face pairs between the active version and another ready version."
        >
          <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
            Target Version
            <input
              value={drafts.collisionTargetVersionId}
              onChange={(event) => updateDrafts({ collisionTargetVersionId: event.target.value.trim() })}
              placeholder="ver_..."
              spellCheck={false}
              className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-100 outline-none placeholder:text-zinc-500"
            />
          </label>
          <InlineToggle
            label="First Pair Only"
            enabled={drafts.collisionFirstOnly}
            onClick={() => updateDrafts({ collisionFirstOnly: !drafts.collisionFirstOnly })}
          />
          <NumberField
            label="Max Pairs"
            value={drafts.collisionMaxPairs}
            min={1}
            max={50000}
            step={100}
            onChange={(value) => updateDrafts({ collisionMaxPairs: value })}
          />
          <CollisionReadout result={collisionResult} />
          <ActionFooter
            busy={busy}
            disabled={!drafts.collisionTargetVersionId || drafts.collisionMaxPairs < 1}
            label="Detect Collision"
            onClick={() =>
              onCollisionDetect({
                other_version_id: drafts.collisionTargetVersionId,
                first_intersection_only: drafts.collisionFirstOnly,
                max_pairs: drafts.collisionMaxPairs,
                epsilon: 1e-8,
              })
            }
          />
        </ToolCard>
      );
    case 'wireframe':
      return (
        <ToolCard eyebrow="Inspect" title="Wireframe" description="Toggle topology linework on the preview mesh.">
          <InlineToggle label="Wireframe" enabled={wireframe} onClick={onWireframeToggle} />
          <ActionFooter
            busy={busy}
            disabled={false}
            label={wireframe ? 'Hide Wireframe' : 'Show Wireframe'}
            onClick={onWireframeToggle}
          />
        </ToolCard>
      );
    case 'snapshots':
      return (
        <ToolCard eyebrow="Inspect" title="Inspection Snapshots" description="Save and restore repeated inspection views.">
          <div className="flex gap-2">
            <input
              value={drafts.snapshotName}
              onChange={(event) => updateDrafts({ snapshotName: event.target.value })}
              placeholder="Name current view"
              className="min-w-0 flex-1 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-500"
            />
            <button
              onClick={() => {
                const trimmed = drafts.snapshotName.trim();
                if (!trimmed) return;
                onSaveSnapshot(trimmed);
                updateDrafts({ snapshotName: '' });
              }}
              className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-200 hover:bg-zinc-900"
            >
              Save
            </button>
          </div>
          <div className="space-y-2">
            {savedSnapshots.length ? (
              savedSnapshots.slice(0, 8).map((snapshot) => (
                <button
                  key={snapshot.id}
                  onClick={() => onLoadSnapshot(snapshot)}
                  className="w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-left hover:bg-zinc-900"
                >
                  <p className="text-sm text-zinc-100">{snapshot.name}</p>
                  <p className="mt-1 text-xs text-zinc-500">
                    section {snapshot.section_enabled ? `${snapshot.section_constant.toFixed(1)} mm` : 'off'} |{' '}
                    {new Date(snapshot.created_at).toLocaleString()}
                  </p>
                </button>
              ))
            ) : (
              <p className="text-sm text-zinc-500">No saved inspection snapshots yet.</p>
            )}
          </div>
        </ToolCard>
      );
    default:
      return null;
  }
}

function HollowTool({
  title,
  busy,
  selectedMaterial,
  wallThickness,
  minThickness,
  fullResolution,
  onMaterialChange,
  onDraftChange,
  onApply,
  actionLabel,
}: {
  title: string;
  busy: boolean;
  selectedMaterial: MaterialType;
  wallThickness: number;
  minThickness: number;
  fullResolution: boolean;
  onMaterialChange: (value: MaterialType) => void;
  onDraftChange: (value: Partial<ToolDrafts>) => void;
  onApply: () => void;
  actionLabel: string;
}) {
  return (
    <ToolCard eyebrow="Modify" title={title} description="Weighted hollowing that preserves decorative head and relief regions.">
      <MaterialField material={selectedMaterial} onMaterialChange={onMaterialChange} />
      <InlineToggle
        label="Full Resolution Batch"
        enabled={fullResolution}
        onClick={() => onDraftChange({ hollowFullResolution: !fullResolution })}
      />
      <div className="grid grid-cols-2 gap-3">
        <NumberField
          label="Wall Thickness"
          value={wallThickness}
          min={0.3}
          max={5}
          step={0.05}
          onChange={(value) => onDraftChange({ wallThickness: value })}
        />
        <NumberField
          label="Min Thickness"
          value={minThickness}
          min={0.2}
          max={5}
          step={0.05}
          onChange={(value) => onDraftChange({ minThickness: value })}
        />
      </div>
      <ActionFooter busy={busy} disabled={false} label={actionLabel} onClick={onApply} />
    </ToolCard>
  );
}

function OffsetSmoothingTool({
  title,
  description,
  busy,
  distance,
  voxelSize,
  padding,
  refine,
  result,
  actionLabel,
  onDistanceChange,
  onVoxelSizeChange,
  onPaddingChange,
  onRefineToggle,
  onApply,
}: {
  title: string;
  description: string;
  busy: boolean;
  distance: number;
  voxelSize: number;
  padding: number;
  refine: boolean;
  result: OffsetShellMeshResponse | null;
  actionLabel: string;
  onDistanceChange: (value: number) => void;
  onVoxelSizeChange: (value: number) => void;
  onPaddingChange: (value: number) => void;
  onRefineToggle: () => void;
  onApply: () => void;
}) {
  return (
    <ToolCard eyebrow="Modify" title={title} description={description}>
      <div className="grid grid-cols-2 gap-3">
        <NumberField
          label="Distance"
          value={distance}
          min={0.01}
          max={20}
          step={0.01}
          onChange={onDistanceChange}
        />
        <NumberField
          label="Voxel Size"
          value={voxelSize}
          min={0.01}
          max={10}
          step={0.05}
          onChange={onVoxelSizeChange}
        />
      </div>
      <NumberField
        label="Padding"
        value={padding}
        min={0.01}
        max={50}
        step={0.1}
        onChange={onPaddingChange}
      />
      <InlineToggle label="Refine Surface" enabled={refine} onClick={onRefineToggle} />
      <OffsetShellReadout result={result} />
      <ActionFooter
        busy={busy}
        disabled={distance <= 0 || voxelSize <= 0 || padding <= 0}
        label={actionLabel}
        onClick={onApply}
      />
    </ToolCard>
  );
}

function ToolCard({
  eyebrow,
  title,
  description,
  children,
  footer,
}: {
  eyebrow: string;
  title: string;
  description: string;
  children?: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <div className="space-y-4 rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
      <div>
        <p className="text-[11px] uppercase tracking-[0.22em] text-zinc-500">{eyebrow}</p>
        <h2 className="mt-2 text-lg font-semibold text-white">{title}</h2>
        <p className="mt-2 text-sm leading-6 text-zinc-500">{description}</p>
      </div>
      {children}
      {footer}
    </div>
  );
}

function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-5">
      <h2 className="text-lg font-semibold text-white">{title}</h2>
      <p className="mt-3 text-sm leading-6 text-zinc-500">{body}</p>
    </div>
  );
}

function MaterialField({
  material,
  onMaterialChange,
}: {
  material: MaterialType;
  onMaterialChange: (value: MaterialType) => void;
}) {
  return (
    <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
      Material
      <select
        value={material}
        onChange={(event) => onMaterialChange(event.target.value as MaterialType)}
        className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100"
      >
        {Object.entries(MATERIALS).map(([value, item]) => (
          <option key={value} value={value}>
            {item.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
      {label}
      <input
        type="number"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100"
      />
    </label>
  );
}

function InlineToggle({
  label,
  enabled,
  onClick,
}: {
  label: string;
  enabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex w-full items-center justify-between rounded-xl border px-3 py-3 text-sm ${
        enabled ? 'border-blue-500/40 bg-blue-500/12 text-blue-200' : 'border-zinc-800 bg-zinc-950 text-zinc-300'
      }`}
    >
      <span>{label}</span>
      <span className="text-xs uppercase tracking-[0.16em]">{enabled ? 'On' : 'Off'}</span>
    </button>
  );
}

function ActionFooter({
  busy,
  disabled,
  disabledReason,
  label,
  onClick,
}: {
  busy: boolean;
  disabled: boolean;
  disabledReason?: string;
  label: string;
  onClick: () => void;
}) {
  return (
    <div className="space-y-3 border-t border-zinc-800 pt-4">
      {disabledReason ? <p className="text-xs text-amber-300">{disabledReason}</p> : null}
      <button
        onClick={onClick}
        disabled={busy || disabled}
        className="w-full rounded-xl bg-amber-500 px-4 py-3 text-sm font-semibold text-black disabled:cursor-not-allowed disabled:opacity-50"
      >
        {busy ? 'Working…' : label}
      </button>
    </div>
  );
}

function SecondaryButton({
  label,
  onClick,
  disabled = false,
  active = false,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  active?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`rounded-xl border px-3 py-2 text-sm hover:bg-zinc-900 disabled:opacity-40 ${
        active ? 'border-blue-500/40 bg-blue-500/12 text-blue-200' : 'border-zinc-800 bg-zinc-950 text-zinc-300'
      }`}
    >
      {label}
    </button>
  );
}

function RegionPicker({
  regions,
  selectedRegionIds,
  selectedRegion,
  onRegionSelect,
  onRegionToggle,
}: {
  regions: RegionManifestEntry[];
  selectedRegionIds: string[];
  selectedRegion: RegionManifestEntry | null;
  onRegionSelect: (regionId: string) => void;
  onRegionToggle: (regionId: string) => void;
}) {
  return (
    <div className="space-y-2">
      <p className="text-[11px] uppercase tracking-[0.18em] text-zinc-500">Regions</p>
      <div className="max-h-72 space-y-2 overflow-y-auto pr-1">
        {regions.map((region) => {
          const checked = selectedRegionIds.includes(region.region_id);
          const primary = selectedRegion?.region_id === region.region_id;
          return (
            <button
              key={region.region_id}
              onClick={() => onRegionSelect(region.region_id)}
              className={`w-full rounded-xl border px-3 py-2 text-left ${
                checked ? 'border-amber-500/30 bg-amber-500/10' : 'border-zinc-800 bg-zinc-950'
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate text-sm text-zinc-100">{region.label}</p>
                  <p className="mt-1 text-xs text-zinc-500">
                    {region.coverage_pct}% coverage
                    {primary ? ' • primary' : ''}
                  </p>
                </div>
                <label
                  className="inline-flex items-center gap-2 text-xs text-zinc-400"
                  onClick={(event) => event.stopPropagation()}
                >
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={() => onRegionToggle(region.region_id)}
                    className="accent-amber-400"
                  />
                  Batch
                </label>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function RegionSummary({
  region,
  fallback,
}: {
  region: RegionManifestEntry | null;
  fallback: string;
}) {
  if (!region) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">{fallback}</p>;
  }

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <p className="text-sm font-medium text-zinc-100">{region.label}</p>
      <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Coverage" value={`${region.coverage_pct}%`} />
        <Readout label="Vertices" value={String(region.vertex_count)} />
        <Readout label="Min T" value={region.min_thickness_mm != null ? `${region.min_thickness_mm.toFixed(2)} mm` : 'n/a'} />
        <Readout label="Avg T" value={region.avg_thickness_mm != null ? `${region.avg_thickness_mm.toFixed(2)} mm` : 'n/a'} />
      </div>
      <p className="mt-3 text-xs text-zinc-500">
        Allowed ops: {region.allowed_operations.length ? region.allowed_operations.join(', ') : 'none'}
      </p>
    </div>
  );
}

function SectionReadout({ sectionContour }: { sectionContour: SectionContourPayload }) {
  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Section Contour</p>
        <span className="text-xs text-zinc-500">{sectionContour.contour_count} contours</span>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Perimeter" value={sectionContour.perimeter_mm != null ? `${sectionContour.perimeter_mm.toFixed(2)} mm` : 'n/a'} />
        <Readout label="Width" value={sectionContour.width_mm != null ? `${sectionContour.width_mm.toFixed(2)} mm` : 'n/a'} />
        <Readout label="Depth" value={sectionContour.depth_mm != null ? `${sectionContour.depth_mm.toFixed(2)} mm` : 'n/a'} />
        <Readout label="Segments" value={String(sectionContour.segment_count)} />
      </div>
    </div>
  );
}

function OverlayReadout({
  overlay,
  fallback,
}: {
  overlay: ScalarOverlayResponse | null;
  fallback: string;
}) {
  if (!overlay) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">{fallback}</p>;
  }

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="h-3 rounded-full bg-gradient-to-r from-red-500 via-amber-400 to-green-500" />
      <div className="mt-2 flex items-center justify-between text-xs text-zinc-500">
        <span>{overlay.min_value.toFixed(2)}</span>
        <span>{overlay.center_value.toFixed(2)}</span>
        <span>{overlay.max_value.toFixed(2)}</span>
      </div>
    </div>
  );
}

function MeasureInspectReadout({ result }: { result: MeasureInspectResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No measurement probe has been captured yet.</p>;
  }

  const point = result.points[0] ?? null;
  const pair = result.point_pairs[0] ?? null;
  const featurePair = result.feature_pairs[0] ?? null;
  const featureObject = result.feature_objects[0] ?? null;
  const featureRefinement = result.feature_refinements[0] ?? null;
  const surfaceDistance = result.surface_distance ?? null;
  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Measurement Result</p>
        <span className="text-xs text-zinc-500">{result.version_id}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Points" value={String(result.points.length)} />
        <Readout label="Pairs" value={String(result.point_pairs.length)} />
        <Readout label="Features" value={String(result.feature_pairs.length)} />
        <Readout label="Objects" value={String(result.feature_objects.length)} />
        <Readout label="Refined" value={String(result.feature_refinements.length)} />
        <Readout label="Surface Gap" value={point ? `${point.distance_to_surface_mm.toFixed(3)} mm` : 'n/a'} />
        <Readout label="Local T" value={point?.local_thickness_mm != null ? `${point.local_thickness_mm.toFixed(3)} mm` : 'n/a'} />
      </div>
      {point ? (
        <p className="text-xs text-zinc-500">
          Closest point: {point.closest_point.map((coordinate) => coordinate.toFixed(2)).join(', ')}
        </p>
      ) : null}
      {pair ? (
        <p className="text-xs text-zinc-500">
          First pair: {pair.distance_mm.toFixed(3)} mm
          {pair.metric === 'geodesic' ? ` geodesic, ${pair.line_segments} segments` : ''}
          {pair.control_vertex_indices.length ? `, ${pair.control_vertex_indices.length} controls` : ''}
          {pair.leg_lengths_mm.length ? `, ${pair.leg_lengths_mm.length} legs` : ''}
          {pair.closed_path ? ', closed' : ''}
          {pair.surface_path_refinement ? `, reduced ${pair.surface_path_refinement.length_mm.toFixed(3)} mm` : ''}
          {pair.cut_contours ? ', cut contours' : ''}
          {pair.path_object_lines ? ', ObjectLines export' : ''}
        </p>
      ) : null}
      {featurePair ? (
        <p className="text-xs text-zinc-500">
          First feature pair: {featurePair.distance.distance_mm != null ? `${featurePair.distance.distance_mm.toFixed(3)} mm exact` : featurePair.distance.status}
          {featurePair.center_distance.distance_mm != null ? `, ${featurePair.center_distance.distance_mm.toFixed(3)} mm center` : `, ${featurePair.center_distance.status}`}
          {featurePair.angle.angle_degrees != null ? `, ${featurePair.angle.angle_degrees.toFixed(2)} deg` : `, ${featurePair.angle.status}`}
          {featurePair.intersections.length ? `, ${featurePair.intersections.length} intersection${featurePair.intersections.length === 1 ? '' : 's'}` : ''}
        </p>
      ) : null}
      {featureObject ? (
        <p className="text-xs text-zinc-500">
          First feature object: {featureObject.object_type}, {featureObject.shared_properties.length} shared properties
        </p>
      ) : null}
      {featureRefinement ? (
        <p className="text-xs text-zinc-500">
          Refined feature: {featureRefinement.kind} {featureRefinement.feature_id}, {featureRefinement.selected_count} vertices
          {featureRefinement.iterations ? `, ${featureRefinement.iterations} iterations` : ''}
          {featureRefinement.converged ? ', converged' : ', max iterations'}
        </p>
      ) : null}
      {surfaceDistance ? (
        <p className="text-xs text-zinc-500">
          Surface distance: {surfaceDistance.reachable_vertex_count} vertices, max {surfaceDistance.max_distance_mm.toFixed(3)} mm
          {surfaceDistance.seed_edges.length || surfaceDistance.seed_face_boundary_edges.length
            ? `, sources ${surfaceDistance.seed_edges.length} edges/${surfaceDistance.seed_face_boundary_edges.length} face-boundary edges`
            : ''}
          {surfaceDistance.iso_value_mm != null
            ? `, iso ${surfaceDistance.iso_value_mm.toFixed(3)} mm, ${surfaceDistance.iso_segments.length} segments, ${surfaceDistance.clipped_faces.length} clipped faces`
            : ''}
        </p>
      ) : null}
    </div>
  );
}

function GcodeParseReadout({ result }: { result: GcodeParsePathsResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No G-code path parse has been captured yet.</p>;
  }

  const firstSegment = result.segments.find((segment) => segment.length > 1) ?? null;
  const firstPoint = firstSegment?.[0] ?? null;
  const lastPoint = firstSegment?.[firstSegment.length - 1] ?? null;

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Toolpath Result</p>
        <span className="text-xs text-zinc-500">{result.unit}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Frames" value={String(result.frame_count)} />
        <Readout label="Commands" value={String(result.command_count)} />
        <Readout label="Segments" value={String(result.segment_count)} />
        <Readout label="Max Feed" value={result.max_feedrate.toFixed(1)} />
      </div>
      {firstPoint && lastPoint ? (
        <p className="text-xs text-zinc-500">
          First path: {firstPoint.map((coordinate) => coordinate.toFixed(2)).join(', ')} to {lastPoint.map((coordinate) => coordinate.toFixed(2)).join(', ')}
        </p>
      ) : null}
      {result.warnings.length ? <p className="text-xs text-amber-300">{result.warnings[0]}</p> : null}
    </div>
  );
}

function OffsetContoursReadout({ result }: { result: OffsetContoursResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No contour offset has been captured yet.</p>;
  }

  const firstPoint = result.contours[0]?.[0] ?? null;

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Offset Result</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'offset_contours')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Contours" value={String(result.contour_count)} />
        <Readout label="Points" value={String(result.point_count)} />
        <Readout label="Origins" value={String(result.origins.length)} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
      </div>
      {firstPoint ? (
        <p className="text-xs text-zinc-500">
          First point: {firstPoint.map((coordinate) => coordinate.toFixed(3)).join(', ')}
        </p>
      ) : null}
    </div>
  );
}

function DistanceMapReadout({ result }: { result: DistanceMapResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No distance map has been captured yet.</p>;
  }

  const centerY = Math.floor(result.height / 2);
  const centerX = Math.floor(result.width / 2);
  const centerValue = result.values[centerY]?.[centerX];

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Distance Map</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'distance_map_from_contours')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Resolution" value={`${result.width} x ${result.height}`} />
        <Readout label="Valid" value={String(result.valid_count)} />
        <Readout label="Min" value={result.min_value.toFixed(3)} />
        <Readout label="Max" value={result.max_value.toFixed(3)} />
      </div>
      <p className="text-xs text-zinc-500">
        Center sample: {typeof centerValue === 'number' ? centerValue.toFixed(3) : 'n/a'} {result.unit}
      </p>
    </div>
  );
}

function TiffExportReadout({ result }: { result: DistanceMapTiffExportResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No TIFF export has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">TIFF Export</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'distance_map_to_tiff')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="File" value={result.file_name} />
        <Readout label="Bytes" value={String(result.byte_count)} />
      </div>
      <p className="text-xs text-zinc-500">
        Payload: {result.contents_base64.length} base64 chars
      </p>
    </div>
  );
}

function IsoLineSegmentsReadout({ result }: { result: IsoLineSegmentsResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No iso-line extraction has been captured yet.</p>;
  }

  const firstSegment = result.segments[0] ?? null;

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Iso-Line Segments</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'distance_map_to_iso_segments')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Segments" value={String(result.segment_count)} />
        <Readout label="Iso" value={result.iso_value.toFixed(3)} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
        <Readout label="Unit" value={result.unit} />
      </div>
      {firstSegment ? (
        <p className="text-xs text-zinc-500">
          First segment: {firstSegment[0].map((coordinate) => coordinate.toFixed(3)).join(', ')} to {firstSegment[1].map((coordinate) => coordinate.toFixed(3)).join(', ')}
        </p>
      ) : null}
    </div>
  );
}

function ObjectLinesReadout({ result }: { result: ObjectLinesResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No ObjectLines payload has been captured yet.</p>;
  }

  const polyline = result.object_lines.Polyline as { Points?: unknown[]; Lines?: unknown[] } | undefined;
  const pointCount = Array.isArray(polyline?.Points) ? polyline.Points.length : result.point_count;
  const edgeIndexCount = Array.isArray(polyline?.Lines) ? polyline.Lines.length : result.line_count * 2;

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">ObjectLines JSON</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'object_lines_from_contours')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Points" value={String(pointCount)} />
        <Readout label="Lines" value={String(result.line_count)} />
        <Readout label="Index Pairs" value={String(edgeIndexCount / 2)} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
      </div>
      <p className="text-xs text-zinc-500">Line width: {result.line_width.toFixed(2)}</p>
    </div>
  );
}

function ObjectLinesTextExportReadout({ result }: { result: ObjectLinesTextExportResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No ObjectLines text export has been captured yet.</p>;
  }

  const lineCount = result.source.split(/\r?\n/).filter((line) => line.length > 0).length;

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">ObjectLines Text</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'object_lines_to_pts')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="File" value={result.file_name} />
        <Readout label="Bytes" value={String(result.byte_count)} />
        <Readout label="Lines" value={String(lineCount)} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
      </div>
      <p className="break-all text-xs text-zinc-500">
        {result.source.slice(0, 96)}
      </p>
    </div>
  );
}

function ObjectLinesBinaryExportReadout({ result }: { result: ObjectLinesBinaryExportResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No ObjectLines binary export has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">ObjectLines Binary</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'object_lines_to_mrlines')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="File" value={result.file_name} />
        <Readout label="Bytes" value={String(result.byte_count)} />
        <Readout label="Payload" value={`${result.contents_base64.length} chars`} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
      </div>
    </div>
  );
}

function ObjectLinesContoursReadout({ result }: { result: ObjectLinesToContoursResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No restored contours have been captured yet.</p>;
  }

  const firstPoint = result.contours[0]?.[0] ?? null;

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Restored Contours</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'object_lines_to_contours')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Contours" value={String(result.contour_count)} />
        <Readout label="Points" value={String(result.point_count)} />
        <Readout label="First Size" value={String(result.contours[0]?.length ?? 0)} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
      </div>
      {firstPoint ? (
        <p className="text-xs text-zinc-500">
          First point: {firstPoint.map((coordinate) => coordinate.toFixed(3)).join(', ')}
        </p>
      ) : null}
    </div>
  );
}

function PointCloudIcpReadout({ result }: { result: PointCloudIcpResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No ICP registration has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">ICP Result</p>
        <span className="text-xs text-zinc-500">{result.method.replaceAll('_', ' ')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Mode" value={result.mode} />
        <Readout label="Pairs" value={String(result.active_pair_count)} />
        <Readout label="Iterations" value={String(result.iterations)} />
        <Readout label="MSD" value={result.mean_square_distance.toExponential(2)} />
      </div>
      <p className="text-xs text-zinc-500">
        Translation: {result.translation.map((coordinate) => coordinate.toFixed(4)).join(', ')}
      </p>
    </div>
  );
}

function VoxelVolumeLoadReadout({ result }: { result: VoxelVolumeLoadResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No voxel volume load has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Voxel Volume</p>
        <span className="text-xs text-zinc-500">{String(result.metadata.sdk_operation ?? 'load_raw_voxels')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Grid" value={result.dimensions.join(' x ')} />
        <Readout label="Values" value={String(result.value_count)} />
        <Readout label="Scalar" value={result.scalar_type} />
        <Readout label="Iso" value={result.default_iso_value == null ? 'n/a' : result.default_iso_value.toFixed(3)} />
        <Readout label="Min" value={result.min_value.toFixed(3)} />
        <Readout label="Max" value={result.max_value.toFixed(3)} />
        <Readout label="Voxel" value={result.voxel_size.map((value) => value.toFixed(2)).join(', ')} />
        <Readout label="Rust" value={result.metadata.rust_backed === true ? 'Yes' : 'No'} />
      </div>
    </div>
  );
}

function MeshToVoxelsReadout({ result }: { result: MeshToVoxelsSdfResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No voxel conversion has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Voxel Result</p>
        <span className="text-xs capitalize text-zinc-500">{result.mode}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Grid" value={result.shape.join(' x ')} />
        <Readout label="Values" value={String(result.value_count)} />
        <Readout label="Active" value={String(result.active_voxel_count)} />
        <Readout label="Volume" value={`${result.estimated_volume_mm3.toFixed(2)} mm3`} />
        <Readout label="Min" value={result.min_value.toFixed(3)} />
        <Readout label="Max" value={result.max_value.toFixed(3)} />
        <Readout label="Vertices" value={String(result.surface_vertex_count)} />
        <Readout label="Faces" value={String(result.surface_face_count)} />
      </div>
      <p className="text-xs text-zinc-500">
        Origin: {result.origin.map((coordinate) => coordinate.toFixed(2)).join(', ')}
      </p>
    </div>
  );
}

function VoxelVolumeRenderRayReadout({ result }: { result: VoxelVolumeRenderRayResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No volume ray sample has been captured yet.</p>;
  }

  const rgba = result.color_rgba.map((channel) => Number(channel).toFixed(3)).join(', ');
  const firstOpaque = result.first_opaque_world
    ? result.first_opaque_world.map((coordinate) => coordinate.toFixed(3)).join(', ')
    : 'none';

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Volume Ray Result</p>
        <span className="text-xs text-zinc-500">{result.version_id}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="RGBA" value={rgba} />
        <Readout label="Visited" value={String(result.visited_indices.length)} />
        <Readout label="Accepted" value={String(result.accepted_indices.length)} />
        <Readout label="First Opaque" value={firstOpaque} />
      </div>
    </div>
  );
}

function OffsetShellReadout({ result }: { result: OffsetShellMeshResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No offset or shell result has been captured yet.</p>;
  }

  const amount =
    result.mode === 'offset'
      ? result.offset_mm != null
        ? `${result.offset_mm.toFixed(3)} mm`
        : 'n/a'
      : result.mode === 'shell' && result.wall_thickness_mm != null
        ? `${result.wall_thickness_mm.toFixed(3)} mm`
        : result.mode === 'thicken' && result.thickness_mm != null
          ? `${result.thickness_mm.toFixed(3)} mm`
          : result.mode === 'weighted_shell' && result.offset_mm != null
            ? `${result.offset_mm.toFixed(3)} mm`
        : result.distance_mm != null
          ? `${result.distance_mm.toFixed(3)} mm`
          : 'n/a';

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Offset Result</p>
        <span className="text-xs capitalize text-zinc-500">{result.mode}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Version" value={result.version.id} />
        <Readout label="Amount" value={amount} />
        <Readout label="Voxel Size" value={`${result.voxel_size_mm.toFixed(3)} mm`} />
        <Readout label="Artifact" value={result.artifact_id} />
        <Readout label="Vertices" value={String(result.output_vertex_count)} />
        <Readout label="Faces" value={String(result.output_face_count)} />
      </div>
    </div>
  );
}

function ExactBooleanReadout({ result }: { result: ExactBooleanResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No boolean result has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Boolean Result</p>
        <span className="text-xs capitalize text-zinc-500">{result.operation.replaceAll('_', ' ')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Version" value={result.version.id} />
        <Readout label="Artifact" value={result.artifact_id} />
        <Readout label="Vertices" value={String(result.output_vertex_count)} />
        <Readout label="Faces" value={String(result.output_face_count)} />
      </div>
      <p className="text-xs text-zinc-500">
        Source {result.source_version_id} with {result.other_version_id}
      </p>
    </div>
  );
}

function VoxelBooleanReadout({ result }: { result: VoxelBooleanResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No voxel boolean result has been captured yet.</p>;
  }

  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Voxel Boolean Result</p>
        <span className="text-xs capitalize text-zinc-500">{result.operation.replaceAll('_', ' ')}</span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Version" value={result.version.id} />
        <Readout label="Voxel Size" value={`${result.voxel_size_mm.toFixed(2)} mm`} />
        <Readout label="Vertices" value={String(result.output_vertex_count)} />
        <Readout label="Faces" value={String(result.output_face_count)} />
      </div>
      <p className="text-xs text-zinc-500">
        Source {result.source_version_id} with {result.other_version_id}
      </p>
    </div>
  );
}

function CollisionReadout({ result }: { result: CollisionDetectResponse | null }) {
  if (!result) {
    return <p className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-500">No collision result has been captured yet.</p>;
  }

  const firstPair = result.pairs[0] ?? null;
  return (
    <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-zinc-100">Collision Result</p>
        <span className={result.colliding ? 'text-xs text-amber-300' : 'text-xs text-emerald-300'}>
          {result.colliding ? 'Colliding' : 'Clear'}
        </span>
      </div>
      <div className="grid grid-cols-2 gap-2 text-xs text-zinc-400">
        <Readout label="Pairs" value={String(result.pair_count)} />
        <Readout label="First Faces" value={String(result.first_face_indices.length)} />
        <Readout label="Second Faces" value={String(result.second_face_indices.length)} />
        <Readout label="Truncated" value={result.truncated ? 'Yes' : 'No'} />
      </div>
      {firstPair ? (
        <p className="text-xs text-zinc-500">
          First pair: face {firstPair.first_face} vs {firstPair.second_face}, {firstPair.intersection_count} exact intersections
        </p>
      ) : null}
    </div>
  );
}

function Readout({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-zinc-800 px-2 py-2">
      <p className="text-[10px] uppercase tracking-[0.16em] text-zinc-500">{label}</p>
      <p className="mt-1 text-sm text-zinc-200">{value}</p>
    </div>
  );
}

function parseEdgePairs(value: string): [number, number][] {
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

function parseIndexList(value: string): number[] {
  return value
    .split(/[,\n;\s]+/)
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => Number(item))
    .filter((item) => Number.isInteger(item) && item >= 0);
}

function getSelectedRegionOperationReason(
  selectedRegion: RegionManifestEntry | null,
  operation: string,
  label: string,
) {
  if (!selectedRegion) {
    return 'Select a primary region first.';
  }
  if (!selectedRegion.allowed_operations.includes(operation)) {
    return `${selectedRegion.label} does not allow ${label}.`;
  }
  return null;
}

function getBatchRegionOperationReason(
  batchRegions: RegionManifestEntry[],
  selectedRegionIds: string[],
  operation: string,
  label: string,
) {
  if (selectedRegionIds.length < 2) {
    return 'Batch commands require at least 2 selected regions.';
  }
  if (batchRegions.length !== selectedRegionIds.length) {
    return 'One or more selected regions is no longer available.';
  }
  const blockedRegion = batchRegions.find((region) => !region.allowed_operations.includes(operation));
  if (blockedRegion) {
    return `${blockedRegion.label} does not allow ${label}.`;
  }
  return null;
}

function getScoopEligibility(
  regions: RegionManifestEntry[],
  selectedRegion: RegionManifestEntry | null,
  scoopDepth: number,
  keepMinThickness: number,
): {
  region: RegionManifestEntry | null;
  reason: string | null;
} {
  const requiredThickness = scoopDepth + keepMinThickness;
  const candidates = regions.filter((region) => region.allowed_operations.includes('scoop') && region.vertex_count > 0);

  const isEligible = (region: RegionManifestEntry) =>
    region.min_thickness_mm == null || region.min_thickness_mm >= requiredThickness;

  if (selectedRegion?.allowed_operations.includes('scoop')) {
    if (isEligible(selectedRegion)) {
      return { region: selectedRegion, reason: null };
    }
    return {
      region: selectedRegion,
      reason:
        `Selected region ${selectedRegion.label} is too thin for a ${scoopDepth.toFixed(2)} mm scoop while keeping ` +
        `${keepMinThickness.toFixed(2)} mm minimum thickness. Thicken it first or reduce scoop depth.`,
    };
  }

  const fallback = candidates.find(isEligible) ?? null;
  if (fallback) {
    return { region: fallback, reason: null };
  }

  if (candidates.length > 0) {
    return {
      region: candidates[0],
      reason:
        `No scoop-safe region can support a ${scoopDepth.toFixed(2)} mm scoop with ${keepMinThickness.toFixed(2)} mm minimum thickness. ` +
        'Thicken the mesh first or reduce scoop depth.',
    };
  }

  return { region: null, reason: 'No scoop-safe region is available on this mesh.' };
}
