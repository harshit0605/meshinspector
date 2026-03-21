'use client';

import { Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import dynamic from 'next/dynamic';
import ManufacturabilityPanel from '@/features/editor/panels/ManufacturabilityPanel';
import GuidedWorkflowPanel from '@/features/editor/panels/GuidedWorkflowPanel';
import AdvancedEditPanel from '@/features/editor/panels/AdvancedEditPanel';
import ComparePanel from '@/features/editor/panels/ComparePanel';
import JobActivityPanel from '@/features/editor/panels/JobActivityPanel';
import OverlayLegendPanel from '@/features/editor/panels/OverlayLegendPanel';
import VersionHistoryPanel from '@/features/editor/panels/VersionHistoryPanel';
import { useEditorStore } from '@/features/editor/store';
import { useJobEventStream } from '@/features/editor/hooks/useJobEventStream';
import { useJobPolling } from '@/features/editor/hooks/useJobPolling';
import {
  useCreateInspectionSnapshot,
  useBranchVersion,
  useCompareCache,
  useCompareOverlay,
  useDownloadModel,
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
  useViewerManifest,
} from '@/hooks/useModelProcessing';
import type {
  HollowRequestV2,
  InspectionSnapshotResponse,
  MakeManufacturableRequest,
  ResizeRequestV2,
  ScoopRequestV2,
  SectionContourPayload,
  SmoothRequestV2,
  ThickenRequestV2,
} from '@/lib/api/types';

const ViewerEngine = dynamic(() => import('@/features/editor/viewer/ViewerEngine'), { ssr: false });

function ViewerPageContent() {
  const searchParams = useSearchParams();
  const router = useRouter();
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

  const versionsQuery = useModelVersions(modelId);
  const compareCacheQuery = useCompareCache(versionId);
  const inspectionSnapshotsQuery = useInspectionSnapshots(versionId);
  const viewerQuery = useViewerManifest(versionId);
  const snapshotQuery = useManufacturability(versionId);
  const thicknessOverlayQuery = useThicknessOverlay(versionId, heatmapEnabled && !compareOverlayEnabled);
  const compareOverlayQuery = useCompareOverlay(versionId, compareTargetVersionId, compareOverlayEnabled);
  const repairMutation = useRepairOperation();
  const resizeMutation = useResizeOperation();
  const hollowMutation = useHollowOperation();
  const thickenMutation = useThickenOperation();
  const scoopMutation = useScoopOperation();
  const smoothMutation = useSmoothOperation();
  const makeMutation = useMakeManufacturableOperation();
  const createInspectionSnapshotMutation = useCreateInspectionSnapshot();
  const branchVersionMutation = useBranchVersion();
  const downloadMutation = useDownloadModel();

  const currentJob = useJobPolling(activeJobId, useCallback((nextVersionId) => {
    setVersionId(nextVersionId);
    setActiveJobId(null);
  }, []));
  const jobEvents = useJobEventStream(activeJobId);

  useEffect(() => {
    if (urlVersionId && urlVersionId !== versionId) {
      setVersionId(urlVersionId);
    }
  }, [urlVersionId, versionId]);

  useEffect(() => {
    if (urlJobId !== activeJobId) {
      setActiveJobId(urlJobId);
    }
  }, [activeJobId, urlJobId]);

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

  const busy = repairMutation.isPending || resizeMutation.isPending || hollowMutation.isPending || thickenMutation.isPending || scoopMutation.isPending || smoothMutation.isPending || makeMutation.isPending || (currentJob.data?.status === 'running') || false;

  const submitAndTrack = async (promise: Promise<unknown>) => {
    const job = await promise as { id: string };
    setActiveJobId(job.id);
  };

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
      setSectionConstant(selectedRegion.centroid_mm[1]);
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
    const allPoints = sectionContour.segments.flatMap((segment) => [
      { x: segment.start[0], z: segment.start[2] },
      { x: segment.end[0], z: segment.end[2] },
    ]);
    const minX = Math.min(...allPoints.map((point) => point.x));
    const maxX = Math.max(...allPoints.map((point) => point.x));
    const minZ = Math.min(...allPoints.map((point) => point.z));
    const maxZ = Math.max(...allPoints.map((point) => point.z));
    const width = Math.max(maxX - minX, 1);
    const depth = Math.max(maxZ - minZ, 1);
    const margin = 12;
    const svgWidth = width + margin * 2;
    const svgHeight = depth + margin * 2;
    const lines = sectionContour.segments.map((segment) => {
      const x1 = segment.start[0] - minX + margin;
      const y1 = maxZ - segment.start[2] + margin;
      const x2 = segment.end[0] - minX + margin;
      const y2 = maxZ - segment.end[2] + margin;
      const stroke = segment.selected_region_hit ? '#f59e0b' : '#f8fafc';
      return `<line x1="${x1.toFixed(3)}" y1="${y1.toFixed(3)}" x2="${x2.toFixed(3)}" y2="${y2.toFixed(3)}" stroke="${stroke}" stroke-width="0.75" />`;
    });
    const svg = [
      `<svg xmlns="http://www.w3.org/2000/svg" width="${svgWidth.toFixed(2)}mm" height="${svgHeight.toFixed(2)}mm" viewBox="0 0 ${svgWidth.toFixed(3)} ${svgHeight.toFixed(3)}">`,
      '<rect width="100%" height="100%" fill="#09090b" />',
      ...lines,
      `<line x1="${margin}" y1="${svgHeight - margin}" x2="${svgWidth - margin}" y2="${svgHeight - margin}" stroke="#22c55e" stroke-width="0.5" />`,
      `<line x1="${svgWidth - margin}" y1="${margin}" x2="${svgWidth - margin}" y2="${svgHeight - margin}" stroke="#38bdf8" stroke-width="0.5" />`,
      `<text x="${margin}" y="${margin - 4}" fill="#f8fafc" font-size="4">Y=${sectionContour.section_constant.toFixed(2)}mm</text>`,
      `<text x="${margin}" y="${svgHeight - 2}" fill="#22c55e" font-size="4">W=${(sectionContour.width_mm ?? 0).toFixed(2)}mm</text>`,
      `<text x="${svgWidth - margin + 2}" y="${margin + 6}" fill="#38bdf8" font-size="4">D=${(sectionContour.depth_mm ?? 0).toFixed(2)}mm</text>`,
      '</svg>',
    ].join('');
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `${versionId ?? 'section'}-y${sectionContour.section_constant.toFixed(1)}.svg`;
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
    setSectionConstant(region.centroid_mm[1]);
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

  const onOpenVersion = (nextVersionId: string) => {
    setVersionId(nextVersionId);
    setActiveJobId(null);
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
    setCompareTargetVersionId(otherVersionId);
    setCompareOverlayEnabled(true);
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
    <main className="min-h-screen bg-[radial-gradient(circle_at_top,_#1b1b1e,_#09090b_60%)] text-zinc-100">
      <header className="flex items-center justify-between px-6 py-4 border-b border-zinc-800 bg-zinc-950/80 backdrop-blur">
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
          <button
            onClick={() => router.push('/')}
            className="rounded-xl border border-zinc-800 px-3 py-2 text-sm text-zinc-300 hover:bg-zinc-900"
          >
            Upload New
          </button>
          <button
            onClick={() => downloadMutation.mutate({ modelId, format: 'stl' })}
            className="rounded-xl bg-amber-500 px-3 py-2 text-sm font-medium text-black"
          >
            Download STL
          </button>
        </div>
      </header>

      <div className="grid min-h-[calc(100vh-73px)] grid-cols-[360px_minmax(0,1fr)_340px] gap-0">
        <aside className="border-r border-zinc-800 bg-zinc-950/70 p-4 overflow-y-auto">
          <ManufacturabilityPanel
            snapshot={snapshotQuery.data ?? null}
            selectedMaterial={selectedMaterial}
            axisMode={resizeAxisMode}
            manualAxis={manualResizeAxis}
            onAxisModeChange={setResizeAxisMode}
            onManualAxisChange={setManualResizeAxis}
          />
        </aside>

        <section className="relative min-h-[720px]">
          {previewLowUrl ? (
            <ViewerEngine
              lowUrl={previewLowUrl}
              highUrl={previewHighUrl}
              wireframe={wireframe}
              sectionEnabled={sectionEnabled}
              sectionConstant={sectionConstant}
              normalizedMeshUrl={normalizedMeshUrl}
              regionArtifactUrl={regionArtifactUrl}
              regionOverlayEnabled={regionOverlayEnabled}
              selectedRegionId={selectedRegionId}
              selectedRegionIds={selectedRegionIds}
              scalarOverlay={scalarOverlay}
              onRegionPick={onRegionPick}
              onSectionContourChange={setSectionContour}
            />
          ) : (
            <div className="h-full flex items-center justify-center text-zinc-500">Preparing viewer artifacts...</div>
          )}
        </section>

        <aside className="border-l border-zinc-800 bg-zinc-950/70 p-4 overflow-y-auto space-y-4">
          <GuidedWorkflowPanel
            material={selectedMaterial}
            onMaterialChange={setSelectedMaterial}
            onRepair={onRepair}
            onResize={onResize}
            onHollow={onHollow}
            onMakeManufacturable={onMakeManufacturable}
            busy={busy}
          />
          <AdvancedEditPanel
            wireframe={wireframe}
            sectionEnabled={sectionEnabled}
            sectionConstant={sectionConstant}
            heatmapEnabled={heatmapEnabled}
            regionOverlayEnabled={regionOverlayEnabled}
            selectedRegionId={selectedRegionId}
            selectedRegionIds={selectedRegionIds}
            onWireframeToggle={() => setWireframe(!wireframe)}
            onSectionToggle={() => setSectionEnabled(!sectionEnabled)}
            onHeatmapToggle={() => setHeatmapEnabled(!heatmapEnabled)}
            onRegionOverlayToggle={() => setRegionOverlayEnabled(!regionOverlayEnabled)}
            onRegionSelect={setSelectedRegionId}
            onRegionToggle={toggleSelectedRegionId}
            onResize={onResize}
            onHollow={onHollow}
            onThicken={onThicken}
            onScoop={onScoop}
            onSmooth={onSmooth}
            busy={busy}
            regions={viewerQuery.data?.region_manifest ?? snapshotQuery.data?.regions ?? []}
          />
          <OverlayLegendPanel
            overlay={scalarOverlay}
            sectionEnabled={sectionEnabled}
            sectionConstant={sectionConstant}
            selectedRegion={selectedRegion}
            sectionContour={sectionContour}
            sectionPresets={sectionPresets}
            savedSnapshots={inspectionSnapshotsQuery.data ?? []}
            onSectionConstantChange={setSectionConstant}
            onSnapToRegion={onSnapToRegion}
            onSnapToCenter={onSnapToCenter}
            onApplySectionPreset={onApplySectionPreset}
            onExportSection={onExportSection}
            onSaveSnapshot={onSaveInspection}
            onLoadSnapshot={onLoadInspection}
          />
          <JobActivityPanel events={jobEvents.events} job={currentJob.data ?? jobEvents.terminalStatus} />
          <ComparePanel
            versions={versionsQuery.data ?? []}
            currentVersionId={versionId}
            compareTargetVersionId={compareTargetVersionId}
            compareEnabled={compareOverlayEnabled}
            onCompareToggle={() => setCompareOverlayEnabled(!compareOverlayEnabled)}
            onCompareTargetChange={setCompareTargetVersionId}
            summary={compareOverlayQuery.data ?? null}
            cacheEntries={compareCacheQuery.data ?? []}
          />
          <VersionHistoryPanel
            versions={versionsQuery.data ?? []}
            currentVersionId={versionId}
            onOpenVersion={onOpenVersion}
            onBranchVersion={onBranchVersion}
            onCompareVersion={onCompareVersion}
            busy={busy || branchVersionMutation.isPending}
          />
        </aside>
      </div>
    </main>
  );
}

export default function ViewerPage() {
  return (
    <Suspense fallback={<div className="min-h-screen bg-zinc-950" />}>
      <ViewerPageContent />
    </Suspense>
  );
}
