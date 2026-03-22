/**
 * API methods for model and version operations.
 */

import { fetchApi } from './client';
import type {
  AnalysisResultRaw,
  AnalysisResponse,
  BranchVersionRequest,
  CompareCacheEntry,
  CompareRequestV2,
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
  MaterialType,
  ProcessRequest,
  ProcessResponse,
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

function transformAnalysisResult(raw: AnalysisResultRaw): AnalysisResponse {
  return {
    volume_mm3: raw.volume_mm3,
    weight_grams: raw.weight_g,
    bounding_box: {
      x: raw.bbox_mm[0],
      y: raw.bbox_mm[1],
      z: raw.bbox_mm[2],
    },
    is_watertight: raw.is_watertight,
    vertex_count: raw.vertex_count,
    face_count: raw.face_count,
  };
}

export async function uploadModel(file: File): Promise<CreateModelResponse> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch(`${API_BASE}/api/models`, {
    method: 'POST',
    body: formData,
  });

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

  const response = await fetch(`${API_BASE}/api/versions/${versionId}/interactive-commit`, {
    method: 'POST',
    body: formData,
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.detail || 'Interactive commit failed');
  }
  return response.json();
}

export async function analyzeModel(
  modelId: string,
  material: MaterialType = 'gold_18k',
): Promise<AnalysisResponse> {
  const raw = await fetchApi<AnalysisResultRaw>(`/api/analyze/${modelId}?material=${material}`);
  return transformAnalysisResult(raw);
}

export async function processModel(params: ProcessRequest): Promise<ProcessResponse> {
  return fetchApi<ProcessResponse>('/api/process', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
}

export async function downloadModel(modelId: string, format: 'glb' | 'stl'): Promise<Blob> {
  const response = await fetch(`${API_BASE}/api/download/${modelId}/${format}`);
  if (!response.ok) {
    throw new Error('Download failed');
  }
  return response.blob();
}
