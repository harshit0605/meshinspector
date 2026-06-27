# MeshInspector UI Algorithm Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and run a consumer-level validation pass that proves every current MeshInspector workbench command is either fully executable from the official MeshLib UI into the Rust-backed backend or explicitly identified as not yet customer-runnable.

**Architecture:** The official MeshLib workbench runtime remains the UI under test. Playwright drives Chrome against the hosted Next.js app, uploads real model fixtures, dispatches visible workbench commands through the same bridge used by customers, polls backend jobs/results, verifies rendered/generated child versions, and records disabled SDK-only gaps separately from successful customer-ready coverage.

**Tech Stack:** Next.js 16, React 19, Playwright, FastAPI, SQLite/database queue runner for local validation, hosted MeshLib WASM workbench, Rust/PyO3 `geometry_sdk`, and the current `/api/versions/{version_id}/meshlib-workbench` manifest.

---

Date: 2026-06-11
Repo: `/Users/harshit/Code/Zennah/meshinspector`
Primary runtime URL: `http://127.0.0.1:48101`
Primary backend URL: `http://127.0.0.1:48100`

## Current Manifest Snapshot

Authoritative source for this plan:

```bash
curl -fsS http://127.0.0.1:48100/api/versions/ver_8508cac9dad1/meshlib-workbench
jq . meshinspector-frontend/public/meshlib-workbench/runtime/assets/MeshInspectorWorkbenchPlugin.items.json
```

Observed state on 2026-06-11:

| Source | Count |
| --- | ---: |
| Backend workbench commands | 85 |
| Rust-backed backend command capabilities | 78 |
| Endpoint-backed command capabilities | 46 |
| Runtime tools | 5 |
| Official parity inventory features | 13 |
| Static plugin items | 46 |
| Static plugin enabled items | 26 |
| Static plugin disabled/missing-backend items | 20 |

This plan does not treat disabled SDK-only manifest entries as customer-ready. It validates them as visible, disabled, and accurately reported until product endpoints and UI forms exist.

## Command Coverage Inventory

Every command below must be covered by exactly one validation class: full UI execution, UI-forwarded host execution, runtime direct execution, non-geometry UI workflow, or disabled product gap.

### Full-Loop UI Execution Required

These commands must be exercised through the official workbench UI/bridge and must prove backend response, job/result status, and visible loaded-version state when they produce a child version.

- File or export: `upload-new`, `download-stl`, `export-section`
- Prepare: `repair`, `fit-size`, `reduce-weight`, `prepare-casting`, `make-manufacturable`
- Modify: `resize`, `protected-hollow`, `offset-mesh`, `shell-mesh`, `thicken-mesh`, `weighted-shell`, `partial-offset`, `offset-verts`, `expand-shrink`, `shrink-expand`, `hollow-drains`, `thicken-violations`, `thicken-region`, `batch-thicken`, `scoop`, `smooth`, `batch-smooth`, `decimate-mesh`, `subdivide-mesh`
- Inspect/result commands: `section`, `heatmap`, `regions`, `measure-inspect`, `gcode-parse-paths`, `mesh-to-voxels-sdf`, `voxel-boolean`, `collision-detect`, `exact-boolean`
- Review workflows: `compare-versions`, `version-history`, `restore-branch`, `job-activity`
- Runtime bridge tools: `runtime-select-mark-region`, `runtime-selection-to-object`, `runtime-thicken-brush`, `runtime-scoop-brush`, `runtime-smooth-brush`, `runtime-measure-inspect`

### Non-Geometry Workflow Coverage

These commands are not Rust algorithms but still need customer-level loop validation because they are part of the product surface.

- `wireframe`
- `snapshots`
- `version-history`
- `restore-branch`
- `job-activity`
- `download-stl`
- `upload-new`

### SDK-Backed But Not Product-Runnable Yet

These commands expose Rust-backed SDK operations in the manifest without a product endpoint or complete customer UI. The validation pass must assert that each one is visible as disabled/missing-backend in the official UI assets or explicitly listed in the backend manifest as not endpoint-backed. A product-ready claim is false until a UI form, endpoint, payload contract, and full-loop test are added.

- Point cloud and ICP: `point-cloud-icp`
- Distance maps and lines: `distance-map-contours`, `object-lines-from-contours`, `object-lines-to-contours`, `offset-contours`, `object-lines-load-mrlines`, `object-lines-save-mrlines`, `object-lines-load-ply`, `object-lines-save-ply`, `object-lines-load-pts`, `object-lines-load-svg`, `object-lines-save-pts`, `object-lines-save-dxf`, `distance-map-from-mesh`, `distance-map-iso-lines`, `distance-map-merge`, `distance-map-contour-boolean`, `distance-map-from-tiff`, `distance-map-to-tiff`
- Voxel/CT internals: `voxel-binary-operations`, `open-raw-voxels`, `open-voxels-from-tiff`, `voxel-slice`, `voxel-line-graph`, `voxel-active-box`, `voxel-volume-render-data`, `voxel-volume-render-lut`, `voxel-volume-render-ray`, `voxel-segmentation`, `voxel-mask-to-mesh`, `voxel-to-mesh-simple`, `voxel-to-mesh-smart`, `voxel-path`, `voxel-path-build-four`
- G-code file IO internals: `gcode-load-source`, `gcode-write-source`, `gcode-parse-file-paths`

