import { expect, request, type APIRequestContext } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

import type {
  ArtifactSummary,
  CreateModelResponse,
  JobResponse,
  MeshLibWorkbenchManifest,
  VersionDetailResponse,
  VersionSummary,
} from '../../src/lib/api/types';

export const API_BASE =
  process.env.MESHINSPECTOR_API_URL ??
  process.env.NEXT_PUBLIC_API_URL ??
  'http://127.0.0.1:48100';

export type UploadedModelFixture = {
  modelId: string;
  versionId: string;
  uploadJobId: string;
};

export type ValidationPrerequisites = UploadedModelFixture & {
  otherVersionId: string;
  regionId: string;
};

export type ValidationPrerequisiteOptions = {
  fixtureName?: string;
  requireEditableRegion?: boolean;
};

export async function expectApiOk(response: { ok(): boolean; status(): number; text(): Promise<string> }): Promise<void> {
  if (response.ok()) {
    return;
  }
  throw new Error(`API request failed with ${response.status()}: ${await response.text()}`);
}

export async function createApiContext(): Promise<APIRequestContext> {
  return request.newContext({ baseURL: API_BASE });
}

export function fixturePath(name: string): string {
  return path.resolve(__dirname, '..', 'fixtures', name);
}

function validationFixturePath(fixtureName?: string): string {
  if (fixtureName) {
    return fixturePath(fixtureName);
  }
  return process.env.MESHINSPECTOR_MODEL_FIXTURE || fixturePath('cube.stl');
}

export function viewerUrl(modelId: string, versionId: string): string {
  return `/viewer?model=${modelId}&version=${versionId}`;
}

export function absoluteApiUrl(endpoint: string | null | undefined): string | null {
  if (!endpoint) return null;
  if (endpoint.startsWith('http://') || endpoint.startsWith('https://')) {
    return endpoint;
  }
  return `${API_BASE}${endpoint}`;
}

export async function uploadFixture(api: APIRequestContext, filePath: string): Promise<UploadedModelFixture> {
  const buffer = await readFile(filePath);
  const response = await api.post('/api/models', {
    multipart: {
      file: {
        name: path.basename(filePath),
        mimeType: filePath.endsWith('.stl') ? 'model/stl' : 'application/octet-stream',
        buffer,
      },
    },
  });
  await expectApiOk(response);
  const payload = (await response.json()) as CreateModelResponse;
  expect(payload.model.id).toMatch(/^mdl_/);
  expect(payload.version.id).toMatch(/^ver_/);
  expect(payload.job?.id).toMatch(/^job_/);

  await waitForJob(api, payload.job!.id);
  await waitForVersionReady(api, payload.version.id);

  return {
    modelId: payload.model.id,
    versionId: payload.version.id,
    uploadJobId: payload.job!.id,
  };
}

export async function waitForJob(
  api: APIRequestContext,
  jobId: string,
  timeoutMs = 120_000,
): Promise<JobResponse> {
  const startedAt = Date.now();
  let latest: JobResponse | null = null;
  while (Date.now() - startedAt < timeoutMs) {
    const response = await api.get(`/api/jobs/${jobId}`);
    await expectApiOk(response);
    latest = (await response.json()) as JobResponse;
    if (latest.status === 'succeeded') {
      return latest;
    }
    if (latest.status === 'failed') {
      throw new Error(`Job ${jobId} failed: ${latest.error_message ?? latest.error_code ?? 'unknown error'}`);
    }
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  throw new Error(`Timed out waiting for job ${jobId}; last status=${latest?.status ?? 'unknown'}`);
}

export async function waitForVersionReady(
  api: APIRequestContext,
  versionId: string,
  timeoutMs = 120_000,
): Promise<VersionDetailResponse> {
  const startedAt = Date.now();
  let latest: VersionDetailResponse | null = null;
  while (Date.now() - startedAt < timeoutMs) {
    latest = await getVersion(api, versionId);
    if (latest.version.status === 'ready') {
      return latest;
    }
    if (latest.version.status === 'failed') {
      throw new Error(`Version ${versionId} failed to prepare`);
    }
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  throw new Error(`Timed out waiting for version ${versionId}; last status=${latest?.version.status ?? 'unknown'}`);
}

export async function getVersion(api: APIRequestContext, versionId: string): Promise<VersionDetailResponse> {
  const response = await api.get(`/api/versions/${versionId}`);
  await expectApiOk(response);
  return (await response.json()) as VersionDetailResponse;
}

export async function getWorkbenchManifest(
  api: APIRequestContext,
  versionId: string,
): Promise<MeshLibWorkbenchManifest> {
  const response = await api.get(`/api/versions/${versionId}/meshlib-workbench`);
  await expectApiOk(response);
  return (await response.json()) as MeshLibWorkbenchManifest;
}

export async function getModelVersions(api: APIRequestContext, modelId: string): Promise<VersionSummary[]> {
  const response = await api.get(`/api/models/${modelId}/versions`);
  await expectApiOk(response);
  return (await response.json()) as VersionSummary[];
}

export async function postJson<T>(
  api: APIRequestContext,
  endpoint: string,
  payload: Record<string, unknown>,
): Promise<T> {
  const response = await api.post(endpoint, {
    data: payload,
    headers: { 'Content-Type': 'application/json' },
  });
  await expectApiOk(response);
  return (await response.json()) as T;
}

export function findArtifact(
  version: VersionDetailResponse,
  artifactType: string,
): ArtifactSummary | null {
  return version.artifacts.find((artifact) => artifact.artifact_type === artifactType) ?? null;
}

export async function expectReadyChildVersion(
  api: APIRequestContext,
  versionId: string,
  artifactType = 'normalized_mesh_ply',
): Promise<VersionDetailResponse> {
  const detail = await waitForVersionReady(api, versionId);
  expect(findArtifact(detail, artifactType), `child version ${versionId} should include ${artifactType}`).toBeTruthy();
  return detail;
}

export async function createValidationPrerequisites(
  api: APIRequestContext,
  options: ValidationPrerequisiteOptions = {},
): Promise<ValidationPrerequisites> {
  const uploaded = await uploadFixture(api, validationFixturePath(options.fixtureName));
  const branch = await postJson<VersionSummary>(
    api,
    `/api/versions/${uploaded.versionId}/branch`,
    { operation_label: 'Workbench validation comparison branch' },
  );
  await waitForVersionReady(api, branch.id);

  const version = await getVersion(api, uploaded.versionId);
  const region =
    version.latest_snapshot?.regions.find((item) => item.region_id === 'inner_band' && item.vertex_count > 0) ??
    version.latest_snapshot?.regions.find((item) => item.allowed_operations.includes('thicken') && item.vertex_count > 0);
  if (options.requireEditableRegion ?? true) {
    expect(region, 'validation fixture should expose at least one editable region').toBeTruthy();
  }

  return {
    ...uploaded,
    otherVersionId: branch.id,
    regionId: region?.region_id ?? 'inner_band',
  };
}
