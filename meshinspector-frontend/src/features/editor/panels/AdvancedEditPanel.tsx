'use client';

import type {
  HollowRequestV2,
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
}) {
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
      </div>

      <ActionButton
        title="Resize To 8"
        description="Create a resized version while preserving ornament-heavy regions."
        onClick={() => onResize({ target_ring_size_us: 8, axis_mode: 'auto', preserve_head: true })}
        busy={busy}
      />

      <ActionButton
        title="Protected Hollow 0.8 mm"
        description="Build a weighted hollow shell that keeps decorative head and relief regions thicker than the inner band."
        onClick={() =>
          onHollow({
            mode: 'fixed_thickness',
            material: 'gold_18k',
            wall_thickness_mm: 0.8,
            min_allowed_thickness_mm: 0.6,
            protect_regions: ['head', 'gem_seat', 'ornament_relief'],
            add_drain_holes: false,
          })
        }
        busy={busy}
      />

      <ActionButton
        title="Protected Hollow + Drains"
        description="Build the protected shell and add two conservative drain holes through opposite sides of the inner band."
        onClick={() =>
          onHollow({
            mode: 'fixed_thickness',
            material: 'gold_18k',
            wall_thickness_mm: 0.8,
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
            min_target_thickness_mm: 0.8,
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
              min_target_thickness_mm: 0.8,
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
              min_target_thickness_mm: 0.8,
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
              depth_mm: 0.35,
              falloff_mm: 1.5,
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
            iterations: 6,
            strength: 0.35,
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
              iterations: 6,
              strength: 0.35,
              global_mode: false,
            })
          }
          busy={busy}
        />
      )}
    </div>
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