## File Structure

Create or modify these files only for the validation harness:

- Create: `meshinspector-frontend/playwright.config.ts`
  - Defines Chromium/Chrome execution, base URL, timeouts, screenshots, and trace retention.
- Create: `meshinspector-frontend/e2e/support/api.ts`
  - Uploads fixtures, polls jobs, fetches workbench manifests, and verifies child versions/artifacts.
- Create: `meshinspector-frontend/e2e/support/workbenchBridge.ts`
  - Locates the official MeshLib iframe, waits for WebGL canvas readiness, dispatches workbench commands, and returns bridge results.
- Create: `meshinspector-frontend/e2e/fixtures/workbenchCommandCases.ts`
  - Defines payloads and assertions for all customer-runnable commands.
- Create: `meshinspector-frontend/e2e/meshinspector-workbench-algorithms.spec.ts`
  - Runs upload, UI navigation, command dispatch, backend result polling, disabled item assertions, screenshots, and coverage accounting.
- Create: `meshinspector-backend/tests/test_workbench_ui_validation_matrix.py`
  - Fails when backend command capabilities drift from the e2e validation matrix.
- Modify: `meshinspector-frontend/package.json`
  - Adds `e2e:workbench` and `e2e:workbench:headed` scripts.
- Create: `docs/reports/meshinspector-ui-validation/README.md`
  - Documents report output paths and evidence expectations.

No production algorithm code should be changed by this validation plan unless the validation pass exposes a real product gap.

## Validation Data

Use three fixture classes so commands execute quickly and still cover prerequisites:

| Fixture | Purpose | Source |
| --- | --- | --- |
| Tiny closed cube STL | Fast upload, measure, section, decimate, subdivide, SDF, compare, boolean/collision pair setup | Use existing small STL under `meshinspector-backend/storage/uploads/job_3b5555422af8/ver_e4024afa4e5a.stl` for the first pass, then promote to a checked-in e2e fixture if stable |
| Ring-like GLB/STL | Resize, hollow, thickness, region commands, manufacturability workflow | `meshinspector-frontend/models/ring.glb` or a smaller generated ring fixture |
| Two-version pair | Exact boolean, voxel boolean, collision, compare, restore branch | Create by uploading the cube twice and by producing one child version through `resize` or `decimate` |

## Task 1: Add Manifest Coverage Contract

**Files:**
- Create: `meshinspector-backend/tests/test_workbench_ui_validation_matrix.py`
- Read: `meshinspector-backend/api/routers/versions.py`
- Read: `meshinspector-frontend/e2e/fixtures/workbenchCommandCases.ts`

- [ ] **Step 1: Write the failing coverage test**

Add this test file:

```python
from __future__ import annotations

import re
from pathlib import Path

from api.routers import versions as versions_router


REPO_ROOT = Path(__file__).resolve().parents[2]
CASE_FILE = REPO_ROOT / "meshinspector-frontend" / "e2e" / "fixtures" / "workbenchCommandCases.ts"


def _case_command_ids() -> set[str]:
    source = CASE_FILE.read_text(encoding="utf-8")
    return set(re.findall(r"commandId: '([^']+)'", source))


def test_every_customer_runnable_workbench_command_has_an_e2e_case() -> None:
    endpoint_backed = {
        capability["command_id"]
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability.get("endpoint_url_key") is not None
    }
    non_endpoint_ui_workflows = {"upload-new", "wireframe"}
    covered = _case_command_ids()

    assert endpoint_backed | non_endpoint_ui_workflows <= covered


def test_every_non_endpoint_rust_command_is_classified_as_gap_or_runtime_case() -> None:
    runtime_commands = {
        capability["command_id"]
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if str(capability["command_id"]).startswith("runtime-")
    }
    covered = _case_command_ids()
    unendpointed_rust = {
        capability["command_id"]
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability.get("rust_backed") is True
        and capability.get("endpoint_url_key") is None
        and capability["command_id"] not in runtime_commands
    }

    assert unendpointed_rust <= covered
```

- [ ] **Step 2: Run it and verify it fails before the e2e case file exists**

Run:

```bash
cd meshinspector-backend
uv run --extra dev pytest tests/test_workbench_ui_validation_matrix.py -q
```

Expected: fail with a missing `meshinspector-frontend/e2e/fixtures/workbenchCommandCases.ts` file.

## Task 2: Add Playwright Configuration and Scripts

