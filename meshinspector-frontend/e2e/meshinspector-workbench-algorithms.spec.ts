import { expect, test, type APIRequestContext, type Page, type Response } from '@playwright/test';
import { writeFile } from 'node:fs/promises';

import {
  API_BASE,
  absoluteApiUrl,
  createApiContext,
  createValidationPrerequisites,
  expectApiOk,
  expectReadyChildVersion,
  getModelVersions,
  getVersion,
  getWorkbenchManifest,
  viewerUrl,
  waitForJob,
} from './support/api';
import {
  dispatchWorkbenchCommand,
  expectForwardedDispatchResult,
  getRuntimeFrame,
  getRuntimeWorkbenchManifest,
  getWorkbenchDataset,
  waitForWorkbenchReady,
} from './support/workbenchBridge';
import {
  JOB_ID_TOKEN,
  OTHER_VERSION_TOKEN,
  REGION_ID_TOKEN,
  allWorkbenchValidationCases,
  customerRunnableCommandCases,
  sdkOnlyGapCases,
  type WorkbenchValidationCase,
} from './fixtures/workbenchCommandCases';

type ResolvedContext = {
  modelId: string;
  versionId: string;
  otherVersionId: string;
  regionId: string;
  jobId: string;
};

type MatrixResult = {
  commandId: string;
  kind: WorkbenchValidationCase['kind'];
  status: 'passed' | 'failed';
  detail?: string;
};

function selectedValidationCases(): readonly WorkbenchValidationCase[] {
  const rawFilter = process.env.WORKBENCH_COMMAND_IDS ?? process.env.WORKBENCH_COMMAND_FILTER;
  if (!rawFilter) {
    return allWorkbenchValidationCases;
  }
  const requested = new Set(rawFilter.split(',').map((item) => item.trim()).filter(Boolean));
  return allWorkbenchValidationCases.filter((testCase) => requested.has(testCase.commandId));
}

function resolveTokens(value: unknown, context: ResolvedContext): unknown {
  if (value === OTHER_VERSION_TOKEN) return context.otherVersionId;
  if (value === REGION_ID_TOKEN) return context.regionId;
  if (value === JOB_ID_TOKEN) return context.jobId;
  if (Array.isArray(value)) {
    return value.map((item) => resolveTokens(item, context));
  }
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, resolveTokens(item, context)]),
    );
  }
  return value;
}

function resolvedPayload(testCase: WorkbenchValidationCase, context: ResolvedContext): Record<string, unknown> {
  return (resolveTokens(testCase.payload ?? {}, context) ?? {}) as Record<string, unknown>;
}

function resolvedOptions(testCase: WorkbenchValidationCase, context: ResolvedContext): Record<string, unknown> {
  return (resolveTokens(testCase.options ?? {}, context) ?? {}) as Record<string, unknown>;
}

function responsePrefix(endpointUrl: string): string {
  const absolute = absoluteApiUrl(endpointUrl);
  if (!absolute) {
    throw new Error(`Cannot wait for empty endpoint URL`);
  }
  return absolute;
}

function endpointUrlForCase(
  testCase: WorkbenchValidationCase,
  endpointUrl: string | null,
  context: ResolvedContext,
): string {
  if (testCase.commandId === 'restore-branch') {
    return `/api/versions/${context.otherVersionId}/branch`;
  }
  if (!endpointUrl) {
    throw new Error(`${testCase.commandId} does not expose an endpoint URL`);
  }
  return endpointUrl;
}

async function waitForEndpointResponse(
  page: Page,
  endpointUrl: string,
  method: 'GET' | 'POST',
): Promise<Response> {
  const prefix = responsePrefix(endpointUrl);
  const response = await page.waitForResponse(
    (candidate) => candidate.url().startsWith(prefix) && candidate.request().method() === method,
    { timeout: 60_000 },
  );
  expect(response.ok(), `${method} ${prefix} should succeed with ${response.status()}`).toBeTruthy();
  return response;
}

