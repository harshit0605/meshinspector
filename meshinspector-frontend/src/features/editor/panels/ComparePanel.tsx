'use client';

import type { CompareCacheEntry, ScalarOverlayResponse, VersionSummary } from '@/lib/api/types';

export default function ComparePanel({
  versions,
  currentVersionId,
  compareTargetVersionId,
  compareEnabled,
  onCompareToggle,
  onCompareTargetChange,
  summary,
  cacheEntries,
}: {
  versions: VersionSummary[];
  currentVersionId: string;
  compareTargetVersionId: string | null;
  compareEnabled: boolean;
  onCompareToggle: () => void;
  onCompareTargetChange: (value: string | null) => void;
  summary: ScalarOverlayResponse | null;
  cacheEntries: CompareCacheEntry[];
}) {
  const options = versions.filter((version) => version.id !== currentVersionId);
  const cachedTargets = new Set(cacheEntries.map((entry) => entry.other_version_id));

  return (
    <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Compare</p>
        <button
          onClick={onCompareToggle}
          className={`rounded-xl border px-3 py-2 text-sm ${compareEnabled ? 'border-blue-500/40 bg-blue-500/15 text-blue-200' : 'border-zinc-800 bg-zinc-950 text-zinc-300'}`}
        >
          {compareEnabled ? 'Overlay On' : 'Overlay Off'}
        </button>
      </div>

      <div className="mt-4">
        <label className="text-xs uppercase tracking-[0.2em] text-zinc-500">Compare Against</label>
        <select
          value={compareTargetVersionId ?? ''}
          onChange={(event) => onCompareTargetChange(event.target.value || null)}
          className="mt-2 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100"
        >
          <option value="">Select a version</option>
          {options.map((version) => (
            <option key={version.id} value={version.id}>
              {version.operation_label}{cachedTargets.has(version.id) ? ' (cached)' : ''}
            </option>
          ))}
        </select>
      </div>

      {summary && summary.summary ? (
        <div className="mt-4 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3 text-sm text-zinc-300">
          <p>Max abs distance: {String(summary.summary.max_abs_distance_mm ?? 'n/a')} mm</p>
          <p className="mt-1">Mean distance: {String(summary.summary.mean_distance_mm ?? 'n/a')} mm</p>
        </div>
      ) : (
        <p className="mt-4 text-sm text-zinc-500">Select another version to visualize signed-distance deltas.</p>
      )}

      {cacheEntries.length > 0 && (
        <div className="mt-4 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
          <p className="text-xs uppercase tracking-[0.18em] text-zinc-500">Cached Comparisons</p>
          <div className="mt-2 space-y-2">
            {cacheEntries.slice(0, 4).map((entry) => (
              <button
                key={entry.artifact_id}
                onClick={() => onCompareTargetChange(entry.other_version_id)}
                className="w-full rounded-lg border border-zinc-800 px-3 py-2 text-left text-sm text-zinc-300 hover:bg-zinc-900"
              >
                <div className="flex items-center justify-between gap-3">
                  <span>{entry.other_version_id}</span>
                  <span className="text-xs text-zinc-500">cached</span>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