**Files:**
- Create: `meshinspector-frontend/playwright.config.ts`
- Modify: `meshinspector-frontend/package.json`

- [ ] **Step 1: Add Playwright config**

Create `meshinspector-frontend/playwright.config.ts`:

```typescript
import { defineConfig, devices } from '@playwright/test';

const baseURL = process.env.MESHINSPECTOR_BASE_URL ?? 'http://127.0.0.1:48101';

export default defineConfig({
  testDir: './e2e',
  timeout: 120_000,
  expect: { timeout: 20_000 },
  retries: process.env.CI ? 1 : 0,
  fullyParallel: false,
  reporter: [
    ['list'],
    ['html', { outputFolder: '../docs/reports/meshinspector-ui-validation/playwright-html', open: 'never' }],
    ['json', { outputFile: '../docs/reports/meshinspector-ui-validation/playwright-results.json' }],
  ],
  use: {
    baseURL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium-desktop',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 1000 } },
    },
  ],
});
```

- [ ] **Step 2: Add package scripts**

Modify `meshinspector-frontend/package.json` scripts:

```json
{
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "eslint",
    "e2e:workbench": "playwright test e2e/meshinspector-workbench-algorithms.spec.ts",
    "e2e:workbench:headed": "playwright test e2e/meshinspector-workbench-algorithms.spec.ts --headed"
  }
}
```

- [ ] **Step 3: Verify Playwright config loads**

Run:

```bash
cd meshinspector-frontend
npx playwright test --list
```

Expected: command succeeds and lists zero tests until Task 5 adds the spec.

## Task 3: Add API Harness

**Files:**
- Create: `meshinspector-frontend/e2e/support/api.ts`

- [ ] **Step 1: Implement API helpers**

Create `meshinspector-frontend/e2e/support/api.ts`:

```typescript
import { expect, request } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

export const API_BASE = process.env.MESHINSPECTOR_API_URL ?? 'http://127.0.0.1:48100';

export type UploadedVersion = {
  modelId: string;
  versionId: string;
  jobId: string;
};

export async function uploadFixture(filePath: string): Promise<UploadedVersion> {
  const api = await request.newContext({ baseURL: API_BASE });
  const response = await api.post('/api/models', {
    multipart: {
      file: {
        name: path.basename(filePath),
        mimeType: 'application/octet-stream',
        buffer: fs.readFileSync(filePath),
      },
    },
  });
  expect(response.ok()).toBeTruthy();
  const payload = await response.json();
  const modelId = payload.model?.id ?? payload.model_id;
  const versionId = payload.version?.id ?? payload.version_id;
  const jobId = payload.job?.id ?? payload.job_id;
  expect(modelId).toBeTruthy();
  expect(versionId).toBeTruthy();
  expect(jobId).toBeTruthy();
  await waitForJob(jobId);
  return { modelId, versionId, jobId };
}

export async function waitForJob(jobId: string): Promise<Record<string, unknown>> {
  const api = await request.newContext({ baseURL: API_BASE });
  for (let attempt = 0; attempt < 90; attempt += 1) {
    const response = await api.get(`/api/jobs/${jobId}`);
    expect(response.ok()).toBeTruthy();
    const payload = await response.json();
    if (payload.status === 'succeeded') return payload;
    if (payload.status === 'failed') throw new Error(`Job ${jobId} failed: ${payload.error_message ?? payload.error_code}`);
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(`Timed out waiting for job ${jobId}`);
}

export async function getWorkbenchManifest(versionId: string): Promise<Record<string, unknown>> {
  const api = await request.newContext({ baseURL: API_BASE });
  const response = await api.get(`/api/versions/${versionId}/meshlib-workbench`);
  expect(response.ok()).toBeTruthy();
  return response.json();
}

export async function getVersion(versionId: string): Promise<Record<string, unknown>> {
  const api = await request.newContext({ baseURL: API_BASE });
  const response = await api.get(`/api/versions/${versionId}`);
  expect(response.ok()).toBeTruthy();
  return response.json();
}
```

- [ ] **Step 2: Run TypeScript validation**

Run:

```bash
cd meshinspector-frontend
npx tsc --noEmit
```

Expected: no TypeScript errors from the helper.

## Task 4: Add Official Workbench Bridge Harness

**Files:**
- Create: `meshinspector-frontend/e2e/support/workbenchBridge.ts`

- [ ] **Step 1: Implement iframe and dispatch helpers**

Create `meshinspector-frontend/e2e/support/workbenchBridge.ts`:

