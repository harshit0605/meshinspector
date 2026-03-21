'use client';

import type { ManufacturabilitySnapshot, MaterialType } from '@/lib/api/types';
import { MATERIALS } from '@/lib/constants';

export default function ManufacturabilityPanel({
  snapshot,
  selectedMaterial,
  axisMode,
  manualAxis,
  onAxisModeChange,
  onManualAxisChange,
}: {
  snapshot: ManufacturabilitySnapshot | null;
  selectedMaterial: MaterialType;
  axisMode: 'auto' | 'manual';
  manualAxis: [number, number, number] | null;
  onAxisModeChange: (mode: 'auto' | 'manual') => void;
  onManualAxisChange: (axis: [number, number, number]) => void;
}) {
  if (!snapshot) {
    return (
      <div className="p-4 rounded-2xl border border-zinc-800 bg-zinc-900/70">
        <p className="text-sm text-zinc-500">Manufacturability snapshot is not ready yet.</p>
      </div>
    );
  }

  const weight = snapshot.material_weight[selectedMaterial];
  const detectedAxis = snapshot.dimensions.ring_axis ?? null;
  const activeAxis = axisMode === 'manual' ? manualAxis : detectedAxis;

  return (
    <div className="space-y-4">
      <div className="p-4 rounded-2xl border border-zinc-800 bg-zinc-900/70">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Manufacturability</p>
            <h2 className="mt-2 text-xl font-semibold text-white">
              {snapshot.export_ready ? 'Export Ready' : 'Needs Work'}
            </h2>
          </div>
          <div className={`px-3 py-1 rounded-full text-xs font-medium ${snapshot.export_ready ? 'bg-green-500/20 text-green-300' : 'bg-amber-500/20 text-amber-300'}`}>
            Score {snapshot.mesh_health.health_score}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3 mt-4">
          <Stat label="Volume" value={`${weight.volume_mm3.toFixed(2)} mm3`} />
          <Stat label={MATERIALS[selectedMaterial].label} value={`${weight.weight_g.toFixed(2)} g`} />
          <Stat label="Ring Size" value={snapshot.dimensions.estimated_ring_size_us?.toFixed(1) ?? 'Unknown'} />
          <Stat label="Inner O" value={snapshot.dimensions.inner_diameter_mm ? `${snapshot.dimensions.inner_diameter_mm.toFixed(2)} mm` : 'Unknown'} />
          <Stat label="Min Thickness" value={snapshot.thickness.min_mm ? `${snapshot.thickness.min_mm.toFixed(2)} mm` : 'N/A'} tone={snapshot.thickness.min_mm && snapshot.thickness.min_mm < snapshot.thickness.threshold_mm ? 'danger' : 'default'} />
          <Stat label="Self Intersections" value={snapshot.mesh_health.self_intersections.toString()} tone={snapshot.mesh_health.self_intersections > 0 ? 'danger' : 'default'} />
        </div>
      </div>

      <div className={`p-4 rounded-2xl border ${snapshot.dimensions.needs_axis_confirmation ? 'border-amber-500/30 bg-amber-500/10' : 'border-zinc-800 bg-zinc-900/70'}`}>
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Ring Axis</p>
            <h3 className="mt-2 text-base font-semibold text-white">
              {snapshot.dimensions.needs_axis_confirmation ? 'Confirmation Recommended' : 'Axis Stable'}
            </h3>
          </div>
          <div className={`rounded-full px-3 py-1 text-xs ${snapshot.dimensions.needs_axis_confirmation ? 'bg-amber-500/20 text-amber-200' : 'bg-green-500/20 text-green-200'}`}>
            confidence {snapshot.dimensions.ring_axis_confidence.toFixed(2)}
          </div>
        </div>
        {detectedAxis ? (
          <p className="mt-3 text-sm text-zinc-400">
            Detected axis: {detectedAxis.map((value) => value.toFixed(2)).join(', ')}
          </p>
        ) : null}
        <p className="mt-1 text-sm text-zinc-500">
          Resizing uses this axis. Lock a manual axis if the detected one looks wrong for the ring opening.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          <AxisButton label="Auto" active={axisMode === 'auto'} onClick={() => onAxisModeChange('auto')} />
          <AxisButton
            label="Lock Detected"
            active={axisMode === 'manual' && !!manualAxis && !!detectedAxis && sameAxis(manualAxis, detectedAxis)}
            onClick={() => {
              if (detectedAxis) {
                onAxisModeChange('manual');
                onManualAxisChange(detectedAxis);
              }
            }}
            disabled={!detectedAxis}
          />
          <AxisButton label="X" active={axisMode === 'manual' && sameAxis(manualAxis, [1, 0, 0])} onClick={() => { onAxisModeChange('manual'); onManualAxisChange([1, 0, 0]); }} />
          <AxisButton label="Y" active={axisMode === 'manual' && sameAxis(manualAxis, [0, 1, 0])} onClick={() => { onAxisModeChange('manual'); onManualAxisChange([0, 1, 0]); }} />
          <AxisButton label="Z" active={axisMode === 'manual' && sameAxis(manualAxis, [0, 0, 1])} onClick={() => { onAxisModeChange('manual'); onManualAxisChange([0, 0, 1]); }} />
        </div>
        <p className="mt-3 text-xs text-zinc-500">
          Active axis: {activeAxis ? activeAxis.map((value) => value.toFixed(2)).join(', ') : 'auto detect'}
        </p>
      </div>

      <div className="p-4 rounded-2xl border border-zinc-800 bg-zinc-900/70">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Recommendations</p>
        <div className="mt-3 space-y-2">
          {snapshot.recommendations.map((recommendation) => (
            <div key={recommendation} className="rounded-xl bg-zinc-950/80 border border-zinc-800 px-3 py-2 text-sm text-zinc-300">
              {recommendation}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function sameAxis(a: [number, number, number] | null | undefined, b: [number, number, number] | null | undefined) {
  if (!a || !b) return false;
  return a.every((value, index) => Math.abs(value - b[index]) < 1e-4);
}

function AxisButton({
  label,
  active,
  onClick,
  disabled = false,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`rounded-lg border px-3 py-2 text-xs ${active ? 'border-amber-500/30 bg-amber-500/10 text-amber-100' : 'border-zinc-800 bg-zinc-950 text-zinc-300'} disabled:opacity-40`}
    >
      {label}
    </button>
  );
}

function Stat({
  label,
  value,
  tone = 'default',
}: {
  label: string;
  value: string;
  tone?: 'default' | 'danger';
}) {
  return (
    <div className={`rounded-xl border px-3 py-3 ${tone === 'danger' ? 'border-red-500/30 bg-red-500/10' : 'border-zinc-800 bg-zinc-950/80'}`}>
      <p className="text-[11px] uppercase tracking-[0.18em] text-zinc-500">{label}</p>
      <p className={`mt-2 text-sm font-medium ${tone === 'danger' ? 'text-red-300' : 'text-zinc-100'}`}>{value}</p>
    </div>
  );
}
