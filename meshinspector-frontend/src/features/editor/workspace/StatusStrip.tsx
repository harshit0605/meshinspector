'use client';

import type { JobResponse, MaterialType } from '@/lib/api/types';
import { MATERIALS } from '@/lib/constants';

export default function StatusStrip({
  currentVersionId,
  activeToolLabel,
  material,
  selectedRegionCount,
  overlays,
  job,
}: {
  currentVersionId: string;
  activeToolLabel: string | null;
  material: MaterialType;
  selectedRegionCount: number;
  overlays: string[];
  job: JobResponse | null;
}) {
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-x-5 gap-y-2 border-t border-zinc-800 bg-zinc-950/95 px-4 py-2 text-[11px] text-zinc-400">
      <StatusItem label="Version" value={currentVersionId} />
      <StatusItem label="Tool" value={activeToolLabel ?? 'None'} />
      <StatusItem label="Material" value={MATERIALS[material].label} />
      <StatusItem label="Regions" value={String(selectedRegionCount)} />
      <StatusItem label="Overlays" value={overlays.length ? overlays.join(' + ') : 'None'} />
      <StatusItem
        label="Job"
        value={job ? `${job.operation_type} ${job.status}${typeof job.progress_pct === 'number' ? ` ${job.progress_pct}%` : ''}` : 'Idle'}
      />
    </div>
  );
}

function StatusItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-2">
      <span className="uppercase tracking-[0.18em] text-zinc-600">{label}</span>
      <span className="text-zinc-300">{value}</span>
    </div>
  );
}