```typescript
import { expect, type FrameLocator, type Page } from '@playwright/test';

export function workbenchHost(page: Page): FrameLocator {
  return page.frameLocator('iframe[title="MeshLib Workbench"]');
}

export function workbenchRuntime(page: Page): FrameLocator {
  return workbenchHost(page).frameLocator('iframe[title="MeshLib Runtime"]');
}

export async function waitForWorkbenchReady(page: Page): Promise<void> {
  await expect(page.locator('iframe[title="MeshLib Workbench"]')).toBeVisible();
  await expect(workbenchRuntime(page).locator('canvas')).toBeVisible({ timeout: 60_000 });
  const ready = await page.evaluate(() => {
    const host = document.querySelector<HTMLIFrameElement>('iframe[title="MeshLib Workbench"]');
    const runtime = host?.contentDocument?.querySelector<HTMLIFrameElement>('iframe[title="MeshLib Runtime"]');
    return {
      outer: host?.contentDocument?.documentElement.dataset.meshinspectorWorkbenchCommandCount,
      bridge: runtime?.contentDocument?.documentElement.dataset.meshinspectorWorkbenchBridge,
      canvasBridge: runtime?.contentDocument?.documentElement.dataset.meshinspectorWorkbenchCanvasCommandBridge,
      commandCount: runtime?.contentDocument?.documentElement.dataset.meshinspectorWorkbenchCommandCount,
    };
  });
  expect(ready.bridge).toBe('ready');
  expect(ready.canvasBridge).toBe('ready');
  expect(Number(ready.commandCount)).toBeGreaterThanOrEqual(85);
}

export async function dispatchWorkbenchCommand(
  page: Page,
  commandId: string,
  payload: Record<string, unknown>,
  options: Record<string, unknown> = { execute: true },
): Promise<unknown> {
  return page.evaluate(
    async ({ commandId: id, payload: commandPayload, options: commandOptions }) => {
      const host = document.querySelector<HTMLIFrameElement>('iframe[title="MeshLib Workbench"]');
      const runtime = host?.contentDocument?.querySelector<HTMLIFrameElement>('iframe[title="MeshLib Runtime"]');
      const dispatcher = runtime?.contentWindow?.meshinspectorWorkbenchDispatchCommand;
      if (typeof dispatcher !== 'function') {
        throw new Error('meshinspectorWorkbenchDispatchCommand is not available');
      }
      return dispatcher(id, commandPayload, commandOptions);
    },
    { commandId, payload, options },
  );
}
```

- [ ] **Step 2: Run TypeScript validation**

Run:

```bash
cd meshinspector-frontend
npx tsc --noEmit
```

Expected: no TypeScript errors from the bridge helper.

## Task 5: Add Command Case Matrix

**Files:**
- Create: `meshinspector-frontend/e2e/fixtures/workbenchCommandCases.ts`

- [ ] **Step 1: Define executable and gap cases**

Create `meshinspector-frontend/e2e/fixtures/workbenchCommandCases.ts`:

