# Workbench UI Testability — Handoff to the workbench plugin workstream

**Date:** 2026-06-23
**From:** productionization/QA pass (Claude-in-Chrome UI test on the local app, snake-ring model)
**To:** the workstream that owns the MeshLib workbench plugin assets

## Why this doc
We tried to verify, **through the UI**, that each geometry operation produces a correct output mesh (e.g. "Boolean difference → result is the correct watertight solid"). The geometry **kernels are already verified correct** (1087 Python + 783 Rust tests + per-operation validations this session: boolean cap → watertight & volume-exact, decimate, hollow fragmentation, resize distortion, etc.). The blockers we hit are all in the **workbench UI layer** (plugin assets + runtime), which this workstream owns — so we're handing them over rather than building a parallel surface.

## Gaps blocking reliable UI-based testing

### 1. Operation payloads are untuned demo values that destroy jewelry-scale models
- **Observed:** Modify → *Decimate Mesh* sends `max_error = 1000 mm`. On a ~10 mm ring this collapsed **20,000 → 38 triangles** (volume 47.985 → 16.366 mm³) — a watertight but unusable blob (confirmed in the Information panel + render).
- **Fix:** replace fixed demo payloads with **sane jewelry-scale defaults** (e.g. decimate by `target_face_ratio ≈ 0.5`, not a 1000 mm error tolerance), or a small **param dialog** per op. Audit *all* operation payloads for jewelry scale (models ~5–20 mm, sub-mm features): offset/shell/thicken distances, smooth iterations, hollow wall thickness, resize target, etc.

### 2. Boolean is not exposed in the UI at all
- **Observed:** no ribbon button for Boolean in any tab (Home/View/Select/CT/Modify/Inspect). Boolean is a **two-mesh** capability and the workbench is single-object.
- **Backend is ready & verified:** `POST /api/versions/{id}/boolean/exact` with body `{ other_version_id, operation, epsilon }`, `operation ∈ {difference, union, intersection, difference_ab, difference_ba, inside_a, inside_b, outside_a, outside_b}`; also `/boolean/voxel`. The exact path is watertight + volume-exact (snake−box: difference 31.467 + intersection 16.518 = snake 47.985 mm³ exactly).
- **Fix:** add a Boolean flow — pick/load a **tool mesh** (a second loaded object, or a generated primitive box/sphere), choose the operation, invoke the endpoint, load the result.

### 3. Home "Prepare" group hides when an object is selected
- **Observed:** Auto Repair / Resize / Reduce Weight / Prepare Casting / Protected Hollow vanish from the ribbon once the loaded object is selected (ribbon ends at "Settings"). They only show with nothing selected — so the natural "select → operate" flow can't reach them.
- **Fix:** keep the Prepare group available with the active object selected.

### 4. "Reduce Weight" no-ops without a region
- **Observed:** clicking it did not create a version (no operation ran). It appears to require a marked region.
- **Fix:** run on the whole mesh by default (or clearly prompt for the region it needs).

### 5. No automatable test surface
- **Observed:** ribbon buttons are HTML overlays **inside the WASM double-iframe**; they are not reachable via the DOM / accessibility tree, so automated UI tests (Playwright / Claude-in-Chrome) can only click by **pixel coordinates** (fragile, breaks on layout changes).
- **Fix:** expose stable `data-testid` / ARIA roles on the ribbon controls, **or** a `postMessage` command API, so operations + params can be invoked deterministically and outputs asserted.

## In-UI output verification (nice-to-have)
The bottom-left **Information panel** already shows Triangles / Vertices / Edges / Volume / Area / Components — excellent for testers. Add explicit **watertight** (boundary-edge count = 0), **manifold** (non-manifold-edge count = 0), and **self-intersection** indicators so "is the output correct?" is confirmable in-UI without backend inspection.

## Ownership / boundary
- Plugin assets: `meshlib-workbench/build-wasm/html/assets/MeshInspectorWorkbenchPlugin.items.json` (ribbon items + command payloads) and `.ui.json`.
- Workbench runtime: `meshinspector-frontend/public/meshlib-workbench/runtime/runtime_bootstrap.js`.
- Frontend host bridge (main frontend, not the plugin): `meshinspector-frontend/src/features/editor/viewer/MeshLibWorkbenchHost.tsx` maps workbench command IDs → backend operations. If you'd rather tune default params host-side, that file is ours — let's coordinate the split.
- Backend endpoints + API client (`meshinspector-frontend/src/lib/api/models.ts` `submit*` + `uploadModel`) are ready and verified.

## Definition of done (re-test plan)
Once the above land: upload the snake, run each operation from the ribbon, and confirm the output mesh is correct **in-UI** via the Information panel (watertight / manifold / sensible volume) + visual — including a Boolean difference producing a clean watertight solid.
