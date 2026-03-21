'use client';

import { MATERIALS } from '@/lib/constants';
import type { HollowRequestV2, MakeManufacturableRequest, MaterialType, ResizeRequestV2 } from '@/lib/api/types';

export default function GuidedWorkflowPanel({
  material,
  onMaterialChange,
  onRepair,
  onResize,
  onHollow,
  onMakeManufacturable,
  busy,
}: {
  material: MaterialType;
  onMaterialChange: (value: MaterialType) => void;
  onRepair: () => void;
  onResize: (request: ResizeRequestV2) => void;
  onHollow: (request: HollowRequestV2) => void;
  onMakeManufacturable: (request: MakeManufacturableRequest) => void;
  busy: boolean;
}) {
  return (
    <div className="space-y-4">
      <div className="p-4 rounded-2xl border border-zinc-800 bg-zinc-900/70">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Guided Manufacturing</p>
        <div className="mt-4">
          <label className="text-xs text-zinc-500 uppercase tracking-[0.2em]">Material</label>
          <select
            value={material}
            onChange={(event) => onMaterialChange(event.target.value as MaterialType)}
            className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100"
          >
            {Object.entries(MATERIALS).map(([value, item]) => (
              <option key={value} value={value}>{item.label}</option>
            ))}
          </select>
        </div>
      </div>

      <ActionCard
        title="Auto Repair"
        description="Heal holes, degeneracies, and manufacturability blockers before editing."
        onClick={onRepair}
        busy={busy}
      />

      <ActionCard
        title="Fit To Size"
        description="Create a new version resized to a production ring size."
        onClick={() => onResize({ target_ring_size_us: 7, axis_mode: 'auto', preserve_head: true })}
        busy={busy}
      />

      <ActionCard
        title="Reduce Weight"
        description="Create a protected-detail hollow shell that hollows safe interior zones first and keeps ornament-heavy regions thicker."
        onClick={() => onHollow({
          mode: 'target_weight',
          material,
          target_weight_g: 5,
          min_allowed_thickness_mm: 0.6,
          protect_regions: ['head', 'gem_seat', 'ornament_relief'],
          add_drain_holes: false,
        })}
        busy={busy}
      />

      <ActionCard
        title="Prepare For Casting"
        description="Create a protected hollow shell and add conservative drain holes through the inner band for castable hollow output."
        onClick={() => onHollow({
          mode: 'fixed_thickness',
          material,
          wall_thickness_mm: 0.8,
          min_allowed_thickness_mm: 0.6,
          protect_regions: ['head', 'gem_seat', 'ornament_relief'],
          add_drain_holes: true,
        })}
        busy={busy}
      />

      <ActionCard
        title="Make Manufacturable"
        description="Run the guided pipeline: repair, size, optimize, and validate."
        onClick={() => onMakeManufacturable({
          material,
          target_ring_size_us: 7,
          target_weight_g: 5,
          min_allowed_thickness_mm: 0.6,
        })}
        busy={busy}
        accent
      />
    </div>
  );
}

function ActionCard({
  title,
  description,
  onClick,
  busy,
  accent = false,
}: {
  title: string;
  description: string;
  onClick: () => void;
  busy: boolean;
  accent?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={busy}
      className={`w-full text-left p-4 rounded-2xl border transition-colors ${
        accent
          ? 'border-amber-500/30 bg-amber-500/10 hover:bg-amber-500/15'
          : 'border-zinc-800 bg-zinc-900/70 hover:bg-zinc-900'
      } ${busy ? 'opacity-60 cursor-wait' : ''}`}
    >
      <h3 className="text-sm font-semibold text-white">{title}</h3>
      <p className="mt-2 text-sm leading-6 text-zinc-400">{description}</p>
    </button>
  );
}