```typescript
export type WorkbenchCommandCase = {
  commandId: string;
  group: 'file' | 'prepare' | 'modify' | 'inspect' | 'review' | 'runtime';
  mode: 'execute' | 'forward' | 'toggle' | 'gap';
  needsSecondVersion?: boolean;
  payload: Record<string, unknown>;
  expectChildVersion?: boolean;
  expectResultKeys?: string[];
};

export const executableCommandCases: WorkbenchCommandCase[] = [
  { commandId: 'upload-new', group: 'file', mode: 'execute', payload: {}, expectChildVersion: false },
  { commandId: 'download-stl', group: 'file', mode: 'forward', payload: {}, expectChildVersion: false },
  { commandId: 'export-section', group: 'file', mode: 'execute', payload: { request: { section_constant: 0, plane_axis: [0, 0, 1] } }, expectChildVersion: false },
  { commandId: 'repair', group: 'prepare', mode: 'forward', payload: { request: {} }, expectChildVersion: true },
  { commandId: 'fit-size', group: 'prepare', mode: 'forward', payload: { request: { target_ring_size_us: 7, axis_mode: 'auto', preserve_head: true } }, expectChildVersion: true },
  { commandId: 'reduce-weight', group: 'prepare', mode: 'forward', payload: { request: { mode: 'target_weight', material: 'gold_14k', target_weight_g: 4.5, wall_thickness_mm: 0.8 } }, expectChildVersion: true },
  { commandId: 'prepare-casting', group: 'prepare', mode: 'forward', payload: { request: { material: 'gold_14k', wall_thickness_mm: 0.8, add_drain_holes: true } }, expectChildVersion: true },
  { commandId: 'make-manufacturable', group: 'prepare', mode: 'forward', payload: { request: { material: 'gold_14k', target_ring_size_us: 7, target_weight_g: 5, min_allowed_thickness_mm: 0.6 } }, expectChildVersion: true },
  { commandId: 'resize', group: 'modify', mode: 'forward', payload: { request: { target_ring_size_us: 8, axis_mode: 'auto', preserve_head: true } }, expectChildVersion: true },
  { commandId: 'protected-hollow', group: 'modify', mode: 'forward', payload: { request: { material: 'gold_14k', wall_thickness_mm: 0.8, protect_regions: ['inner_band'] } }, expectChildVersion: true },
  { commandId: 'offset-mesh', group: 'modify', mode: 'forward', payload: { request: { offset_mm: 0.1, voxel_size_mm: 1, padding_mm: 2, refine: false } }, expectChildVersion: true },
  { commandId: 'shell-mesh', group: 'modify', mode: 'forward', payload: { request: { wall_thickness_mm: 0.6, voxel_size_mm: 1, padding_mm: 2, refine: false } }, expectChildVersion: true },
  { commandId: 'thicken-mesh', group: 'modify', mode: 'forward', payload: { request: { thickness_mm: 0.2, voxel_size_mm: 1, padding_mm: 2, refine: false } }, expectChildVersion: true },
  { commandId: 'weighted-shell', group: 'modify', mode: 'forward', payload: { request: { offset_mm: 0.1, region_weights: [], voxel_size_mm: 1, padding_mm: 2, interpolation_distance_mm: 0, refine: false } }, expectChildVersion: true },
  { commandId: 'partial-offset', group: 'modify', mode: 'forward', payload: { request: { offset_mm: 0.1, region_ids: [], voxel_size_mm: 1, padding_mm: 2, refine: false } }, expectChildVersion: true },
  { commandId: 'offset-verts', group: 'modify', mode: 'forward', payload: { request: { offset_mm: 0.05, region_ids: [] } }, expectChildVersion: true },
  { commandId: 'expand-shrink', group: 'modify', mode: 'forward', payload: { request: { distance_mm: 0.1, voxel_size_mm: 1, padding_mm: 2, refine: false } }, expectChildVersion: true },
  { commandId: 'shrink-expand', group: 'modify', mode: 'forward', payload: { request: { distance_mm: 0.1, voxel_size_mm: 1, padding_mm: 2, refine: false } }, expectChildVersion: true },
  { commandId: 'hollow-drains', group: 'modify', mode: 'forward', payload: { request: { material: 'gold_14k', wall_thickness_mm: 0.8, add_drain_holes: true } }, expectChildVersion: true },
  { commandId: 'thicken-violations', group: 'modify', mode: 'forward', payload: { request: { mode: 'violations_only', target_thickness_mm: 0.8 } }, expectChildVersion: true },
  { commandId: 'thicken-region', group: 'modify', mode: 'forward', payload: { request: { mode: 'selected_region', region_id: 'inner_band', target_thickness_mm: 0.8 } }, expectChildVersion: true },
  { commandId: 'batch-thicken', group: 'modify', mode: 'forward', payload: { request: { mode: 'selected_regions', region_ids: ['inner_band', 'gem_seat'], target_thickness_mm: 0.8 } }, expectChildVersion: true },
  { commandId: 'scoop', group: 'modify', mode: 'forward', payload: { request: { region_id: 'inner_band', depth_mm: 0.2, falloff_mm: 1, keep_min_thickness_mm: 0.6 } }, expectChildVersion: true },
  { commandId: 'smooth', group: 'modify', mode: 'forward', payload: { request: { iterations: 2, strength: 0.25, global_mode: true } }, expectChildVersion: true },
  { commandId: 'batch-smooth', group: 'modify', mode: 'forward', payload: { request: { iterations: 2, strength: 0.25, global_mode: true, region_ids: [] } }, expectChildVersion: true },
  { commandId: 'decimate-mesh', group: 'modify', mode: 'forward', payload: { request: { strategy: 'shortest_edge_first', max_error: 100, target_face_count: 8, stabilizer: 0.001, subdivide_parts: 1, decimate_between_parts: true, collapse_near_not_flippable: false, angle_weighted_dist_to_plane: false, max_deleted_vertices: 2147483647, max_deleted_faces: 2147483647, max_triangle_aspect_ratio: 20, touch_near_bd_edges: true, touch_bd_verts: true, optimize_vertex_pos: true, pack_mesh: true } }, expectChildVersion: true },
  { commandId: 'subdivide-mesh', group: 'modify', mode: 'forward', payload: { request: { max_edge_len: 30, max_edge_splits: 4, subdivide_border: true, curvature_priority: 0, project_on_original_mesh: false, smooth_mode: false, min_sharp_dihedral_angle: 0.5235987755982989, max_tri_aspect_ratio: 0 } }, expectChildVersion: true },
  { commandId: 'section', group: 'inspect', mode: 'forward', payload: { request: { section_constant: 0, plane_axis: [0, 0, 1] } }, expectResultKeys: ['segments'] },
  { commandId: 'heatmap', group: 'inspect', mode: 'toggle', payload: { enabled: true }, expectResultKeys: ['values'] },
  { commandId: 'regions', group: 'inspect', mode: 'execute', payload: { selection: { metadata: { selector: 'largest_component' } }, label: 'E2E largest component' }, expectResultKeys: ['resolved_counts'] },
  { commandId: 'measure-inspect', group: 'inspect', mode: 'execute', payload: { points: [[0, 0, 0]], include_local_thickness: false }, expectResultKeys: ['points'] },
  { commandId: 'gcode-parse-paths', group: 'inspect', mode: 'forward', payload: { request: { source: 'G21\\nG90\\nG1 X1 Y2 Z0.5 F1200\\nG2 X2 Y2 I0.5 J0 F900' } }, expectResultKeys: ['segment_count'] },
  { commandId: 'mesh-to-voxels-sdf', group: 'inspect', mode: 'forward', payload: { request: { voxel_size_mm: 10, surface_offset_voxels: 1, mode: 'unsigned', iso_value: 0, extract_surface: false } }, expectResultKeys: ['shape'] },
  { commandId: 'voxel-boolean', group: 'inspect', mode: 'forward', needsSecondVersion: true, payload: { request: { operation: 'union', voxel_size_mm: 10, padding_mm: 10, refine: false } }, expectChildVersion: true },
  { commandId: 'collision-detect', group: 'inspect', mode: 'forward', needsSecondVersion: true, payload: { request: { first_intersection_only: false, max_pairs: 1000, epsilon: 1e-8 } }, expectResultKeys: ['colliding'] },
  { commandId: 'exact-boolean', group: 'inspect', mode: 'forward', needsSecondVersion: true, payload: { request: { operation: 'union' } }, expectChildVersion: true },
  { commandId: 'wireframe', group: 'inspect', mode: 'toggle', payload: { enabled: true } },
  { commandId: 'snapshots', group: 'inspect', mode: 'forward', payload: { request: { name: 'E2E snapshot' } }, expectResultKeys: ['id'] },
  { commandId: 'compare-versions', group: 'review', mode: 'forward', needsSecondVersion: true, payload: { request: {} }, expectResultKeys: ['volume_delta_mm3'] },
  { commandId: 'version-history', group: 'review', mode: 'toggle', payload: {} },
  { commandId: 'restore-branch', group: 'review', mode: 'forward', payload: { request: { operation_label: 'E2E branch restore' } }, expectChildVersion: true },
  { commandId: 'job-activity', group: 'review', mode: 'toggle', payload: {} },
  { commandId: 'runtime-select-mark-region', group: 'runtime', mode: 'execute', payload: { selection: { metadata: { selector: 'largest_component' } }, label: 'E2E runtime selection' }, expectResultKeys: ['resolved_counts'] },
  { commandId: 'runtime-selection-to-object', group: 'runtime', mode: 'execute', payload: { selection: { metadata: { selector: 'largest_component' } }, create_object: true, label: 'E2E selection object' }, expectResultKeys: ['selected_object_version_id'] },
  { commandId: 'runtime-thicken-brush', group: 'runtime', mode: 'execute', payload: { stroke: { tool_id: 'thicken_brush', selection: { metadata: { selector: 'largest_component' } }, amount_mm: 0.05, falloff_mm: 1, iterations: 1, strength: 0.25 } }, expectChildVersion: true },
  { commandId: 'runtime-scoop-brush', group: 'runtime', mode: 'execute', payload: { stroke: { tool_id: 'scoop_brush', selection: { metadata: { selector: 'largest_component' } }, amount_mm: 0.05, falloff_mm: 1, iterations: 1, strength: 0.25 } }, expectChildVersion: true },
  { commandId: 'runtime-smooth-brush', group: 'runtime', mode: 'execute', payload: { stroke: { tool_id: 'smooth_brush', selection: { metadata: { selector: 'largest_component' } }, amount_mm: 0.05, falloff_mm: 1, iterations: 1, strength: 0.25 } }, expectChildVersion: true },
  { commandId: 'runtime-measure-inspect', group: 'runtime', mode: 'execute', payload: { points: [[0, 0, 0]], include_local_thickness: false }, expectResultKeys: ['points'] },
];

export const sdkOnlyGapCases: WorkbenchCommandCase[] = [
  { commandId: 'point-cloud-icp', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-contours', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-from-contours', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-to-contours', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'offset-contours', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-load-mrlines', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-save-mrlines', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-load-ply', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-save-ply', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-load-pts', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-load-svg', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-save-pts', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'object-lines-save-dxf', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-from-mesh', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-iso-lines', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-merge', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-contour-boolean', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-from-tiff', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'distance-map-to-tiff', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-binary-operations', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'open-raw-voxels', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'open-voxels-from-tiff', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-slice', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-line-graph', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-active-box', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-volume-render-data', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-volume-render-lut', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-volume-render-ray', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-segmentation', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-mask-to-mesh', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-to-mesh-simple', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-to-mesh-smart', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-path', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'voxel-path-build-four', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'gcode-load-source', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'gcode-write-source', group: 'inspect', mode: 'gap', payload: {} },
  { commandId: 'gcode-parse-file-paths', group: 'inspect', mode: 'gap', payload: {} },
];

export const allWorkbenchCommandCases = [...executableCommandCases, ...sdkOnlyGapCases];
```

