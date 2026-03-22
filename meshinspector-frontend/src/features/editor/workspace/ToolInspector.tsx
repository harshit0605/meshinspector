'use client';

import type { ReactNode } from 'react';
import type {
  HollowRequestV2,
  InspectionSnapshotResponse,
  MakeManufacturableRequest,
  MaterialType,
  RegionManifestEntry,
  ResizeRequestV2,
  ScalarOverlayResponse,
  ScoopRequestV2,
  SectionContourPayload,
  SmoothRequestV2,
  ThickenRequestV2,
} from '@/lib/api/types';
import { MATERIALS } from '@/lib/constants';
import type { ContextToolId, ToolDrafts } from './types';

type SectionPreset = {
  id: string;
  label: string;
  description: string;
};

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
  sectionPresets,
  savedSnapshots,
  onRepair,
  onResize,
  onHollow,
  onThicken,
  onScoop,
  onSmooth,
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
  sectionPresets: SectionPreset[];
  savedSnapshots: InspectionSnapshotResponse[];
  onRepair: () => void;
  onResize: (request: ResizeRequestV2) => void;
  onHollow: (request: HollowRequestV2) => void;
  onThicken: (request: ThickenRequestV2) => void;
  onScoop: (request: ScoopRequestV2) => void;
  onSmooth: (request: SmoothRequestV2) => void;
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
          onMaterialChange={onMaterialChange}
          onDraftChange={updateDrafts}
          onApply={() =>
            onHollow({
              mode: 'fixed_thickness',
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
          onMaterialChange={onMaterialChange}
          onDraftChange={updateDrafts}
          onApply={() =>
            onHollow({
              mode: 'fixed_thickness',
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
      const reason = selectedRegion ? null : 'Select a primary region first.';
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
      const reason = batchRegions.length > 1 ? null : 'Batch-thicken requires at least 2 batch-selected regions.';
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
    case 'smooth':
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
            disabled={false}
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
    case 'batch-smooth': {
      const reason = batchRegions.length > 1 ? null : 'Batch smooth requires at least 2 batch-selected regions.';
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
              disabled={!sectionEnabled || !sectionContour?.segments.length}
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
  onMaterialChange: (value: MaterialType) => void;
  onDraftChange: (value: Partial<ToolDrafts>) => void;
  onApply: () => void;
  actionLabel: string;
}) {
  return (
    <ToolCard eyebrow="Modify" title={title} description="Weighted hollowing that preserves decorative head and relief regions.">
      <MaterialField material={selectedMaterial} onMaterialChange={onMaterialChange} />
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
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-300 hover:bg-zinc-900 disabled:opacity-40"
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

function Readout({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-zinc-800 px-2 py-2">
      <p className="text-[10px] uppercase tracking-[0.16em] text-zinc-500">{label}</p>
      <p className="mt-1 text-sm text-zinc-200">{value}</p>
    </div>
  );
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
