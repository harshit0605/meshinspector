# MeshLib Workbench Scaffold

This directory is the source scaffold for the MeshLib-based interactive workbench that will eventually replace the classic React Three Fiber viewport.

## What is here

- `ViewerApp.cpp`
  - MeshLib viewer entry point with ribbon, scene panel, search bar, notifications, and viewport widgets enabled through `RibbonMenuUIConfig`.
- `MeshInspectorWorkbenchPlugin.cpp`
  - Initial interactive tool scaffold using `MR::SurfaceManipulationWidget` for:
    - `Thicken Brush`
    - `Scoop Brush`
    - `Smooth Brush`
  - Plus placeholders for:
    - `Select / Mark Region`
    - `Measure / Inspect`
- `MeshInspectorWorkbenchPlugin.items.json`
  - Ribbon item metadata.
- `MeshInspectorWorkbenchPlugin.ui.json`
  - Ribbon tab/group layout for the workbench.
- `wasm/index.html`
  - Emscripten viewer shell copied into the generated runtime.
- `build_wasm.sh`
  - Packaging script that builds the workbench and installs the runtime bundle into:
    - `/Users/harshit/Code/Zennah/meshinspector/meshinspector-frontend/public/meshlib-workbench/runtime`

## Intended integration

The Next.js app now exposes:

- `GET /api/versions/{version_id}/meshlib-workbench`
  - Returns runtime entry URLs, mesh artifact URLs, built-in UI expectations, and feature flags.
- `POST /api/versions/{version_id}/interactive-commit`
  - Accepts an edited mesh plus tool metadata and creates a new derived version through the normal job/version pipeline.

The frontend viewer now prefers the MeshLib host seam when a compiled runtime bundle is installed in `/public/meshlib-workbench/runtime`. Until then, it falls back to the existing classic viewer.

## Current status

This scaffold is checked in and wired to the application shell, but it is not yet guaranteed to compile out of the box in this repository without finishing MeshLib's WASM build/dependency setup.

The main unresolved build concern is the custom MeshLib Emscripten app packaging step, not the MeshInspector app-side integration.
