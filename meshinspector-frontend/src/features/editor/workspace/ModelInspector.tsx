'use client';

import ManufacturabilityPanel from '@/features/editor/panels/ManufacturabilityPanel';
import { MATERIALS } from '@/lib/constants';
import type { ManufacturabilitySnapshot, MaterialType } from '@/lib/api/types';

export default function ModelInspector({
  snapshot,
  selectedMaterial,
  onMaterialChange,
  axisMode,
  manualAxis,
  onAxisModeChange,
  onManualAxisChange,
}: {
  snapshot: ManufacturabilitySnapshot | null;
  selectedMaterial: MaterialType;
  onMaterialChange: (value: MaterialType) => void;
  axisMode: 'auto' | 'manual';
  manualAxis: [number, number, number] | null;
  onAxisModeChange: (mode: 'auto' | 'manual') => void;
  onManualAxisChange: (axis: [number, number, number]) => void;
}) {
  return (
    <div className="space-y-4">
      <section className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
        <p className="text-[11px] uppercase tracking-[0.22em] text-zinc-500">Model</p>
        <label className="mt-4 block text-[11px] uppercase tracking-[0.18em] text-zinc-500">
          Active Material
          <select
            value={selectedMaterial}
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
        <p className="mt-3 text-sm leading-6 text-zinc-500">
          Material selection updates weight projections across the workspace and becomes the default for hollowing and
          manufacturability commands.
        </p>
      </section>

      <ManufacturabilityPanel
        snapshot={snapshot}
        selectedMaterial={selectedMaterial}
        axisMode={axisMode}
        manualAxis={manualAxis}
        onAxisModeChange={onAxisModeChange}
        onManualAxisChange={onManualAxisChange}
      />
    </div>
  );
}
