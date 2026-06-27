'use client';

import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type { JobResponse, MeshLibRuntimeManifest, MeshLibWorkbenchManifest } from '@/lib/api/types';
import type { WorkspaceCommandId } from '../workspace/types';

export type WorkbenchHostCommandPayload = {
  commandId: WorkspaceCommandId;
  payload: Record<string, unknown>;
  options: Record<string, unknown>;
  endpointUrl: string | null;
  endpointUrlKey: string | null;
  rustBacked: boolean;
  sdkOperations: string[];
};

type MeshLibWorkbenchHostProps = {
  manifest: MeshLibWorkbenchManifest | null;
  onJobSubmitted?: (job: JobResponse) => void;
  onWorkspaceCommand?: (command: WorkbenchHostCommandPayload) => void;
  children: ReactNode;
};

type WorkbenchHostCommandMessagePayload = {
  command_id?: string;
  payload?: unknown;
  options?: unknown;
  endpoint_url?: string | null;
  endpoint_url_key?: string | null;
  rust_backed?: boolean;
  sdk_operations?: unknown;
};

type WorkbenchMessage =
  | { type: 'meshlib-workbench:request-init' }
  | { type: 'meshlib-workbench:ready' }
  | { type: 'meshlib-workbench:commit-complete'; payload?: { version_id?: string; job?: JobResponse } }
  | { type: 'meshlib-workbench:commit-failed'; payload?: { version_id?: string; error?: string } }
  | {
      type: 'meshlib-workbench:selection-complete';
      payload?: {
        version_id?: string;
        source_version_id?: string;
        selected_object_version_id?: string | null;
        selection?: unknown;
      };
    }
  | { type: 'meshlib-workbench:selection-failed'; payload?: { version_id?: string; error?: string } }
  | { type: 'meshlib-workbench:brush-complete'; payload?: { version_id?: string; job?: JobResponse } }
  | { type: 'meshlib-workbench:brush-failed'; payload?: { version_id?: string; error?: string } }
  | { type: 'meshlib-workbench:measure-complete'; payload?: { version_id?: string } }
  | { type: 'meshlib-workbench:measure-failed'; payload?: { version_id?: string; error?: string } }
  | { type: 'meshlib-workbench:host-command'; payload?: WorkbenchHostCommandMessagePayload }
  | { type: 'meshlib-workbench:command-failed'; payload?: { command_id?: string; error?: string } };

function recordFromUnknown(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }
  return value as Record<string, unknown>;
}

function stringArrayFromUnknown(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is string => typeof item === 'string');
}