function childVersionIdFromResponse(payload: unknown): string | null {
  if (!payload || typeof payload !== 'object') return null;
  const record = payload as Record<string, unknown>;
  const directId = record.id;
  if (typeof directId === 'string' && directId.startsWith('ver_')) return directId;
  const nestedVersion = record.version;
  if (nestedVersion && typeof nestedVersion === 'object') {
    const nestedId = (nestedVersion as Record<string, unknown>).id;
    if (typeof nestedId === 'string' && nestedId.startsWith('ver_')) return nestedId;
  }
  const selectedObjectId = record.selected_object_version_id;
  if (typeof selectedObjectId === 'string' && selectedObjectId.startsWith('ver_')) return selectedObjectId;
  const resultJson = record.result_json;
  if (resultJson && typeof resultJson === 'object') {
    const resultVersionId = (resultJson as Record<string, unknown>).version_id;
    if (typeof resultVersionId === 'string' && resultVersionId.startsWith('ver_')) return resultVersionId;
  }
  return null;
}

async function navigateToWorkbench(page: Page, context: ResolvedContext): Promise<void> {
  await page.goto(viewerUrl(context.modelId, context.versionId), { waitUntil: 'domcontentloaded' });
  await waitForWorkbenchReady(page);
  const manifest = await getRuntimeWorkbenchManifest(page);
  expect(manifest.version_id).toBe(context.versionId);
}

function capabilityFor(
  manifest: Awaited<ReturnType<typeof getWorkbenchManifest>>,
  commandId: string,
) {
  const capability = manifest.command_capabilities.find((item) => item.command_id === commandId);
  expect(capability, `${commandId} should be advertised in the workbench manifest`).toBeTruthy();
  return capability!;
}

async function expectUrlParam(page: Page, context: ResolvedContext, testCase: WorkbenchValidationCase): Promise<void> {
  if (!testCase.urlParam) return;
  const expectedValue = String(resolveTokens(testCase.urlParam.value, context));
  await expect
    .poll(() => new URL(page.url()).searchParams.get(testCase.urlParam!.name), {
      message: `${testCase.commandId} should set ${testCase.urlParam.name}=${expectedValue}`,
      timeout: 15_000,
    })
    .toBe(expectedValue);
}

async function expectSectionContourVisualFeedback(page: Page, payload: unknown): Promise<void> {
  const record = payload && typeof payload === 'object' ? payload as Record<string, unknown> : {};
  expect(record.segment_count, 'section endpoint should return contour segments for the fixture').toEqual(
    expect.any(Number),
  );
  expect(record.segment_count as number).toBeGreaterThan(0);

  const runtimeFrame = await getRuntimeFrame(page);
  await expect
    .poll(
      () =>
        runtimeFrame.evaluate(() => document.documentElement.dataset.meshinspectorWorkbenchSectionOverlay ?? ''),
      {
        message: 'Section Slice should render an official runtime contour overlay',
        timeout: 15_000,
      },
    )
    .toBe('ready');

  const overlayStats = await runtimeFrame.evaluate(() => ({
    segmentDataset: Number(document.documentElement.dataset.meshinspectorWorkbenchSectionSegmentCount ?? 0),
    lineElements: document.querySelectorAll('[data-meshinspector-section-segment]').length,
    svgReady: Boolean(document.querySelector('[data-meshinspector-section-overlay="ready"] svg')),
  }));

  expect(overlayStats.segmentDataset).toBeGreaterThan(0);
  expect(overlayStats.lineElements).toBe(overlayStats.segmentDataset);
  expect(overlayStats.svgReady).toBe(true);
}

