'use client';

import type { JobEventResponse, JobResponse } from '@/lib/api/types';

export default function JobActivityPanel({
  events,
  job,
  jobHistory,
}: {
  events: JobEventResponse[];
  job: JobResponse | null;
  jobHistory: JobResponse[];
}) {
  return (
    <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
      <div className="flex items-center justify-between">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Activity</p>
        {job && (
          <span className="rounded-full border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300">
            {job.operation_type} {job.status}
          </span>
        )}
      </div>
      <div className="mt-3 space-y-2">
        {events.length === 0 ? (
          job?.status === 'failed' ? (
            <div className="rounded-xl border border-rose-900/60 bg-rose-950/40 px-3 py-3">
              <p className="text-sm text-rose-200">{job.error_message ?? job.error_code ?? 'Job failed.'}</p>
            </div>
          ) : (
            <p className="text-sm text-zinc-500">No live job events yet.</p>
          )
        ) : (
          events.map((event) => (
            <div key={event.id} className="rounded-xl border border-zinc-800 bg-zinc-950/80 px-3 py-2">
              <div className="flex items-center justify-between gap-3">
                <p className="text-sm text-zinc-200">{event.message}</p>
                {typeof event.progress_pct === 'number' && (
                  <span className="text-xs text-zinc-500">{event.progress_pct}%</span>
                )}
              </div>
            </div>
          ))
        )}
      </div>
      <div className="mt-5 border-t border-zinc-800 pt-4">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Recent Jobs</p>
        <div className="mt-3 space-y-2">
          {jobHistory.length === 0 ? (
            <p className="text-sm text-zinc-500">No version jobs recorded yet.</p>
          ) : (
            jobHistory.slice(0, 8).map((item) => (
              <div key={item.id} className="rounded-xl border border-zinc-800 bg-zinc-950/70 px-3 py-2">
                <div className="flex items-center justify-between gap-3">
                  <p className="text-sm text-zinc-200">{item.operation_type}</p>
                  <span className="text-xs text-zinc-500">{item.status}</span>
                </div>
                <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-zinc-800">
                  <div className="h-full bg-amber-400" style={{ width: `${Math.max(0, Math.min(100, item.progress_pct))}%` }} />
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