const RUNTIME_HOST_COMMAND_ALIASES: Record<string, WorkspaceCommandId> = {
  'runtime-select-mark-region': 'regions',
  'runtime-thicken-brush': 'runtime-thicken-brush',
  'runtime-scoop-brush': 'runtime-scoop-brush',
  'runtime-smooth-brush': 'runtime-smooth-brush',
  'runtime-measure-inspect': 'measure-inspect',
  'G-code Path Parser': 'gcode-parse-paths',
  'gcode-parse-paths': 'gcode-parse-paths',
  gcode_parse_paths: 'gcode-parse-paths',
  GcodeParsePathsTool: 'gcode-parse-paths',
  'Load G-code Source': 'gcode-load-source',
  'gcode-load-source': 'gcode-load-source',
  gcode_load_source: 'gcode-load-source',
  GcodeLoadSourceTool: 'gcode-load-source',
  'Write G-code Source': 'gcode-write-source',
  'gcode-write-source': 'gcode-write-source',
  gcode_write_source: 'gcode-write-source',
  GcodeWriteSourceTool: 'gcode-write-source',
  'Parse G-code File Paths': 'gcode-parse-file-paths',
  'gcode-parse-file-paths': 'gcode-parse-file-paths',
  gcode_parse_file_paths: 'gcode-parse-file-paths',
  GcodeParseFilePathsTool: 'gcode-parse-file-paths',
  'Decimate Mesh': 'decimate-mesh',
  decimate_mesh: 'decimate-mesh',
  DecimateMeshTool: 'decimate-mesh',
  'Make Delone': 'make-delone',
  make_delone: 'make-delone',
  MakeDeloneTool: 'make-delone',
  'Exact Boolean': 'exact-boolean',
  exact_boolean: 'exact-boolean',
  ExactBooleanTool: 'exact-boolean',
  'Voxel Boolean': 'voxel-boolean',
  voxel_boolean: 'voxel-boolean',
  VoxelBooleanTool: 'voxel-boolean',
  'Binary Operations': 'voxel-binary-operations',
  'voxel-binary-operations': 'voxel-binary-operations',
  voxel_binary_operations: 'voxel-binary-operations',
  BinaryOperationsTool: 'voxel-binary-operations',
  'Voxels Line Graph': 'voxel-line-graph',
  'voxel-line-graph': 'voxel-line-graph',
  voxel_line_graph: 'voxel-line-graph',
  VoxelsLineGraphTool: 'voxel-line-graph',
  'Set Active Voxel Box': 'voxel-active-box',
  'voxel-active-box': 'voxel-active-box',
  voxel_active_box: 'voxel-active-box',
  SetActiveVoxelBoxTool: 'voxel-active-box',
  'Voxels Slice': 'voxel-slice',
  'voxel-slice': 'voxel-slice',
  voxel_slice: 'voxel-slice',
  VoxelsSliceTool: 'voxel-slice',
  'Voxels Segmentation': 'voxel-segmentation',
  'voxel-segmentation': 'voxel-segmentation',
  voxel_segmentation: 'voxel-segmentation',
  VoxelsSegmentationTool: 'voxel-segmentation',
  'Voxels Mask to Mesh': 'voxel-mask-to-mesh',
  'voxel-mask-to-mesh': 'voxel-mask-to-mesh',
  voxel_mask_to_mesh: 'voxel-mask-to-mesh',
  VoxelsMaskToMeshTool: 'voxel-mask-to-mesh',
  'Voxels Path': 'voxel-path',
  'voxel-path': 'voxel-path',
  voxel_path: 'voxel-path',
  VoxelsPathTool: 'voxel-path',
  'Voxels Path Build Four': 'voxel-path-build-four',
  'voxel-path-build-four': 'voxel-path-build-four',
  voxel_path_build_four: 'voxel-path-build-four',
  VoxelsPathBuildFourTool: 'voxel-path-build-four',
  'Voxels to Mesh Simple': 'voxel-to-mesh-simple',
  'voxel-to-mesh-simple': 'voxel-to-mesh-simple',
  voxel_to_mesh_simple: 'voxel-to-mesh-simple',
  VoxelsToMeshSimpleTool: 'voxel-to-mesh-simple',
  'Voxels to Mesh Dual': 'voxel-to-mesh-dual',
  'voxel-to-mesh-dual': 'voxel-to-mesh-dual',
  voxel_to_mesh_dual: 'voxel-to-mesh-dual',
  VoxelsToMeshDualTool: 'voxel-to-mesh-dual',
  'Voxels to Mesh Smart': 'voxel-to-mesh-smart',
  'voxel-to-mesh-smart': 'voxel-to-mesh-smart',
  voxel_to_mesh_smart: 'voxel-to-mesh-smart',
  VoxelsToMeshSmartTool: 'voxel-to-mesh-smart',
  'Voxels Volume Rendering Data': 'voxel-volume-render-data',
  'voxel-volume-render-data': 'voxel-volume-render-data',
  voxel_volume_render_data: 'voxel-volume-render-data',
  VoxelsVolumeRenderingDataTool: 'voxel-volume-render-data',
  'Voxels Volume Rendering LUT': 'voxel-volume-render-lut',
  'voxel-volume-render-lut': 'voxel-volume-render-lut',
  voxel_volume_render_lut: 'voxel-volume-render-lut',
  VoxelsVolumeRenderingLutTool: 'voxel-volume-render-lut',
  'Voxels Volume Rendering Ray': 'voxel-volume-render-ray',
  'voxel-volume-render-ray': 'voxel-volume-render-ray',
  voxel_volume_render_ray: 'voxel-volume-render-ray',
  VoxelsVolumeRenderingRayTool: 'voxel-volume-render-ray',
  'Offset Mesh': 'offset-verts',
  offset_mesh: 'offset-verts',
  OffsetMeshTool: 'offset-verts',
  'Shell Mesh': 'shell-mesh',
  shell_mesh: 'shell-mesh',
  ShellMeshTool: 'shell-mesh',
  'Thickening': 'thicken-mesh',
  thicken_mesh: 'thicken-mesh',
  ThickeningTool: 'thicken-mesh',
  'Weighted Shell': 'weighted-shell',
  weighted_shell: 'weighted-shell',
  WeightedShellTool: 'weighted-shell',
  'Partial Offset': 'partial-offset',
  partial_offset: 'partial-offset',
  PartialOffsetTool: 'partial-offset',
  'Offset Verts': 'offset-verts',
  offset_verts: 'offset-verts',
  OffsetVertsTool: 'offset-verts',
  'Offset Contours': 'offset-contours',
  offset_contours: 'offset-contours',
  OffsetContoursTool: 'offset-contours',
  'Mesh Distance Map': 'distance-map-from-mesh',
  'Distance Map From Mesh': 'distance-map-from-mesh',
  distance_map_from_mesh: 'distance-map-from-mesh',
  DistanceMapFromMeshTool: 'distance-map-from-mesh',
  'Contour Distance Map': 'distance-map-contours',
  'Distance Map From Contours': 'distance-map-contours',
  distance_map_from_contours: 'distance-map-contours',
  DistanceMapContoursTool: 'distance-map-contours',
  'Distance Map Iso-Lines': 'distance-map-iso-lines',
  'Distance Map Iso Lines': 'distance-map-iso-lines',
  distance_map_to_iso_segments: 'distance-map-iso-lines',
  DistanceMapIsoLinesTool: 'distance-map-iso-lines',
  'Distance Map Merge': 'distance-map-merge',
  distance_map_merge: 'distance-map-merge',
  DistanceMapMergeTool: 'distance-map-merge',
  'Contour Boolean From Distance Maps': 'distance-map-contour-boolean',
  'Contour Boolean': 'distance-map-contour-boolean',
  distance_map_contour_boolean: 'distance-map-contour-boolean',
  DistanceMapContourBooleanTool: 'distance-map-contour-boolean',
  'TIFF Distance Map Import': 'distance-map-from-tiff',
  'Distance Map From TIFF': 'distance-map-from-tiff',
  distance_map_from_tiff: 'distance-map-from-tiff',
  DistanceMapFromTiffTool: 'distance-map-from-tiff',
  'TIFF Distance Map Export': 'distance-map-to-tiff',
  'Distance Map To TIFF': 'distance-map-to-tiff',
  distance_map_to_tiff: 'distance-map-to-tiff',
  DistanceMapToTiffTool: 'distance-map-to-tiff',
  'ObjectLines From Contours': 'object-lines-from-contours',
  object_lines_from_contours: 'object-lines-from-contours',
  ObjectLinesFromContoursTool: 'object-lines-from-contours',
  'ObjectLines Load MrLines': 'object-lines-load-mrlines',
  object_lines_from_mrlines: 'object-lines-load-mrlines',
  ObjectLinesLoadMrLinesTool: 'object-lines-load-mrlines',
  'ObjectLines Save MrLines': 'object-lines-save-mrlines',
  object_lines_to_mrlines: 'object-lines-save-mrlines',
  ObjectLinesSaveMrLinesTool: 'object-lines-save-mrlines',
  'ObjectLines Load PLY': 'object-lines-load-ply',
  object_lines_from_ply: 'object-lines-load-ply',
  ObjectLinesLoadPlyTool: 'object-lines-load-ply',
  'ObjectLines Save PLY': 'object-lines-save-ply',
  object_lines_to_ply: 'object-lines-save-ply',
  ObjectLinesSavePlyTool: 'object-lines-save-ply',
  'ObjectLines Load PTS': 'object-lines-load-pts',
  object_lines_from_pts: 'object-lines-load-pts',
  ObjectLinesLoadPtsTool: 'object-lines-load-pts',
  'ObjectLines Save PTS': 'object-lines-save-pts',
  object_lines_to_pts: 'object-lines-save-pts',
  ObjectLinesSavePtsTool: 'object-lines-save-pts',
  'ObjectLines Load SVG': 'object-lines-load-svg',
  object_lines_from_svg: 'object-lines-load-svg',
  ObjectLinesLoadSvgTool: 'object-lines-load-svg',
  'ObjectLines Save DXF': 'object-lines-save-dxf',
  object_lines_to_dxf: 'object-lines-save-dxf',
  ObjectLinesSaveDxfTool: 'object-lines-save-dxf',
  'ObjectLines To Contours': 'object-lines-to-contours',
  object_lines_to_contours: 'object-lines-to-contours',
  ObjectLinesToContoursTool: 'object-lines-to-contours',
  'Mesh Cut & Measure Path': 'mesh-cut-measure-path',
  mesh_cut_measure_path: 'mesh-cut-measure-path',
  mesh_cut_and_measure: 'mesh-cut-measure-path',
  MeshCutMeasurePathTool: 'mesh-cut-measure-path',
  'Point Cloud / ICP': 'point-cloud-icp',
  'point-cloud-icp': 'point-cloud-icp',
  point_cloud_icp: 'point-cloud-icp',
  PointCloudIcpTool: 'point-cloud-icp',
  'Open RAW Voxels': 'open-raw-voxels',
  open_raw_voxels: 'open-raw-voxels',
  load_raw_voxels: 'open-raw-voxels',
  OpenRawVoxelsTool: 'open-raw-voxels',
  'Open Voxels From TIFF': 'open-voxels-from-tiff',
  open_voxels_from_tiff: 'open-voxels-from-tiff',
  load_tiff_voxels_dir: 'open-voxels-from-tiff',
  OpenVoxelsFromTiffTool: 'open-voxels-from-tiff',
  'Expand/Shrink': 'expand-shrink',
  expand_shrink: 'expand-shrink',
  ExpandShrinkTool: 'expand-shrink',
  'Shrink/Expand': 'shrink-expand',
  shrink_expand: 'shrink-expand',
  ShrinkExpandTool: 'shrink-expand',
};

