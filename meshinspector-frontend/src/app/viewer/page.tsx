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
import {
  useCreateInspectionSnapshot,
  useBranchVersion,
  useCompareCache,
  useCompareOperation,
  useCompareOverlay,
  useHollowOperation,
  useInspectionSnapshots,
  useMakeManufacturableOperation,
  useManufacturability,
  useModelVersions,
  useRepairOperation,
  useResizeOperation,
  useScoopOperation,
  useSmoothOperation,
  useThicknessOverlay,
  useThickenOperation,
  useVersion,
  useViewerManifest,
} from '@/hooks/useModelProcessing';
import type {
  HollowRequestV2,
  InspectionSnapshotResponse,
  MakeManufacturableRequest,
  ManufacturabilitySnapshot,
  MaterialType,
  RegionManifestEntry,
  ResizeRequestV2,
  ScoopRequestV2,
  SectionContourPayload,
  SmoothRequestV2,
  ThickenRequestV2,
} from '@/lib/api/types';
import { getArtifactUrl } from '@/lib/api/client';

const ViewerEngine = dynamic(() => import('@/features/editor/viewer/ViewerEngine'), { ssr: false });

function normalizeAxis(axis: [number, number, number] | null | undefined): [number, number, number] {
  if (!axis) return [0, 1, 0];
  const length = Math.hypot(axis[0], axis[1], axis[2]);
  if (length < 1e-8) return [0, 1, 0];
  return [axis[0] / length, axis[1] / length, axis[2] / length];
}

function createPlaneBasis(axis: [number, number, number]) {
  const normal = normalizeAxis(axis);
  const reference = Math.abs(normal[1]) < 0.95 ? [0, 1, 0] : [1, 0, 0];
  const u: [number, number, number] = normalizeAxis([
    reference[1] * normal[2] - reference[2] * normal[1],
    reference[2] * normal[0] - reference[0] * normal[2],
    reference[0] * normal[1] - reference[1] * normal[0],
  ]);
  const v: [number, number, number] = normalizeAxis([
    normal[1] * u[2] - normal[2] * u[1],
    normal[2] * u[0] - normal[0] * u[2],
    normal[0] * u[1] - normal[1] * u[0],
  ]);
  return { normal, u, v };
}

