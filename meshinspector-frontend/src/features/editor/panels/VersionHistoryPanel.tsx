'use client';

import type { VersionSummary } from '@/lib/api/types';

export default function VersionHistoryPanel({
  versions,
  currentVersionId,
  onOpenVersion,
  onBranchVersion,
  onCompareVersion,
  busy,
}: {
  versions: VersionSummary[];
  currentVersionId: string;
  onOpenVersion: (versionId: string) => void;
  onBranchVersion: (versionId: string) => void;
  onCompareVersion: (versionId: string) => void;
  busy: boolean;
}) {
  return (
    <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Version History</p>
        <span className="text-xs text-zinc-500">{versions.length} versions</span>
      </div>

      <div className="mt-4 space-y-3">
        {versions.map((version) => {
          const active = version.id === currentVersionId;
          return (
            <div
              key={version.id}
              className={`rounded-xl border px-3 py-3 ${active ? 'border-amber-500/30 bg-amber-500/10' : 'border-zinc-800 bg-zinc-950'}`}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-sm font-medium text-zinc-100">{version.operation_label}</p>
                  <p className="mt-1 text-xs text-zinc-500">
                    {version.operation_type} | {new Date(version.created_at).toLocaleString()}
                  </p>
                  <p className="mt-1 text-xs text-zinc-500">
                    {version.id}
                    {version.parent_version_id ? ` | parent ${version.parent_version_id}` : ''}
                  </p>
                </div>
                <span className={`rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.18em] ${active ? 'bg-amber-500/20 text-amber-200' : 'bg-zinc-800 text-zinc-400'}`}>
                  {active ? 'Current' : version.status}
                </span>
              </div>

              <div className="mt-3 flex flex-wrap gap-2">
                <button
                  onClick={() => onOpenVersion(version.id)}
                  className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-xs text-zinc-200 hover:bg-zinc-800"
                >
                  Open
                </button>
                {!active ? (
                  <button
                    onClick={() => onCompareVersion(version.id)}
                    className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-xs text-zinc-200 hover:bg-zinc-800"
                  >
                    Compare To Current
                  </button>
                ) : null}
                <button
                  onClick={() => onBranchVersion(version.id)}
                  disabled={busy}
                  className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-100 hover:bg-amber-500/15 disabled:opacity-40"
                >
                  Restore As Branch
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
