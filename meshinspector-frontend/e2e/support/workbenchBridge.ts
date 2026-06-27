import { expect, type Frame, type Page } from '@playwright/test';

import type { MeshLibWorkbenchManifest } from '../../src/lib/api/types';

type WorkbenchWindow = Window & {
  MeshInspectorWorkbenchBridge?: {
    manifest: MeshLibWorkbenchManifest | null;
  };
  meshinspectorWorkbenchDispatchCommand?: (
    commandId: string,
    payload?: Record<string, unknown>,
    options?: Record<string, unknown>,
  ) => Promise<unknown>;
};

export async function getWorkbenchHostFrame(page: Page): Promise<Frame> {
  const hostFrameElement = page.locator('iframe[title="MeshLib Workbench"]');
  await expect(hostFrameElement).toBeVisible({ timeout: 60_000 });
  const elementHandle = await hostFrameElement.elementHandle();
  const frame = await elementHandle?.contentFrame();
  if (!frame) {
    throw new Error('MeshLib Workbench iframe did not expose a content frame');
  }
  return frame;
}

export async function getRuntimeFrame(page: Page): Promise<Frame> {
  const hostFrame = await getWorkbenchHostFrame(page);
  const runtimeFrameElement = hostFrame.locator('iframe[title="MeshLib Runtime"]');
  await expect(runtimeFrameElement).toBeVisible({ timeout: 90_000 });
  const elementHandle = await runtimeFrameElement.elementHandle();
  const frame = await elementHandle?.contentFrame();
  if (!frame) {
    throw new Error('MeshLib Runtime iframe did not expose a content frame');
  }
  return frame;
}

export async function waitForWorkbenchReady(page: Page): Promise<Frame> {
  const hostFrame = await getWorkbenchHostFrame(page);
  await hostFrame.waitForFunction(
    () => Number(document.documentElement.dataset.meshinspectorWorkbenchCommandCount ?? 0) > 0,
    null,
    { timeout: 90_000 },
  );

  const runtimeFrame = await getRuntimeFrame(page);
  await runtimeFrame.waitForFunction(
    () => {
      const candidate = window as WorkbenchWindow;
      return (
        document.documentElement.dataset.meshinspectorWorkbenchBridge === 'ready' &&
        typeof candidate.meshinspectorWorkbenchDispatchCommand === 'function' &&
        !!candidate.MeshInspectorWorkbenchBridge?.manifest?.version_id
      );
    },
    null,
    { timeout: 90_000 },
  );
  return runtimeFrame;
}

export async function getRuntimeWorkbenchManifest(page: Page): Promise<MeshLibWorkbenchManifest> {
  const runtimeFrame = await waitForWorkbenchReady(page);
  return runtimeFrame.evaluate(() => {
    const candidate = window as WorkbenchWindow;
    const manifest = candidate.MeshInspectorWorkbenchBridge?.manifest;
    if (!manifest) {
      throw new Error('MeshLib runtime manifest is unavailable');
    }
    return manifest;
  });
}

export async function getWorkbenchDataset(page: Page): Promise<Record<string, string | undefined>> {
  const hostFrame = await getWorkbenchHostFrame(page);
  return hostFrame.evaluate(() => ({ ...document.documentElement.dataset }));
}

export async function dispatchWorkbenchCommand(
  page: Page,
  commandId: string,
  payload: Record<string, unknown> = {},
  options: Record<string, unknown> = {},
): Promise<unknown> {
  const runtimeFrame = await waitForWorkbenchReady(page);
  return runtimeFrame.evaluate(
    async ({ commandId: nextCommandId, payload: nextPayload, options: nextOptions }) => {
      const candidate = window as WorkbenchWindow;
      if (typeof candidate.meshinspectorWorkbenchDispatchCommand !== 'function') {
        throw new Error('MeshLib workbench dispatch function is unavailable');
      }
      return candidate.meshinspectorWorkbenchDispatchCommand(nextCommandId, nextPayload, nextOptions);
    },
    { commandId, payload, options },
  );
}

export function expectForwardedDispatchResult(commandId: string, result: unknown): void {
  expect(result, `${commandId} should forward from MeshLib runtime to React host`).toMatchObject({
    status: 'forwarded',
    command_id: commandId,
  });
}