async function validateJobResult(
  api: APIRequestContext,
  testCase: WorkbenchValidationCase,
  jobPayload: unknown,
): Promise<void> {
  expect(jobPayload).toMatchObject({ id: expect.stringMatching(/^job_/) });
  const completed = await waitForJob(api, (jobPayload as { id: string }).id);
  if (testCase.expectChildVersion) {
    const childVersionId = childVersionIdFromResponse(completed);
    expect(childVersionId, `${testCase.commandId} should produce a child version id`).toBeTruthy();
    await expectReadyChildVersion(api, childVersionId!);
  }
}

async function validateDirectResult(
  api: APIRequestContext,
  testCase: WorkbenchValidationCase,
  payload: unknown,
): Promise<void> {
  expect(payload, `${testCase.commandId} should return a JSON payload`).toBeTruthy();
  const record = payload && typeof payload === 'object' ? payload as Record<string, unknown> : {};
  if (testCase.expectSelectedObject) {
    const selectedObjectId = childVersionIdFromResponse(payload);
    expect(selectedObjectId, `${testCase.commandId} should create a selected-object version`).toBeTruthy();
    await expectReadyChildVersion(api, selectedObjectId!);
    return;
  }
  if (testCase.expectChildVersion) {
    if (typeof record.output_face_count === 'number') {
      expect(record.output_face_count, `${testCase.commandId} should produce a non-empty mesh`).toBeGreaterThan(0);
    }
    const childVersionId = childVersionIdFromResponse(payload);
    expect(childVersionId, `${testCase.commandId} should produce a child version id`).toBeTruthy();
    await expectReadyChildVersion(api, childVersionId!);
  }
}

