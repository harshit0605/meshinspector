/**
 * API methods for model and version operations.
 */

import { fetchApi } from './client';
import type {
  BranchVersionRequest,
  CompareCacheEntry,
  CompareRequestV2,
  CompareSummary,
  CreateModelResponse,
  HollowRequestV2,
  InspectionSnapshotResponse,
  InspectionSnapshotState,
  InteractiveCommitRequest,
  JobResponse,
  JobEventResponse,
  MakeManufacturableRequest,
  ManufacturabilitySnapshot,
  MeshLibWorkbenchManifest,
  ResizeRequestV2,
  VersionSummary,
  ScoopRequestV2,
  ScalarOverlayResponse,
  SmoothRequestV2,
  VersionDetailResponse,
  ViewerManifest,
  ThickenRequestV2,
} from './types';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

export async function uploadModel(file: File): Promise<CreateModelResponse> {
  const formData = new FormData();
  formData.append('file', file);

  let response: Response;
  try {
    response = await fetch(`${API_BASE}/api/models`, {
      method: 'POST',
      body: formData,
    });
  } catch (error) {
    throw new Error(
      error instanceof Error && error.message
        ? `Upload request failed: ${error.message}`
        : 'Upload request failed: backend unavailable or network request blocked'
    );
  }

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.detail || 'Upload failed');
  }

  return response.json();
}

export async function getModel(modelId: string) {
  return fetchApi(`/api/models/${modelId}`);
}

export async function getModelVersions(modelId: string): Promise<VersionSummary[]> {
  return fetchApi(`/api/models/${modelId}/versions`);
}

export async function getVersion(versionId: string): Promise<VersionDetailResponse> {
  return fetchApi(`/api/versions/${versionId}`);
}

export async function branchVersion(versionId: string, params: BranchVersionRequest): Promise<VersionSummary> {
  return fetchApi(`/api/versions/${versionId}/branch`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function getManufacturability(versionId: string): Promise<ManufacturabilitySnapshot> {
  return fetchApi(`/api/versions/${versionId}/manuf`);
}

export async function getViewerManifest(versionId: string): Promise<ViewerManifest> {
  return fetchApi(`/api/versions/${versionId}/viewer`);
}

export async function getMeshLibWorkbenchManifest(versionId: string): Promise<MeshLibWorkbenchManifest> {
  return fetchApi(`/api/versions/${versionId}/meshlib-workbench`);
}

export async function getThicknessOverlay(versionId: string): Promise<ScalarOverlayResponse> {
  return fetchApi(`/api/versions/${versionId}/overlays/thickness`);
}

export async function getCompareOverlay(versionId: string, otherVersionId: string): Promise<ScalarOverlayResponse> {
  return fetchApi(`/api/versions/${versionId}/overlays/compare/${otherVersionId}`);
}

export async function getCompareCache(versionId: string): Promise<CompareCacheEntry[]> {
  return fetchApi(`/api/versions/${versionId}/compare-cache`);
}

export async function getCompareSummary(versionId: string, otherVersionId: string): Promise<CompareSummary> {
  return fetchApi(`/api/versions/${versionId}/compare/${otherVersionId}`);
}

export async function getInspectionSnapshots(versionId: string): Promise<InspectionSnapshotResponse[]> {
  return fetchApi(`/api/versions/${versionId}/inspection-snapshots`);
}

export async function createInspectionSnapshot(
  versionId: string,
  params: InspectionSnapshotState,
): Promise<InspectionSnapshotResponse> {
  return fetchApi(`/api/versions/${versionId}/inspection-snapshots`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function getJob(jobId: string): Promise<JobResponse> {
  return fetchApi(`/api/jobs/${jobId}`);
}

export async function streamJobEvents(
  jobId: string,
  handlers: {
    onEvent: (event: JobEventResponse) => void;
    onStatus?: (status: JobResponse) => void;
    onError?: (error: Event) => void;
  },
): Promise<() => void> {
  const source = new EventSource(`${API_BASE}/api/jobs/${jobId}/events`);
  source.onmessage = (event) => {
    handlers.onEvent(JSON.parse(event.data) as JobEventResponse);
  };
  source.addEventListener('status', (event) => {
    handlers.onStatus?.(JSON.parse((event as MessageEvent).data) as JobResponse);
  });
  source.onerror = (event) => {
    handlers.onError?.(event);
    source.close();
  };
  return () => source.close();
}

export async function submitRepair(versionId: string): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/repair`, { method: 'POST' });
}

export async function submitResize(versionId: string, params: ResizeRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/resize`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitHollow(versionId: string, params: HollowRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/hollow`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitThicken(versionId: string, params: ThickenRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/thicken`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitCompare(versionId: string, params: CompareRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/compare`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitSmooth(versionId: string, params: SmoothRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/smooth`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitScoop(versionId: string, params: ScoopRequestV2): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/scoop`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitMakeManufacturable(
  versionId: string,
  params: MakeManufacturableRequest,
): Promise<JobResponse> {
  return fetchApi(`/api/versions/${versionId}/make-manufacturable`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function submitInteractiveCommit(
  versionId: string,
  params: InteractiveCommitRequest,
  meshFile: File,
): Promise<JobResponse> {
  const formData = new FormData();
  formData.append('request_json', JSON.stringify(params));
  formData.append('mesh_file', meshFile);

  let response: Response;
  try {
    response = await fetch(`${API_BASE}/api/versions/${versionId}/interactive-commit`, {
      method: 'POST',
      body: formData,
    });
  } catch (error) {
    throw new Error(
      error instanceof Error && error.message
        ? `Interactive commit request failed: ${error.message}`
        : 'Interactive commit request failed: backend unavailable or network request blocked'
    );
  }
  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.detail || 'Interactive commit failed');
  }
  return response.json();
}
