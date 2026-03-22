'use client';

import { useState } from 'react';
import type {
  HollowRequestV2,
  MaterialType,
  RegionManifestEntry,
  ResizeRequestV2,
  ScoopRequestV2,
  SmoothRequestV2,
  ThickenRequestV2,
} from '@/lib/api/types';

export default function AdvancedEditPanel({
  wireframe,
  sectionEnabled,
  sectionConstant,
  heatmapEnabled,
  regionOverlayEnabled,
  selectedRegionId,
  selectedRegionIds,
  onWireframeToggle,
  onSectionToggle,
  onHeatmapToggle,
  onRegionOverlayToggle,
  onRegionSelect,
  onRegionToggle,
  onResize,
  onHollow,
  onThicken,
  onScoop,
  onSmooth,
  busy,
  regions,
  selectedMaterial,
}: {
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  heatmapEnabled: boolean;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  onWireframeToggle: () => void;
  onSectionToggle: () => void;
  onHeatmapToggle: () => void;
  onRegionOverlayToggle: () => void;
  onRegionSelect: (regionId: string) => void;
  onRegionToggle: (regionId: string) => void;
  onResize: (request: ResizeRequestV2) => void;
  onHollow: (request: HollowRequestV2) => void;
  onThicken: (request: ThickenRequestV2) => void;
  onScoop: (request: ScoopRequestV2) => void;
  onSmooth: (request: SmoothRequestV2) => void;
  busy: boolean;
  regions: RegionManifestEntry[];
  selectedMaterial: MaterialType;
}) {
  const [resizeTargetSize, setResizeTargetSize] = useState(8);
  const [wallThickness, setWallThickness] = useState(0.8);
  const [thickenTarget, setThickenTarget] = useState(0.8);
  const [scoopDepth, setScoopDepth] = useState(0.35);
  const [scoopFalloff, setScoopFalloff] = useState(1.5);
  const [smoothIterations, setSmoothIterations] = useState(6);
  const [smoothStrength, setSmoothStrength] = useState(0.35);
  const selectedRegion = regions.find((region) => region.region_id === selectedRegionId) ?? null;
  const selectedRegions = regions.filter((region) => selectedRegionIds.includes(region.region_id));
  const scoopRegion = selectedRegion?.allowed_operations.includes('scoop')
    ? selectedRegion
    : regions.find((region) => region.allowed_operations.includes('scoop') && region.vertex_count > 0);

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Advanced Edit</p>
        <div className="mt-4 grid grid-cols-2 gap-2">
          <Toggle label="Wireframe" enabled={wireframe} onClick={onWireframeToggle} />
          <Toggle label="Section" enabled={sectionEnabled} onClick={onSectionToggle} />
          <Toggle label="Heatmap" enabled={heatmapEnabled} onClick={onHeatmapToggle} />
          <Toggle label="Regions" enabled={regionOverlayEnabled} onClick={onRegionOverlayToggle} />
        </div>
        {sectionEnabled && (
          <p className="mt-3 text-xs text-zinc-500">Section plane offset: {sectionConstant.toFixed(1)} mm</p>
        )}
        <div className="mt-4 space-y-2">
          {regions.map((region) => {
            const checked = selectedRegionIds.includes(region.region_id);
            const primary = selectedRegionId === region.region_id;
            return (
              <button
                key={region.region_id}
                onClick={() => onRegionSelect(region.region_id)}
                className={`w-full rounded-xl border px-3 py-2 text-left ${
                  checked ? 'border-amber-500/30 bg-amber-500/10' : 'border-zinc-800 bg-zinc-950'
                }`}
              >
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm text-zinc-200">{region.label}</p>
                    {primary ? <p className="mt-1 text-[10px] uppercase tracking-[0.18em] text-amber-300">Primary</p> : null}
                  </div>
                  <div className="flex items-center gap-3">
                    <label
                      onClick={(event) => event.stopPropagation()}
                      className="inline-flex items-center gap-2 text-xs text-zinc-500"
                    >
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={() => onRegionToggle(region.region_id)}
                        className="accent-amber-400"
                      />
                      Batch
                    </label>
                    <span className="text-xs text-zinc-500">{region.coverage_pct}%</span>
                  </div>
                </div>
                <p className="mt-1 text-xs text-zinc-500">
                  min {region.min_thickness_mm ?? 'n/a'} mm
                  {region.allowed_operations.length ? ` | ${region.allowed_operations.join(', ')}` : ''}
                </p>
              </button>
            );
          })}
        </div>
        <div className="mt-4 grid grid-cols-2 gap-3">
          <Field label="Resize To" value={resizeTargetSize} min={3} max={15} step={0.5} onChange={setResizeTargetSize} />
          <Field label="Wall mm" value={wallThickness} min={0.3} max={5} step={0.05} onChange={setWallThickness} />
          <Field label="Thicken To" value={thickenTarget} min={0.3} max={5} step={0.05} onChange={setThickenTarget} />
          <Field label="Scoop mm" value={scoopDepth} min={0.05} max={5} step={0.05} onChange={setScoopDepth} />
          <Field label="Falloff mm" value={scoopFalloff} min={0.1} max={10} step={0.1} onChange={setScoopFalloff} />
          <Field label="Smooth Iter" value={smoothIterations} min={1} max={50} step={1} onChange={setSmoothIterations} />
        </div>
        <div className="mt-3">
          <Field label="Smooth Strength" value={smoothStrength} min={0.01} max={1} step={0.01} onChange={setSmoothStrength} />
        </div>
      </div>

      <ActionButton
        title={`Resize To ${resizeTargetSize}`}
        description="Create a resized version while preserving ornament-heavy regions."
        onClick={() => onResize({ target_ring_size_us: resizeTargetSize, axis_mode: 'auto', preserve_head: true })}
        busy={busy}
      />

      <ActionButton
        title={`Protected Hollow ${wallThickness.toFixed(2)} mm`}
        description="Build a weighted hollow shell that keeps decorative head and relief regions thicker than the inner band."
        onClick={() =>
          onHollow({
            mode: 'fixed_thickness',
            material: selectedMaterial,
            wall_thickness_mm: wallThickness,
            min_allowed_thickness_mm: 0.6,
            protect_regions: ['head', 'gem_seat', 'ornament_relief'],
            add_drain_holes: false,
          })
        }
        busy={busy}
      />

      <ActionButton
        title={`Protected Hollow + Drains (${wallThickness.toFixed(2)} mm)`}
        description="Build the protected shell and add two conservative drain holes through opposite sides of the inner band."
        onClick={() =>
          onHollow({
            mode: 'fixed_thickness',
            material: selectedMaterial,
            wall_thickness_mm: wallThickness,
            min_allowed_thickness_mm: 0.6,
            protect_regions: ['head', 'gem_seat', 'ornament_relief'],
            add_drain_holes: true,
          })
        }
        busy={busy}
      />

      <ActionButton
        title="Thicken Violations"
        description="Create a new version that thickens unsafe regions."
        onClick={() =>
          onThicken({
            mode: 'violations_only',
            min_target_thickness_mm: thickenTarget,
            smoothing_pass: true,
          })
        }
        busy={busy}
      />

      {selectedRegion && (
        <ActionButton
          title={`Thicken ${selectedRegion.label}`}
          description="Apply a localized outward thickening pass around the primary region."
          onClick={() =>
            onThicken({
              mode: 'selected_region',
              region_id: selectedRegion.region_id,
              min_target_thickness_mm: thickenTarget,
              smoothing_pass: true,
            })
          }
          busy={busy}
        />
      )}

      {selectedRegions.length > 1 && (
        <ActionButton
          title={`Batch Thicken ${selectedRegions.length} Regions`}
          description="Apply one localized thickening pass across all batch-selected regions."
          onClick={() =>
            onThicken({
              mode: 'selected_regions',
              region_ids: selectedRegions.map((region) => region.region_id),
              min_target_thickness_mm: thickenTarget,
              smoothing_pass: true,
            })
          }
          busy={busy}
        />
      )}

      {scoopRegion && (
        <ActionButton
          title={`Scoop ${scoopRegion.label}`}
          description="Carve a controlled recess into a scoop-safe region while enforcing minimum thickness."
          onClick={() =>
            onScoop({
              region_id: scoopRegion.region_id,
              depth_mm: scoopDepth,
              falloff_mm: scoopFalloff,
              keep_min_thickness_mm: 0.6,
            })
          }
          busy={busy}
        />
      )}

      <ActionButton
        title={selectedRegion ? `Smooth ${selectedRegion.label}` : 'Smooth Surface'}
        description={
          selectedRegion
            ? `Smooth the primary ${selectedRegion.label.toLowerCase()} region with a localized falloff.`
            : 'Apply a conservative global smoothing pass to soften AI-generated chatter.'
        }
        onClick={() =>
          onSmooth({
            region_id: selectedRegion?.region_id,
            iterations: smoothIterations,
            strength: smoothStrength,
            global_mode: !selectedRegion,
          })
        }
        busy={busy}
      />

      {selectedRegions.length > 1 && (
        <ActionButton
          title={`Batch Smooth ${selectedRegions.length} Regions`}
          description="Smooth all batch-selected regions in one localized pass while preserving the rest of the ring."
          onClick={() =>
            onSmooth({
              region_ids: selectedRegions.map((region) => region.region_id),
              iterations: smoothIterations,
              strength: smoothStrength,
              global_mode: false,
            })
          }
          busy={busy}
        />
      )}
    </div>
  );
}

function Field({
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
    <label className="text-[11px] uppercase tracking-[0.18em] text-zinc-500">
      {label}
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(Number(event.target.value))}
        className="mt-2 w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100"
      />
    </label>
  );
}

function Toggle({
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
      className={`rounded-xl border px-3 py-2 text-sm ${enabled ? 'border-blue-500/40 bg-blue-500/15 text-blue-200' : 'border-zinc-800 bg-zinc-950 text-zinc-300'}`}
    >
      {label}
    </button>
  );
}

function ActionButton({
  title,
  description,
  onClick,
  busy,
}: {
  title: string;
  description: string;
  onClick: () => void;
  busy: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={busy}
      className={`w-full rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4 text-left hover:bg-zinc-900 ${busy ? 'cursor-wait opacity-60' : ''}`}
    >
      <h3 className="text-sm font-semibold text-white">{title}</h3>
      <p className="mt-2 text-sm text-zinc-400">{description}</p>
    </button>
  );
}
