'use client';

import ComparePanel from '@/features/editor/panels/ComparePanel';
import VersionHistoryPanel from '@/features/editor/panels/VersionHistoryPanel';
import type { CompareCacheEntry, ScalarOverlayResponse, VersionSummary } from '@/lib/api/types';
import type { ReviewPane } from './types';

export default function ReviewInspector({
  reviewPane,
  onReviewPaneChange,
  versions,
  currentVersionId,
  compareTargetVersionId,
  compareEnabled,
  onCompareToggle,
  onCompareTargetChange,
  compareSummary,
  cacheEntries,
  onOpenVersion,
  onBranchVersion,
  onCompareVersion,
  busy,
}: {
  reviewPane: ReviewPane;
  onReviewPaneChange: (value: ReviewPane) => void;
  versions: VersionSummary[];
  currentVersionId: string;
  compareTargetVersionId: string | null;
  compareEnabled: boolean;
  onCompareToggle: () => void;
  onCompareTargetChange: (value: string | null) => void;
  compareSummary: ScalarOverlayResponse | null;
  cacheEntries: CompareCacheEntry[];
  onOpenVersion: (versionId: string) => void;
  onBranchVersion: (versionId: string) => void;
  onCompareVersion: (versionId: string) => void;
  busy: boolean;
}) {
  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-2">
        <div className="grid grid-cols-2 gap-2">
          <ReviewTabButton active={reviewPane === 'compare'} label="Compare" onClick={() => onReviewPaneChange('compare')} />
          <ReviewTabButton active={reviewPane === 'history'} label="Version History" onClick={() => onReviewPaneChange('history')} />
        </div>
      </div>

      {reviewPane === 'compare' ? (
        <ComparePanel
          versions={versions}
          currentVersionId={currentVersionId}
          compareTargetVersionId={compareTargetVersionId}
          compareEnabled={compareEnabled}
          onCompareToggle={onCompareToggle}
          onCompareTargetChange={onCompareTargetChange}
          summary={compareSummary}
          cacheEntries={cacheEntries}
        />
      ) : (
        <VersionHistoryPanel
          versions={versions}
          currentVersionId={currentVersionId}
          onOpenVersion={onOpenVersion}
          onBranchVersion={onBranchVersion}
          onCompareVersion={onCompareVersion}
          busy={busy}
        />
      )}
    </div>
  );
}

function ReviewTabButton({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-xl px-3 py-2 text-sm transition-colors ${
        active ? 'bg-zinc-100 text-zinc-950' : 'bg-zinc-950 text-zinc-300 hover:bg-zinc-900'
      }`}
    >
      {label}
    </button>
  );
}
