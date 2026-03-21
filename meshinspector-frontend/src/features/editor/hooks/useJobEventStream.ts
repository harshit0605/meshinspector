'use client';

import { useEffect, useState } from 'react';
import { streamJobEvents } from '@/lib/api/models';
import type { JobEventResponse, JobResponse } from '@/lib/api/types';

export function useJobEventStream(jobId: string | null) {
  const [events, setEvents] = useState<JobEventResponse[]>([]);
  const [terminalStatus, setTerminalStatus] = useState<JobResponse | null>(null);

  useEffect(() => {
    setEvents([]);
    setTerminalStatus(null);
    if (!jobId) {
      return;
    }

    let closed = false;
    let cleanup: (() => void) | undefined;

    void streamJobEvents(jobId, {
      onEvent: (event) => {
        if (closed) return;
        setEvents((current) => [...current.slice(-11), event]);
      },
      onStatus: (status) => {
        if (closed) return;
        setTerminalStatus(status);
      },
    }).then((close) => {
      cleanup = close;
    });

    return () => {
      closed = true;
      cleanup?.();
    };
  }, [jobId]);

  return { events, terminalStatus };
}
