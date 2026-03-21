'use client';

import { useEffect } from 'react';
import { useJob } from '@/hooks/useModelProcessing';

export function useJobPolling(jobId: string | null, onCompleted: (versionId: string) => void) {
  const query = useJob(jobId);

  useEffect(() => {
    if (query.data?.status === 'succeeded') {
      onCompleted(query.data.version_id);
    }
  }, [onCompleted, query.data]);

  return query;
}