async function exerciseCommandCase(
  page: Page,
  api: APIRequestContext,
  context: ResolvedContext,
  testCase: WorkbenchValidationCase,
): Promise<void> {
  const manifest = await getWorkbenchManifest(api, context.versionId);
  const capability = capabilityFor(manifest, testCase.commandId);
  if (testCase.endpointKey !== undefined) {
    expect(capability.endpoint_url_key).toBe(testCase.endpointKey);
  }
  const endpointUrl = capability.endpoint_url;
  if (testCase.endpointKey) {
    expect(endpointUrl, `${testCase.commandId} should have a product endpoint URL`).toBeTruthy();
  }

  if (testCase.kind === 'sdk-gap') {
    expect(capability.rust_backed, `${testCase.commandId} should still be Rust-backed`).toBe(true);
    expect(capability.endpoint_url, `${testCase.commandId} must not be counted customer-ready without an endpoint`).toBeNull();
    return;
  }

  await navigateToWorkbench(page, context);
  const payload = resolvedPayload(testCase, context);
  const options = resolvedOptions(testCase, context);

  switch (testCase.kind) {
    case 'navigation': {
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      await page.waitForURL((url) => url.pathname === '/', { timeout: 15_000 });
      return;
    }
    case 'download': {
      if (testCase.commandId === 'download-stl') {
        await page.evaluate(() => {
          const windowWithDownloadProbe = window as Window & {
            __meshinspectorOpenedUrls?: string[];
            __meshinspectorOriginalOpen?: typeof window.open;
          };
          windowWithDownloadProbe.__meshinspectorOpenedUrls = [];
          if (!windowWithDownloadProbe.__meshinspectorOriginalOpen) {
            windowWithDownloadProbe.__meshinspectorOriginalOpen = window.open.bind(window);
            window.open = ((url?: string | URL) => {
              if (url !== undefined) {
                windowWithDownloadProbe.__meshinspectorOpenedUrls?.push(String(url));
              }
              return null;
            }) as typeof window.open;
          }
        });
        const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
        expectForwardedDispatchResult(testCase.commandId, result);
        await expect
          .poll(
            () =>
              page.evaluate(() => {
                const windowWithDownloadProbe = window as Window & { __meshinspectorOpenedUrls?: string[] };
                return windowWithDownloadProbe.__meshinspectorOpenedUrls ?? [];
              }),
            {
            message: 'download-stl should open the manufacturing artifact URL',
            timeout: 10_000,
            },
          )
          .toContainEqual(expect.stringContaining('/api/artifacts/'));
        return;
      }

      const responsePromise = waitForEndpointResponse(page, endpointUrl!, testCase.method ?? 'GET');
      const downloadPromise = page.waitForEvent('download', { timeout: 30_000 });
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      await responsePromise;
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toMatch(/\.svg$/);
      return;
    }
    case 'host-job': {
      const responsePromise = waitForEndpointResponse(page, endpointUrl!, testCase.method ?? 'POST');
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      const response = await responsePromise;
      await validateJobResult(api, testCase, await response.json());
      return;
    }
    case 'host-direct': {
      const responsePromise = waitForEndpointResponse(page, endpointUrlForCase(testCase, endpointUrl, context), testCase.method ?? 'POST');
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      const response = await responsePromise;
      await validateDirectResult(api, testCase, await response.json());
      return;
    }
    case 'host-query': {
      const responsePromise = waitForEndpointResponse(page, endpointUrl!, testCase.method ?? 'GET');
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      const response = await responsePromise;
      const responsePayload = await response.json();
      if (testCase.commandId === 'section') {
        await expectSectionContourVisualFeedback(page, responsePayload);
      }
      await expectUrlParam(page, context, testCase);
      return;
    }
    case 'ui-state': {
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      await expectUrlParam(page, context, testCase);
      return;
    }
    case 'version-navigation': {
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      await expect
        .poll(() => new URL(page.url()).searchParams.get('version'), {
          message: `${testCase.commandId} should open the target version from history`,
          timeout: 15_000,
        })
        .toBe(context.otherVersionId);
      const versions = await getModelVersions(api, context.modelId);
      expect(versions.map((version) => version.id)).toContain(context.otherVersionId);
      return;
    }
    case 'job-activity': {
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      expectForwardedDispatchResult(testCase.commandId, result);
      await expectUrlParam(page, context, testCase);
      const jobsResponse = await api.get(`/api/versions/${context.versionId}/jobs`);
      await expectApiOk(jobsResponse);
      const jobs = (await jobsResponse.json()) as Array<{ id: string }>;
      expect(jobs.map((job) => job.id)).toContain(context.jobId);
      return;
    }
    case 'runtime-direct': {
      const responsePromise = waitForEndpointResponse(page, endpointUrl!, testCase.method ?? 'POST');
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      await responsePromise;
      await validateDirectResult(api, testCase, result);
      return;
    }
    case 'runtime-job': {
      const responsePromise = waitForEndpointResponse(page, endpointUrl!, testCase.method ?? 'POST');
      const result = await dispatchWorkbenchCommand(page, testCase.commandId, payload, options);
      await responsePromise;
      await validateJobResult(api, testCase, result);
      return;
    }
    default:
      throw new Error(`Unhandled validation kind: ${testCase.kind}`);
  }
}