- [ ] **Step 2: Verify backend coverage contract passes**

Run:

```bash
cd meshinspector-backend
uv run --extra dev pytest tests/test_workbench_ui_validation_matrix.py -q
```

Expected: both tests pass, proving every current backend command is covered or classified.

## Task 6: Add End-to-End Workbench Spec

**Files:**
- Create: `meshinspector-frontend/e2e/meshinspector-workbench-algorithms.spec.ts`
- Read: `meshinspector-frontend/e2e/support/api.ts`
- Read: `meshinspector-frontend/e2e/support/workbenchBridge.ts`
- Read: `meshinspector-frontend/e2e/fixtures/workbenchCommandCases.ts`

- [ ] **Step 1: Implement the e2e validation spec**

Create `meshinspector-frontend/e2e/meshinspector-workbench-algorithms.spec.ts`:

```typescript
import { expect, test } from '@playwright/test';
import path from 'node:path';
import { allWorkbenchCommandCases, executableCommandCases, sdkOnlyGapCases } from './fixtures/workbenchCommandCases';
import { getVersion, getWorkbenchManifest, uploadFixture, waitForJob } from './support/api';
import { dispatchWorkbenchCommand, waitForWorkbenchReady } from './support/workbenchBridge';

const repoRoot = path.resolve(__dirname, '..', '..');
const cubeFixture = path.join(repoRoot, 'meshinspector-backend/storage/uploads/job_3b5555422af8/ver_e4024afa4e5a.stl');

test.describe.serial('MeshInspector official workbench algorithm coverage', () => {
  test('manifest command coverage matches e2e matrix', async () => {
    const uploaded = await uploadFixture(cubeFixture);
    const manifest = await getWorkbenchManifest(uploaded.versionId);
    const manifestCommandIds = new Set((manifest.command_capabilities as { command_id: string }[]).map((item) => item.command_id));
    const caseCommandIds = new Set(allWorkbenchCommandCases.map((item) => item.commandId));

    for (const commandId of manifestCommandIds) {
      expect(caseCommandIds.has(commandId), `${commandId} is missing from e2e cases`).toBeTruthy();
    }
  });

  test('official MeshLib UI boots and exposes bridge metadata', async ({ page }) => {
    const uploaded = await uploadFixture(cubeFixture);
    await page.goto(`/viewer?model=${uploaded.modelId}&version=${uploaded.versionId}&axis_mode=auto`);
    await waitForWorkbenchReady(page);
    await page.screenshot({ path: '../docs/reports/meshinspector-ui-validation/workbench-ready.png' });
  });

  for (const commandCase of executableCommandCases.filter((item) => item.mode !== 'gap')) {
    test(`${commandCase.commandId} closes the UI to backend loop`, async ({ page }) => {
      const uploaded = await uploadFixture(cubeFixture);
      let secondVersionId: string | undefined;
      if (commandCase.needsSecondVersion) {
        const second = await uploadFixture(cubeFixture);
        secondVersionId = second.versionId;
      }

      await page.goto(`/viewer?model=${uploaded.modelId}&version=${uploaded.versionId}&axis_mode=auto`);
      await waitForWorkbenchReady(page);

      const payload = JSON.parse(JSON.stringify(commandCase.payload));
      if (secondVersionId) {
        payload.request = { ...(payload.request ?? {}), other_version_id: secondVersionId };
      }

      const result = await dispatchWorkbenchCommand(page, commandCase.commandId, payload, { execute: true });

      if (commandCase.expectResultKeys) {
        for (const key of commandCase.expectResultKeys) {
          expect(JSON.stringify(result), `${commandCase.commandId} result should include ${key}`).toContain(key);
        }
      }

      if (commandCase.expectChildVersion) {
        const resultText = JSON.stringify(result);
        const jobMatch = resultText.match(/job_[a-z0-9]+/);
        if (jobMatch) {
          const job = await waitForJob(jobMatch[0]);
          const resultJson = job.result_json as { version_id?: string } | undefined;
          expect(resultJson?.version_id ?? job.version_id).toBeTruthy();
          const childVersion = await getVersion(String(resultJson?.version_id ?? job.version_id));
          expect(JSON.stringify(childVersion)).toContain('"status":"ready"');
        } else {
          expect(resultText).toMatch(/version_id|selected_object_version_id/);
        }
      }
    });
  }

  test('SDK-only gaps are not falsely exposed as customer-ready commands', async () => {
    const uploaded = await uploadFixture(cubeFixture);
    const manifest = await getWorkbenchManifest(uploaded.versionId);
    const capabilityById = new Map((manifest.command_capabilities as { command_id: string; endpoint_url_key?: string | null }[]).map((item) => [item.command_id, item]));

    for (const gap of sdkOnlyGapCases) {
      const capability = capabilityById.get(gap.commandId);
      expect(capability, `${gap.commandId} must remain present in manifest for tracking`).toBeTruthy();
      expect(capability?.endpoint_url_key ?? null, `${gap.commandId} must not be counted as customer-runnable until an endpoint exists`).toBeNull();
    }
  });
});
```