function dot(point: [number, number, number], axis: [number, number, number]) {
  return point[0] * axis[0] + point[1] * axis[1] + point[2] * axis[2];
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
  const [sectionContour, setSectionContour] = useState<SectionContourPayload | null>(null);
  const [resizeAxisMode, setResizeAxisMode] = useState<'auto' | 'manual'>('auto');
  const [manualResizeAxis, setManualResizeAxis] = useState<[number, number, number] | null>(null);
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

  const versionsQuery = useModelVersions(modelId);
  const versionDetailQuery = useVersion(versionId);
  const compareCacheQuery = useCompareCache(versionId);
  const inspectionSnapshotsQuery = useInspectionSnapshots(versionId);
  const viewerQuery = useViewerManifest(versionId);
  const snapshotQuery = useManufacturability(versionId);
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
  const repairMutation = useRepairOperation();
  const resizeMutation = useResizeOperation();
  const hollowMutation = useHollowOperation();
  const thickenMutation = useThickenOperation();
  const compareMutation = useCompareOperation();
  const scoopMutation = useScoopOperation();
  const smoothMutation = useSmoothOperation();
  const makeMutation = useMakeManufacturableOperation();
  const createInspectionSnapshotMutation = useCreateInspectionSnapshot();
  const branchVersionMutation = useBranchVersion();

  const submitAndTrack = useCallback(async (promise: Promise<unknown>) => {
    const job = await promise as { id: string };
    setActiveJobId(job.id);
  }, []);

  const currentJob = useJobPolling(activeJobId, useCallback((nextVersionId) => {
    if (modelId) {
      void queryClient.invalidateQueries({ queryKey: ['model-versions', modelId] });
    }
    if (versionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['manufacturability', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['compare-cache', versionId] });
      void queryClient.invalidateQueries({ queryKey: ['inspection-snapshots', versionId] });
    }
    if (nextVersionId) {
      void queryClient.invalidateQueries({ queryKey: ['version', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['viewer-manifest', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['manufacturability', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['compare-cache', nextVersionId] });
      void queryClient.invalidateQueries({ queryKey: ['inspection-snapshots', nextVersionId] });
    }
    setVersionId(nextVersionId);
    setActiveJobId(null);
  }, [modelId, queryClient, versionId]));
  const jobEvents = useJobEventStream(activeJobId);

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
    if (Number.isFinite(planeValue)) {
      setSectionConstant(planeValue);
    }

    const regionId = searchParams.get('region');
    if (regionId) {
      setSelectedRegionId(regionId);
    }
    const selectedIds = searchParams.get('regions_selected');
    if (selectedIds) {
      setSelectedRegionIds(selectedIds.split(',').filter(Boolean));
    }
    const axisMode = searchParams.get('axis_mode');
    if (axisMode === 'auto' || axisMode === 'manual') {
      setResizeAxisMode(axisMode);
    }
    const axis = searchParams.get('axis');
    if (axis) {
      const values = axis.split(',').map(Number);
      if (values.length === 3 && values.every(Number.isFinite)) {
        setManualResizeAxis([values[0], values[1], values[2]]);
      }
    }

    const compareTarget = searchParams.get('compare_target');
    if (compareTarget) {
      setCompareTargetVersionId(compareTarget);
    }
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
      !!activeJobId
    ) {
      return;
    }
    void submitAndTrack(compareMutation.mutateAsync({ versionId, params: { other_version_id: compareTargetVersionId } }));
  }, [
    activeJobId,
    compareCacheTargets,
    compareMutation,
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
  const scalarOverlay = compareOverlayEnabled ? compareOverlayQuery.data ?? null : heatmapEnabled ? thicknessOverlayQuery.data ?? null : null;
  const sectionAxis = useMemo<[number, number, number]>(() => {
    if (resizeAxisMode === 'manual' && manualResizeAxis) {
      return normalizeAxis(manualResizeAxis);
    }
    const detected = snapshotQuery.data?.dimensions.ring_axis ?? null;
    return normalizeAxis(detected);
  }, [manualResizeAxis, resizeAxisMode, snapshotQuery.data?.dimensions.ring_axis]);
  const selectedRegion =
    (viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? []).find((region) => region.region_id === selectedRegionId) ?? null;
  const sectionPresets = useMemo(() => {
    const regions = viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? [];
    const presets = [
      { id: 'center', label: 'Centerline', description: 'Reset the section plane to the ring centerline.' },
    ];
    for (const regionId of ['inner_band', 'head', 'ornament_relief', 'outer_band'] as const) {
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
    scoopMutation.isPending ||
    smoothMutation.isPending ||
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
    if (compareOverlayEnabled) values.push('Compare');
    return values;
  }, [compareOverlayEnabled, heatmapEnabled, regionOverlayEnabled, sectionEnabled, wireframe]);

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
    if (!sectionContour?.segments.length) {
      return;
    }
    const { u, v } = createPlaneBasis(sectionAxis);
    const projectedSegments = sectionContour.segments.map((segment) => ({
      x1: dot(segment.start, u),
      y1: dot(segment.start, v),
      x2: dot(segment.end, u),
      y2: dot(segment.end, v),
      selectedRegionHit: segment.selected_region_hit,
    }));
    const allPoints = projectedSegments.flatMap((segment) => [
      { x: segment.x1, y: segment.y1 },
      { x: segment.x2, y: segment.y2 },
    ]);
    const minX = Math.min(...allPoints.map((point) => point.x));
    const maxX = Math.max(...allPoints.map((point) => point.x));
    const minY = Math.min(...allPoints.map((point) => point.y));
    const maxY = Math.max(...allPoints.map((point) => point.y));
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
    const svg = [
      `<svg xmlns="http://www.w3.org/2000/svg" width="${svgWidth.toFixed(2)}mm" height="${svgHeight.toFixed(2)}mm" viewBox="0 0 ${svgWidth.toFixed(3)} ${svgHeight.toFixed(3)}">`,
      '<rect width="100%" height="100%" fill="#09090b" />',
      ...lines,
      `<line x1="${margin}" y1="${svgHeight - margin}" x2="${svgWidth - margin}" y2="${svgHeight - margin}" stroke="#22c55e" stroke-width="0.5" />`,
      `<line x1="${svgWidth - margin}" y1="${margin}" x2="${svgWidth - margin}" y2="${svgHeight - margin}" stroke="#38bdf8" stroke-width="0.5" />`,
      `<text x="${margin}" y="${margin - 4}" fill="#f8fafc" font-size="4">Offset=${sectionContour.section_constant.toFixed(2)}mm</text>`,
      `<text x="${margin}" y="${svgHeight - 2}" fill="#22c55e" font-size="4">W=${(sectionContour.width_mm ?? 0).toFixed(2)}mm</text>`,
      `<text x="${svgWidth - margin + 2}" y="${margin + 6}" fill="#38bdf8" font-size="4">D=${(sectionContour.depth_mm ?? 0).toFixed(2)}mm</text>`,
      '</svg>',
    ].join('');
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `${versionId ?? 'section'}-offset${sectionContour.section_constant.toFixed(1)}.svg`;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    URL.revokeObjectURL(url);
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

  const onSaveInspection = (name: string) => {
    if (!versionId) return;
    createInspectionSnapshotMutation.mutate({
      versionId,
      params: {
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
      },
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
          disabled: !sectionContour?.segments.length,
          reason: sectionContour?.segments.length ? undefined : 'Enable a section with contour data before exporting.',
        };
      case 'thicken-region':
        return { disabled: !selectedRegion, reason: selectedRegion ? undefined : 'Select a primary region first.' };
      case 'batch-thicken':
      case 'batch-smooth':
        return {
          disabled: selectedRegionIds.length < 2,
          reason: selectedRegionIds.length >= 2 ? undefined : 'Batch commands require at least 2 selected regions.',
        };
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

  const onCommandSelect = (commandId: WorkspaceCommandId) => {
    const definition = WORKSPACE_COMMANDS.find((command) => command.id === commandId);
    if (!definition) return;
    const availability = getCommandAvailability(commandId);
    if (availability.disabled) {
      return;
    }

    setActiveToolbarGroup(definition.group);

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
            sectionContour={sectionContour}
            sectionPresets={sectionPresets}
            savedSnapshots={inspectionSnapshotsQuery.data ?? []}
            onRepair={onRepair}
            onResize={onResize}
            onHollow={onHollow}
            onThicken={onThicken}
            onScoop={onScoop}
            onSmooth={onSmooth}
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
            onCompareToggle={() => setCompareOverlayEnabled(!compareOverlayEnabled)}
            onCompareTargetChange={onRequestCompare}
            compareSummary={compareOverlayQuery.data ?? null}
            cacheEntries={compareCacheQuery.data ?? []}
            onOpenVersion={onOpenVersion}
            onBranchVersion={onBranchVersion}
            onCompareVersion={onCompareVersion}
            busy={busy || branchVersionMutation.isPending}
          />
        );
      case 'activity':
        return <JobActivityPanel events={jobEvents.events} job={currentJob.data ?? jobEvents.terminalStatus} />;
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
    <main className="flex h-screen flex-col overflow-hidden bg-[radial-gradient(circle_at_top,_#1b1b1e,_#09090b_60%)] text-zinc-100">
      <header className="flex shrink-0 items-center justify-between border-b border-zinc-800 bg-zinc-950/80 px-6 py-4 backdrop-blur">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-zinc-500">MeshInspector Production</p>
          <h1 className="text-lg font-semibold text-white">Manufacturing Workspace</h1>
        </div>
        <div className="flex items-center gap-3">
          {currentJob.data && (
            <div className="rounded-full border border-amber-500/30 bg-amber-500/10 px-3 py-1 text-xs text-amber-200">
              {currentJob.data.operation_type} {currentJob.data.progress_pct}%
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
                normalizedMeshUrl={normalizedMeshUrl}
                regionArtifactUrl={regionArtifactUrl}
                regionOverlayEnabled={regionOverlayEnabled}
                selectedRegionId={selectedRegionId}
                selectedRegionIds={selectedRegionIds}
                scalarOverlay={scalarOverlay}
                onRegionPick={onRegionPick}
                onSectionContourChange={setSectionContour}
              />
              <ViewerMetricsHud snapshot={snapshotQuery.data ?? null} material={selectedMaterial} />
            </>
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
        job={currentJob.data ?? jobEvents.terminalStatus ?? null}
      />
    </main>
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

function regionsHaveScoopTarget(regions: RegionManifestEntry[], selectedRegionId: string | null) {
  const primary = regions.find((region) => region.region_id === selectedRegionId && region.allowed_operations?.includes('scoop'));
  if (primary) return true;
  return regions.some((region) => region.allowed_operations?.includes('scoop') && region.vertex_count > 0);
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
