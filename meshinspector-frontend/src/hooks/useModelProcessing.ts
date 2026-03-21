'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  branchVersion,
  createInspectionSnapshot,
  getCompareCache,
  getInspectionSnapshots,
  downloadModel,
  getCompareOverlay,
  getJob,
  getManufacturability,
  getModelVersions,
  getThicknessOverlay,
  getVersion,
  getViewerManifest,
  submitCompare,
  submitHollow,
  submitMakeManufacturable,
  submitRepair,
  submitResize,
  submitScoop,
  submitSmooth,
  submitThicken,
  uploadModel,
} from '@/lib/api/models';
import type {
  BranchVersionRequest,
  CompareRequestV2,
  CompareCacheEntry,
  HollowRequestV2,
  InspectionSnapshotResponse,
  InspectionSnapshotState,
  MakeManufacturableRequest,
  ResizeRequestV2,
  ScoopRequestV2,
  ScalarOverlayResponse,
  SmoothRequestV2,
  ThickenRequestV2,
} from '@/lib/api/types';

export function useUploadModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: uploadModel,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['models'] });
    },
  });
}

export function useVersion(versionId: string | null) {
  return useQuery({
    queryKey: ['version', versionId],
    queryFn: () => getVersion(versionId!),
    enabled: !!versionId,
  });
}

export function useManufacturability(versionId: string | null) {
  return useQuery({
    queryKey: ['manufacturability', versionId],
    queryFn: () => getManufacturability(versionId!),
    enabled: !!versionId,
    staleTime: 1000 * 30,
  });
}

export function useViewerManifest(versionId: string | null) {
  return useQuery({
    queryKey: ['viewer-manifest', versionId],
    queryFn: () => getViewerManifest(versionId!),
    enabled: !!versionId,
  });
}

export function useModelVersions(modelId: string | null) {
  return useQuery({
    queryKey: ['model-versions', modelId],
    queryFn: () => getModelVersions(modelId!),
    enabled: !!modelId,
  });
}

export function useThicknessOverlay(versionId: string | null, enabled: boolean) {
  return useQuery<ScalarOverlayResponse>({
    queryKey: ['thickness-overlay', versionId],
    queryFn: () => getThicknessOverlay(versionId!),
    enabled: !!versionId && enabled,
  });
}

export function useCompareOverlay(versionId: string | null, otherVersionId: string | null, enabled: boolean) {
  return useQuery<ScalarOverlayResponse>({
    queryKey: ['compare-overlay', versionId, otherVersionId],
    queryFn: () => getCompareOverlay(versionId!, otherVersionId!),
    enabled: !!versionId && !!otherVersionId && enabled,
  });
}

export function useCompareCache(versionId: string | null) {
  return useQuery<CompareCacheEntry[]>({
    queryKey: ['compare-cache', versionId],
    queryFn: () => getCompareCache(versionId!),
    enabled: !!versionId,
  });
}

export function useInspectionSnapshots(versionId: string | null) {
  return useQuery<InspectionSnapshotResponse[]>({
    queryKey: ['inspection-snapshots', versionId],
    queryFn: () => getInspectionSnapshots(versionId!),
    enabled: !!versionId,
  });
}

export function useJob(jobId: string | null) {
  return useQuery({
    queryKey: ['job', jobId],
    queryFn: () => getJob(jobId!),
    enabled: !!jobId,
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      return status && ['succeeded', 'failed'].includes(status) ? false : 2000;
    },
  });
}

function createOperationMutation<TArgs>(submitter: (versionId: string, params: TArgs) => Promise<unknown>) {
  return function useOperation() {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ versionId, params }: { versionId: string; params: TArgs }) => submitter(versionId, params),
      onSuccess: (_data, variables) => {
        queryClient.invalidateQueries({ queryKey: ['version', variables.versionId] });
      },
    });
  };
}

export const useResizeOperation = createOperationMutation<ResizeRequestV2>(submitResize);
export const useHollowOperation = createOperationMutation<HollowRequestV2>(submitHollow);
export const useThickenOperation = createOperationMutation<ThickenRequestV2>(submitThicken);
export const useCompareOperation = createOperationMutation<CompareRequestV2>(submitCompare);
export const useMakeManufacturableOperation = createOperationMutation<MakeManufacturableRequest>(submitMakeManufacturable);
export const useScoopOperation = createOperationMutation<ScoopRequestV2>(submitScoop);
export const useSmoothOperation = createOperationMutation<SmoothRequestV2>(submitSmooth);

export function useCreateInspectionSnapshot() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, params }: { versionId: string; params: InspectionSnapshotState }) =>
      createInspectionSnapshot(versionId, params),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['inspection-snapshots', variables.versionId] });
    },
  });
}

export function useBranchVersion() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ versionId, params }: { versionId: string; params: BranchVersionRequest }) =>
      branchVersion(versionId, params),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['model-versions', data.model_id] });
      queryClient.invalidateQueries({ queryKey: ['version', data.id] });
    },
  });
}

export function useRepairOperation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (versionId: string) => submitRepair(versionId),
    onSuccess: (_data, versionId) => {
      queryClient.invalidateQueries({ queryKey: ['version', versionId] });
    },
  });
}

export function useDownloadModel() {
  return useMutation({
    mutationFn: ({ modelId, format }: { modelId: string; format: 'glb' | 'stl' }) => downloadModel(modelId, format),
    onSuccess: (blob, { modelId, format }) => {
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${modelId}.${format}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    },
  });
}