function workspaceCommandIdFromHostCommandId(commandId: string): WorkspaceCommandId {
  return RUNTIME_HOST_COMMAND_ALIASES[commandId] ?? (commandId as WorkspaceCommandId);
}

export default function MeshLibWorkbenchHost({ manifest, onJobSubmitted, onWorkspaceCommand, children }: MeshLibWorkbenchHostProps) {
  const queryClient = useQueryClient();
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const [runtimeManifest, setRuntimeManifest] = useState<MeshLibRuntimeManifest | null>(null);
  const [runtimeLoadError, setRuntimeLoadError] = useState<string | null>(null);
  const [workbenchReady, setWorkbenchReady] = useState(false);
  const apiBase = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

  useEffect(() => {
    let active = true;
    setRuntimeLoadError(null);
    void fetch('/meshlib-workbench/runtime/manifest.json', { cache: 'no-store' })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error(`Runtime manifest unavailable (${response.status})`);
        }
        return response.json() as Promise<MeshLibRuntimeManifest>;
      })
      .then((payload) => {
        if (!active) {
          return;
        }
        setRuntimeManifest(payload);
      })
      .catch((error) => {
        if (!active) {
          return;
        }
        setRuntimeManifest({
          status: 'missing',
          message: error instanceof Error ? error.message : 'Runtime manifest unavailable',
        });
        setRuntimeLoadError(error instanceof Error ? error.message : 'Runtime manifest unavailable');
      });

    return () => {
      active = false;
    };
  }, []);

  const normalizedManifest = useMemo<MeshLibWorkbenchManifest | null>(() => {
    if (!manifest) {
      return null;
    }
    const absolutize = (value: string | null | undefined) => {
      if (!value) {
        return value ?? null;
      }
      if (value.startsWith('http://') || value.startsWith('https://')) {
        return value;
      }
      return `${apiBase}${value}`;
    };

    return {
      ...manifest,
      normalized_mesh_url: absolutize(manifest.normalized_mesh_url),
      meshlib_scene_object_url: absolutize(manifest.meshlib_scene_object_url),
      meshlib_scene_mru_url: absolutize(manifest.meshlib_scene_mru_url),
      texture_artifact_url: absolutize(manifest.texture_artifact_url),
      texture_artifacts: manifest.texture_artifacts.map((texture) => ({
        ...texture,
        artifact_url: absolutize(texture.artifact_url) || texture.artifact_url,
      })),
      preview_low_url: absolutize(manifest.preview_low_url),
      preview_high_url: absolutize(manifest.preview_high_url),
      commit_endpoint_url: absolutize(manifest.commit_endpoint_url) || manifest.commit_endpoint_url,
      selection_endpoint_url: absolutize(manifest.selection_endpoint_url) || manifest.selection_endpoint_url,
      brush_endpoint_url: absolutize(manifest.brush_endpoint_url) || manifest.brush_endpoint_url,
      measurement_endpoint_url: absolutize(manifest.measurement_endpoint_url) || manifest.measurement_endpoint_url,
      mesh_cut_measure_topology_endpoint_url:
        absolutize(manifest.mesh_cut_measure_topology_endpoint_url) ||
        manifest.mesh_cut_measure_topology_endpoint_url,
      command_capabilities: manifest.command_capabilities.map((capability) => ({
        ...capability,
        endpoint_url: absolutize(capability.endpoint_url),
      })),
    };
  }, [apiBase, manifest]);

  useEffect(() => {
    setWorkbenchReady(false);
  }, [normalizedManifest?.version_id]);

  useEffect(() => {
    const handleMessage = (event: MessageEvent<WorkbenchMessage>) => {
      if (event.origin !== window.location.origin) {
        return;
      }
      if (event.source !== iframeRef.current?.contentWindow) {
        return;
      }
      if (event.data?.type === 'meshlib-workbench:request-init') {
        iframeRef.current?.contentWindow?.postMessage(
          {
            type: 'meshlib-workbench:init',
            payload: {
              manifest: normalizedManifest,
              runtimeManifest,
            },
          },
          window.location.origin,
        );
      }
      if (event.data?.type === 'meshlib-workbench:ready') {
        setWorkbenchReady(true);
      }
      if (event.data?.type === 'meshlib-workbench:host-command' && event.data.payload?.command_id && onWorkspaceCommand) {
        const hostCommand = event.data.payload;
        const hostCommandId = hostCommand.command_id;
        if (!hostCommandId) {
          return;
        }
        const command: WorkbenchHostCommandPayload = {
          commandId: workspaceCommandIdFromHostCommandId(hostCommandId),
          payload: recordFromUnknown(hostCommand.payload),
          options: recordFromUnknown(hostCommand.options),
          endpointUrl: hostCommand.endpoint_url ?? null,
          endpointUrlKey: hostCommand.endpoint_url_key ?? null,
          rustBacked: hostCommand.rust_backed === true,
          sdkOperations: stringArrayFromUnknown(hostCommand.sdk_operations),
        };
        onWorkspaceCommand(command);
      }
      if (
        event.data?.type === 'meshlib-workbench:commit-complete' ||
        event.data?.type === 'meshlib-workbench:brush-complete'
      ) {
        if (event.data.payload?.job?.id) {
          onJobSubmitted?.(event.data.payload.job);
        }
      }
      if (
        event.data?.type === 'meshlib-workbench:commit-complete' ||
        event.data?.type === 'meshlib-workbench:selection-complete' ||
        event.data?.type === 'meshlib-workbench:brush-complete'
      ) {
        const committedVersionId = event.data.payload?.version_id || normalizedManifest?.version_id;
        if (committedVersionId) {
          queryClient.invalidateQueries({ queryKey: ['version', committedVersionId] });
          queryClient.invalidateQueries({ queryKey: ['viewer-manifest', committedVersionId] });
          queryClient.invalidateQueries({ queryKey: ['meshlib-workbench', committedVersionId] });
          queryClient.invalidateQueries({ queryKey: ['version-jobs', committedVersionId] });
        }
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [normalizedManifest, onJobSubmitted, onWorkspaceCommand, queryClient, runtimeManifest]);

  useEffect(() => {
    if (!iframeRef.current?.contentWindow || !normalizedManifest || runtimeManifest?.status !== 'ready') {
      return;
    }
    iframeRef.current.contentWindow.postMessage(
      {
        type: 'meshlib-workbench:init',
        payload: {
          manifest: normalizedManifest,
          runtimeManifest,
        },
      },
      window.location.origin,
    );
  }, [normalizedManifest, runtimeManifest]);

  const runtimeBundleReady = runtimeManifest?.status === 'ready';
  const runtimeReady = runtimeBundleReady && !!manifest;
  const iframeSrc = useMemo(() => {
    if (!manifest) {
      return null;
    }
    return manifest.entry_html_url;
  }, [manifest]);

  if (runtimeReady && iframeSrc) {
    return (
      <div className="relative h-screen w-full">
        <iframe
          ref={iframeRef}
          title="MeshLib Workbench"
          src={iframeSrc}
          className="h-full w-full border-0 bg-zinc-950"
          allow="clipboard-read; clipboard-write"
        />
        {!workbenchReady && (
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-zinc-950/40 backdrop-blur-[1px]">
            <div className="rounded-2xl border border-zinc-800 bg-zinc-950/90 px-4 py-3 text-sm text-zinc-300">
              Booting MeshLib workbench runtime...
            </div>
          </div>
        )}
      </div>
    );
  }

  if (runtimeBundleReady && !manifest) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-zinc-950 text-sm text-zinc-400">
        Loading MeshLib workbench manifest...
      </div>
    );
  }

  return (
    <div className="relative h-full w-full">
      {children}
      <div className="pointer-events-none absolute bottom-4 right-4 z-20 max-w-sm rounded-2xl border border-amber-500/20 bg-zinc-950/92 px-4 py-3 text-xs text-zinc-300 shadow-[0_18px_48px_rgba(0,0,0,0.35)] backdrop-blur">
        <p className="text-[10px] uppercase tracking-[0.24em] text-amber-300">MeshLib Workbench</p>
        <p className="mt-2 leading-5 text-zinc-300">
          The MeshLib workbench host is active, but the runtime bundle is not available in
          <span className="mx-1 rounded bg-zinc-900 px-1.5 py-0.5 text-zinc-100">/public/meshlib-workbench/runtime</span>.
          The fallback viewer remains active until the MeshLib WASM workbench is available.
        </p>
        {(runtimeManifest?.message || runtimeLoadError) && (
          <p className="mt-2 text-zinc-500">{runtimeManifest?.message || runtimeLoadError}</p>
        )}
      </div>
    </div>
  );
}
