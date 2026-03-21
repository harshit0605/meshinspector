'use client';

import { useViewerManifest } from '@/hooks/useModelProcessing';

export function useVersionViewer(versionId: string | null) {
  return useViewerManifest(versionId);
}
