'use client';

import { useEffect, useRef } from 'react';
import { useJob } from '@/hooks/useModelProcessing';

export function useJobPolling(jobId: string | null, onCompleted: (versionId: string) => void) {
  const query = useJob(jobId);
  const handledJobIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (jobId !== handledJobIdRef.current) {
      handledJobIdRef.current = null;
    }
  }, [jobId]);

  useEffect(() => {
    if (!jobId || query.data?.status !== 'succeeded') {
      return;
    }
    if (handledJobIdRef.current === jobId) {
      return;
    }
    handledJobIdRef.current = jobId;
    if (query.data.version_id) {
      onCompleted(query.data.version_id);
    }
  }, [jobId, onCompleted, query.data]);

  return query;
}