- [ ] **Step 2: Run the spec against the local high-port stack**

Run:

```bash
cd meshinspector-frontend
MESHINSPECTOR_BASE_URL=http://127.0.0.1:48101 \
MESHINSPECTOR_API_URL=http://127.0.0.1:48100 \
npm run e2e:workbench
```

Expected:
- Upload succeeds through backend API.
- Viewer page opens with the official MeshLib iframe.
- Runtime dataset contains `meshinspectorWorkbenchBridge=ready`.
- Each endpoint-backed command returns a result or creates a ready child version.
- SDK-only gap commands remain not endpoint-backed and are reported separately.

## Task 7: Add Manual Chrome/Computer Use Checklist

**Files:**
- Create: `docs/reports/meshinspector-ui-validation/README.md`

- [ ] **Step 1: Document manual evidence requirements**

Create `docs/reports/meshinspector-ui-validation/README.md`:

```markdown
# MeshInspector UI Validation Reports

Each full validation pass must preserve:

- `playwright-results.json` from `npm run e2e:workbench`
- Playwright HTML report under `playwright-html/`
- `workbench-ready.png` showing the official MeshLib workbench canvas, scene tree, ribbon, and loaded model
- A Chrome/Computer Use note confirming the real Chrome window rendered the same viewer URL and visible MeshLib UI
- Backend health output from `/health/ready`
- A manifest summary with command count, Rust-backed count, endpoint-backed count, and disabled static plugin count

Manual Chrome check:

1. Open `http://127.0.0.1:48101/viewer?model=<model_id>&version=<version_id>&axis_mode=auto` in Chrome.
2. Confirm the page shows the official MeshLib ribbon, scene tree, model viewport, object information panel, and view cube.
3. Click `Modify`.
4. Confirm the hosted Rust-backed buttons such as `Decimate Mesh` and `Subdivide Mesh` appear.
5. Click one hosted button with no execute payload and confirm it selects/opens the tool instead of mutating unexpectedly.
6. Record any visible runtime warning that affects customer workflow.
```

- [ ] **Step 2: Verify the report directory is present**

Run:

```bash
test -f docs/reports/meshinspector-ui-validation/README.md
```

Expected: command exits `0`.

## Task 8: Run Focused Existing Contract Gates

**Files:**
- Read: `meshinspector-backend/tests/test_geometry_sdk_architecture.py`
- Read: `meshinspector-backend/tests/test_meshinspector_official_parity_inventory.py`
- Read: `meshinspector-frontend/package.json`

- [ ] **Step 1: Run backend manifest and parity gates**

Run:

```bash
cd meshinspector-backend
uv run --extra dev pytest \
  tests/test_geometry_sdk_architecture.py::test_meshlib_workbench_manifest_exposes_command_level_rust_capabilities \
  tests/test_geometry_sdk_architecture.py::test_official_workbench_plugin_assets_expose_parity_inventory_tools \
  tests/test_meshinspector_official_parity_inventory.py \
  tests/test_workbench_ui_validation_matrix.py \
  -q