test.describe('official MeshLib workbench UI backed by Rust algorithms', () => {
  test('boots the hosted official workbench and exposes the current manifest inventory', async ({ page }) => {
    const api = await createApiContext();
    try {
      const prereqs = await createValidationPrerequisites(api);
      await page.goto(viewerUrl(prereqs.modelId, prereqs.versionId), { waitUntil: 'domcontentloaded' });
      await waitForWorkbenchReady(page);

      const dataset = await getWorkbenchDataset(page);
      const manifest = await getRuntimeWorkbenchManifest(page);

      expect(manifest.command_capabilities).toHaveLength(90);
      expect(Number(dataset.meshinspectorWorkbenchCommandCount)).toBe(90);
      expect(manifest.command_capabilities.filter((capability) => capability.rust_backed)).toHaveLength(83);
      expect(manifest.command_capabilities.filter((capability) => capability.endpoint_url).length).toBeGreaterThanOrEqual(88);
      expect(manifest.official_parity_inventory.length).toBeGreaterThanOrEqual(13);
      expect(dataset.meshinspectorWorkbenchRuntimeTools).toContain('select_mark_region');
    } finally {
      void api.dispose().catch(() => undefined);
    }
  });

  test('covers every current workbench command as executable UI or explicit SDK-only gap', async ({ page }, testInfo) => {
    test.setTimeout(900_000);
    const api = await createApiContext();
    const results: MatrixResult[] = [];
    const failures: string[] = [];

    try {
      const prereqs = await createValidationPrerequisites(api);
      const context: ResolvedContext = {
        modelId: prereqs.modelId,
        versionId: prereqs.versionId,
        otherVersionId: prereqs.otherVersionId,
        regionId: prereqs.regionId,
        jobId: prereqs.uploadJobId,
      };

      const detail = await getVersion(api, context.versionId);
      expect(detail.version.status).toBe('ready');

      const manifest = await getWorkbenchManifest(api, context.versionId);
      const advertised = new Set(manifest.command_capabilities.map((item) => item.command_id));
      const covered = new Set(allWorkbenchValidationCases.map((item) => item.commandId));
      expect([...advertised].sort()).toEqual([...covered].sort());

      const casesToRun = selectedValidationCases();
      expect(casesToRun.length, 'WORKBENCH_COMMAND_IDS should match at least one matrix case').toBeGreaterThan(0);
      const fixtureContexts = new Map<string, ResolvedContext>();
      const contextForCase = async (testCase: WorkbenchValidationCase): Promise<ResolvedContext> => {
        if (!testCase.fixtureName) return context;
        const cached = fixtureContexts.get(testCase.fixtureName);
        if (cached) return cached;
        const fixturePrereqs = await createValidationPrerequisites(api, {
          fixtureName: testCase.fixtureName,
          requireEditableRegion: false,
        });
        const fixtureContext = {
          modelId: fixturePrereqs.modelId,
          versionId: fixturePrereqs.versionId,
          otherVersionId: fixturePrereqs.otherVersionId,
          regionId: fixturePrereqs.regionId,
          jobId: fixturePrereqs.uploadJobId,
        };
        fixtureContexts.set(testCase.fixtureName, fixtureContext);
        return fixtureContext;
      };

      for (const testCase of casesToRun) {
        await test.step(`${testCase.kind}: ${testCase.commandId}`, async () => {
          console.log(`[workbench-e2e] ${testCase.kind}: ${testCase.commandId}`);
          try {
            await exerciseCommandCase(page, api, await contextForCase(testCase), testCase);
            results.push({ commandId: testCase.commandId, kind: testCase.kind, status: 'passed' });
          } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            failures.push(`${testCase.commandId}: ${message}`);
            results.push({ commandId: testCase.commandId, kind: testCase.kind, status: 'failed', detail: message });
            await page.screenshot({
              path: testInfo.outputPath(`${testCase.commandId.replace(/[^a-z0-9-]/gi, '_')}.png`),
              fullPage: true,
            }).catch(() => undefined);
          }
        });
      }

      const resultPath = testInfo.outputPath('workbench-command-matrix-results.json');
      await writeFile(resultPath, JSON.stringify({
        apiBase: API_BASE,
        modelId: context.modelId,
        versionId: context.versionId,
        otherVersionId: context.otherVersionId,
        customerRunnableCount: customerRunnableCommandCases.length,
        sdkOnlyGapCount: sdkOnlyGapCases.length,
        commandFilter: process.env.WORKBENCH_COMMAND_IDS ?? process.env.WORKBENCH_COMMAND_FILTER ?? null,
        results,
        failures,
      }, null, 2));
      await testInfo.attach('workbench-command-matrix-results', {
        path: resultPath,
        contentType: 'application/json',
      });

      expect(failures).toEqual([]);
    } finally {
      void api.dispose().catch(() => undefined);
    }
  });
});