```

Expected: all selected tests pass.

- [ ] **Step 2: Run frontend lint and e2e**

Run:

```bash
cd meshinspector-frontend
npm run lint
MESHINSPECTOR_BASE_URL=http://127.0.0.1:48101 \
MESHINSPECTOR_API_URL=http://127.0.0.1:48100 \
npm run e2e:workbench
```

Expected: lint and e2e pass. Any failure is treated as either a product bug or a validation-case payload bug and must be triaged with the failing command id.

## Completion Definition

The UI validation goal is complete only when all of these are true:

- Every current command in `/api/versions/{version_id}/meshlib-workbench` is present in `allWorkbenchCommandCases`.
- Every endpoint-backed command has a customer-level UI/bridge test.
- Every mutating algorithm proves a ready child version or expected result artifact.
- Every non-mutating algorithm proves a backend result payload or visible UI state change.
- Every SDK-backed but not product-runnable command is asserted as a gap, not reported as a pass.
- The official MeshLib runtime is verified in Chrome with WebGL canvas, ribbon, scene tree, and command bridge.
- Existing parity inventory tests pass.
- Frontend lint passes.
- A validation report is saved under `docs/reports/meshinspector-ui-validation/`.

## Self-Review

- Spec coverage: The plan covers the full runtime command surface from the live manifest, the static disabled plugin items, backend algorithm endpoints, official workbench UI rendering, Chrome manual validation, and report artifacts.
- Red flag scan: No task uses banned placeholder tokens or unspecified follow-up language. SDK-only items are deliberately classified as product gaps until endpoint/UI contracts exist.
- Type consistency: Command ids match the live 2026-06-11 manifest names; helper names used in specs are defined in the support files above.
