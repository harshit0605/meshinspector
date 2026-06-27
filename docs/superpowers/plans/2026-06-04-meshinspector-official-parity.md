# MeshInspector Official Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reach feature parity between this hosted MeshInspector app and the official MeshInspector product / MeshLib SDK, with Rust-owned algorithms exposed through the Python `GeometrySDK`, backend workbench capabilities, and the hosted official MeshLib workbench UI.

**Architecture:** MeshLib/MeshInspector remains the behavioral oracle. Each official feature is tracked in the parity inventory, implemented in modular Rust crates, exposed through PyO3 and the Python SDK facade, then enabled in the official workbench manifest only after focused Rust/Python/backend validation gates pass.

**Tech Stack:** Rust workspace with PyO3/maturin, FastAPI backend capability registry, Python `geometry_sdk`, hosted MeshLib workbench plugin manifests, and React/Next.js host integration.

---

Date: 2026-06-04
Repo: `/Users/harshit/Code/Zennah/meshinspector`

## Goal

Reach feature parity between this hosted MeshInspector app and the official MeshInspector product / MeshLib SDK, with production app operations backed by Rust-owned algorithms exposed through `geometry_sdk`, backend workbench capabilities, and the hosted official MeshLib workbench UI.

This plan treats MeshLib/MeshInspector as the behavioral oracle. We should use the local `MeshLib/` source and official docs to understand algorithms and expected outputs, then keep the production implementation Rust-owned behind the existing Python `GeometrySDK` facade.

## Primary Sources

- Official MeshLib feature catalog: https://meshlib.io/feature/
- Official MeshLib feature catalog mirror/current URL: https://meshlib.io/features/
- Official MeshLib documentation home and examples: https://meshlib.io/documentation/
- MeshLib `MR::MeshComponents` C++ reference: https://meshlib.io/documentation/namespaceMR_1_1MeshComponents.html
- MeshLib code samples: https://meshlib.io/documentation/Examples.html
- MeshLib plugin docs: https://meshlib.io/documentation/HowtoAddPluginOverview.html
- MeshInspector product features: https://meshinspector.com/feature/
- MeshInspector Mesh Healer docs: https://meshinspector.com/knowledge-base/mesh-repair/mesh-healer-guide-a-comprehensive-tutorial-for-effective-3d-model-inspection-and-repair/
- MeshInspector offset docs: https://meshinspector.com/knowledge-base/mesh-editing/using-the-offset-tool-in-meshinspector/
- MeshInspector selection docs: https://meshinspector.com/knowledge-base/selection/how-to-use-meshinspectors-select-tools/
- MeshInspector primitive selector docs: https://meshinspector.com/knowledge-base/selection/how-to-use-selector-tool-in-meshinspector/
- MeshInspector graph-cut region docs: https://meshinspector.com/knowledge-base/selection/advanced-segmentation-using-select-region-tool/
- MeshInspector automation/UI tree docs: https://meshinspector.com/knowledge-base/automation/how-to-start-using-python-in-meshinspector/
- Local MeshLib SDK source: `MeshLib/source/`
- Local current hosted workbench plugin: `meshlib-workbench/MeshInspectorWorkbenchPlugin.cpp`

## Current State

The hosted app currently exposes a jewelry-focused workbench, not full official product parity.

- Current hosted workbench plugin exposes 5 runtime tools:
  - `Select / Mark Region`
  - `Thicken Brush`
  - `Scoop Brush`
  - `Smooth Brush`
  - `Measure / Inspect`
- Current backend workbench capability registry is in `meshinspector-backend/api/routers/versions.py`.
- Current frontend command registry is in `meshinspector-frontend/src/features/editor/workspace/toolRegistry.ts`.
- Current Rust crate is in `meshinspector-backend/geometry-rs`.
- Current Rust facade already covers a meaningful subset: repair basics, planar holes, health, stats, ring sizing, regions, thickness, closest point, raycast, signed distance, compare, SDF grid, voxel boolean/offset/shell, hollowing, resize, local brush deformation, smoothing, section, and an in-progress MeshLib-style exact boolean path.
- 2026-06-07 selection update: MeshLib `MeshComponents::getComponents` / `expandToComponents` shared-edge face-component expansion is now Rust-backed through `expand_face_selection_to_components`, exposed via PyO3, `geometry_sdk.core.mesh`, package-root `geometry_sdk.expand_face_selection_to_components`, `default_sdk.expand_face_selection_to_components`, backend `selection.metadata.expand_to_components`, and the hosted workbench parity manifest. MeshLib `MeshTopology::findBdFaces` plus boundary-edge-style selection is now Rust-backed through `select_boundary_faces` / `select_boundary_edges`, exposed through PyO3, `geometry_sdk.core.mesh`, `default_sdk`, and backend selector metadata values `boundary_faces` / `boundary_edges`. MeshLib `MRSelectScreenLasso`-style projected screen-polygon face selection is now Rust-backed through `select_faces_by_screen_polygon`, exposed through PyO3, `geometry_sdk.core.mesh`, package-root `geometry_sdk.select_faces_by_screen_polygon`, `default_sdk`, and backend selector metadata value `screen_lasso_faces`. MeshLib `SelfIntersections::getFaces` strict self-intersecting face selection is now Rust-backed through `self_intersecting_faces`, exposed through `geometry_sdk.spatial.intersections`, `default_sdk`, and backend selector metadata value `self_intersections`.

The official product / SDK surface is broader: mesh repair/healing, boolean, collision, offset, smoothing, decimation, remesh/subdivide, feature measurements, selection tools, compare/reporting, point cloud processing, ICP/global registration, voxels/CT/SDF, distance maps, polyline/G-code workflows, full file format coverage, viewer scene/history/search/camera controls, Python automation, and plugin/ribbon extension.

## Parity Matrix

| Official group | Official references | Current state | Rust/backend target | Hosted UI target |
| --- | --- | --- | --- | --- |
| File, scene, viewer | `MRCommonPlugins/MRRibbonCommonMenuStructure.*.json`, MeshLib file formats page | Partial upload/download/export; no full scene/file/view parity | Add format capability model, importer/exporter contracts, format tests | Mirror Home/View base ribbon: open files, open directory, save object/scene, camera presets, viewport layout, object info |
| Selection | MeshInspector selection docs, `MRSelectScreenLasso`, `MRMeshBoundarySelectionWidget`, `MRMeshComponents`, `SelfIntersections::getFaces` | Select/mark plus semantic ring regions, closest-point brush resolution, Rust-backed shared-edge face-component expansion, Rust-backed boundary face/edge selectors, Rust-backed screen-polygon lasso face selection, and Rust-backed strict self-intersection face selection | Add Inside Part/Overlaps self-intersection modes, degeneracy, overhang, outer-layer, primitive paint/pick/rectangle masks, and graph-cut region segmentation | Replace placeholder with real selection tools and selection-mode controls |
| Mesh repair/healing | Mesh Healer docs, `MRMeshFillHole`, `MRMeshFixer`, `MRFixSelfIntersections`, `MRMeshComponents` | Basic repair and service planar hole filling; SDF rebuild exists | Port fill/stitch variants, degeneracy fixer, normal orientation, component pruning, self-intersection fix, tunnel fix, auto rebuild presets | Mesh Repair tab with Mesh Healer, Local Repair, Auto Repair, repair diagnostics, deviation map |
| Editing/simplification | STL editor docs, `MRMeshDecimate`, `MRMeshRelax`, `MRMeshSubdivide`, `MRFreeFormDeformer`, `MRLaplacian` | Smooth/local deform exists; no official decimate/remesh/subdivide/noise/reposition parity | Add decimation, remesh, subdivide, noise, transform/reposition, Laplacian and freeform deformation parity modules | Mesh Edit tab with Simplify, Remesh, Subdivide, Smooth, Noise, Transform/Reposition |
| Boolean/collision | MeshLib boolean docs, feature page, `MRMeshBoolean`, `MRBooleanOperation`, `MRMeshCollide` | Voxel boolean and exact boolean groundwork; exact parity still in progress | Finish MeshLib-style exact boolean, expose direct/voxel modes, collision/colliding face queries | Advanced Edit/Boolean tools with union, intersection, difference, direct vs voxel mode, collision detection |
| Offset/shell/thickening | MeshInspector offset docs, MeshLib offset and weighted offset docs, `MROffset`, `MRWeightedPointsShell` | Voxel offset/shell and protected hollow exist; product offset modes not fully exposed | Add entire offset, thicken, shell, expand/shrink, partial offset, weighted shell parameters, MeshLib voxel-size/sign modes | Offset tool with modes, voxel size, decimation, selected-region offset |
| Inspection/features/measurement | MeshInspector features/measure docs, `MRFeatures`, `MRFeatureRefine`, `MRSubfeatures`, `MRMeshProject` | Section, closest point, thickness heatmap, basic measure | Add point/line/plane/circle/sphere/cylinder/cone features, refine-to-selection, distance, angle, radius/diameter, feature submeasurements | Features tab, Measure Distance, Measure Angle, create/refine feature panels |
| Compare/QA reports | MeshInspector QA/reporting, MeshLib signed distance docs, `MRMeshMeshDistance`, `MRPointsToMeshProjector` | Signed compare field and summary exist | Add bidirectional distance, Hausdorff-style summaries, thresholds/deviation maps, report artifacts | Compare/Report tab with model-to-model deviation, threshold pass/fail, downloadable report |
| Point clouds and scan-to-mesh | MeshLib point cloud, triangulation, ICP docs, `MRPointCloudTriangulation`, `MRICP`, `MRMultiwayICP` | Missing as product feature | Add point cloud document type, sampling, normals, triangulatePointCloud, fusion, ICP/global registration | Point Cloud tab: load cloud, sample, triangulate, align/register |
| Voxels, CT, SDF | MeshLib SDF docs, feature page, `MRVDBConversions`, `MRMarchingCubes`, `MRDistanceMap`, CT docs | SDF grid and voxel mesh ops exist for mesh workflows; no CT/volume UI | Add DICOM/RAW/TIFF/VDB import, volume-to-mesh, mesh-to-SDF sparse/dense modes, volume rendering payloads, voxel binary ops | CT/Voxels tab: open DICOM/RAW/TIFF/VDB, binary operations, volume render, convert to mesh |
| Distance maps, lines, G-code | MeshLib feature page, `MRDistanceMap`, `MRExtractIsolines`, `MROffsetContours`, `MRObjectLines`, `MRLinesLoad`, `MRLinesSave`, `MRGcodeLoad` | Rust-backed mesh/contour distance maps, iso-lines, map merge, contour boolean, ObjectLines .mrlines/.ply/.pts/.dxf workflows including ASCII and binary little-/big-endian PLY line import/export with RGB vertex colors, TIFF distance-map IO, and G-code source/path workflows exist | Broaden PLY UV/texture variants, finish official data-object UI flows, and keep expanding voxel/volume coverage | Distance Map / Lines / G-code tab with enabled Rust-backed tools and disabled entries for remaining official data-object gaps |
| Automation/plugin API | MeshInspector Python automation docs, MeshLib plugin docs, ribbon JSON | Hosted bridge handles command posts; no full UI tree/parameter API | Add manifest schema for tool parameters/actions/states, backend command introspection, parity inventory endpoint | Hosted MeshLib plugin exposes official-like tabs and tool panels; bridge mirrors visible tool names |

## Implementation Workstreams

### 1. Create a canonical parity inventory

Files:
- `docs/MeshInspector Official Parity Inventory.md`
- `meshinspector-backend/tests/test_meshinspector_official_parity_inventory.py`
- `meshinspector-backend/api/routers/versions.py`

Steps:
- Build a structured inventory with one row per official feature/tool.
- For each row include:
  - official feature label
  - official source URL
  - local MeshLib source/header reference
  - current app status: `implemented`, `partial`, `missing`, `not-applicable`
  - Rust owner module
  - backend command id
  - hosted UI tool id
  - validation oracle
- Add a test that fails when an implemented backend capability lacks inventory coverage.
- Add a test that fails when inventory rows marked `implemented` lack `rust_backed=True` or documented non-geometry rationale.

Acceptance:
- Inventory covers every group in the parity matrix.
- Current 5 hosted plugin tools are marked partial, not full official parity.
- Backend manifest endpoint can return inventory summaries for the frontend.

### 2. Expand hosted official workbench UI surface

Files:
- `meshlib-workbench/MeshInspectorWorkbenchPlugin.cpp`
- `meshlib-workbench/MeshInspectorWorkbenchPlugin.items.json`
- `meshlib-workbench/MeshInspectorWorkbenchPlugin.ui.json`
- `meshlib-workbench/wasm/runtime_bootstrap.js`
- `meshinspector-frontend/public/meshlib-workbench/runtime/assets/MeshInspectorWorkbenchPlugin.items.json`
- `meshinspector-frontend/public/meshlib-workbench/runtime/assets/MeshInspectorWorkbenchPlugin.ui.json`
- `meshinspector-frontend/public/meshlib-workbench/bridge.js`
- `meshinspector-frontend/src/features/editor/viewer/MeshLibWorkbenchHost.tsx`
- `meshinspector-frontend/src/features/editor/workspace/toolRegistry.ts`
- `meshinspector-frontend/src/features/editor/workspace/types.ts`

Steps:
- Mirror the official ribbon groups:
  - Home: file open/save, scene reset, settings
  - View: camera, viewport layout, object info
  - Select: object, component, boundary, overhang, self-intersection, degeneracy, lasso/paint
  - Mesh Repair: Mesh Healer, Fill Holes, Stitch Holes, Fix Tunnels, Unite Close Vertices
  - Mesh Edit: Offset, Boolean, Simplify, Remesh, Subdivide, Smooth, Noise, Transform
  - Inspect/Features: Section, Feature creation, Distance, Angle, Thickness, Collision
  - Compare/Report: distance compare, deviation maps, report export
  - Point Cloud: import, sampling, triangulation, ICP
  - CT/Voxels: DICOM/RAW/TIFF/VDB, binary ops, volume-to-mesh
  - Distance Maps/Lines/G-code: distance maps, iso-lines, polyline/G-code load
- For features not yet implemented, show disabled hosted UI entries with `missing_backend_operation` metadata, not fake execution.
- Generate frontend command ids from the same manifest to avoid drift.
- Keep actual geometry execution in backend commands; hosted workbench posts semantic command payloads to the host bridge.

Acceptance:
- Hosted official MeshLib workbench UI shows the parity tabs/tools instead of only the five jewelry runtime tools.
- Disabled tools make missing Rust ownership explicit.
- `test_geometry_sdk_architecture.py` validates plugin JSON, frontend registry, and backend capability registry stay aligned.

### 2A. Selection tool parity execution order

Files:
- `MeshLib/source/MRMesh/MRMeshComponents.*`
- `MeshLib/source/MRViewer/MRSelectScreenLasso.*`
- `MeshLib/source/MRViewer/MRSelectCurvaturePreference.*`
- `MeshLib/source/MRMesh/MRRegionBoundary.*`
- `MeshLib/source/MRMesh/MRMeshSegmentation.*`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/mesh.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-py/src/mesh.rs`
- `meshinspector-backend/geometry_sdk/core/mesh.py`
- `meshinspector-backend/api/routers/versions.py`
- `meshlib-workbench/MeshInspectorWorkbenchPlugin.items.json`
- `meshinspector-frontend/public/meshlib-workbench/runtime/assets/MeshInspectorWorkbenchPlugin.items.json`
- `meshinspector-backend/tests/test_geometry_sdk_core.py`
- `meshinspector-backend/tests/test_geometry_sdk_operation_contracts.py`
- `meshinspector-backend/tests/test_meshinspector_official_parity_inventory.py`

- [x] **Step 1: Implement MeshComponents-style face-component expansion**

Rust/Python/backend behavior:
```text
Given selected face ids, expand to all faces in each selected shared-edge connected component.
Use MeshLib default face incidence semantics: faces are adjacent through shared undirected edges.
Expose app opt-in through selection.metadata.expand_to_components.
```

Validation:
```bash
cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend/geometry-rs
cargo test -p zennah-geometry-core expand_face_selection_to_components -- --nocapture

cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend
uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_expand_face_selection_to_components_matches_meshlib_component_selection -q
uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_expands_selected_faces_to_meshlib_components -q
```

- [x] **Step 2: Add boundary triangle and boundary edge selectors**

Rust API target:
```rust
pub fn select_boundary_faces(vertices: &[[f64; 3]], faces_i64: &[[i64; 3]]) -> Result<Vec<i64>, GeometryError>;
pub fn select_boundary_edges(vertices: &[[f64; 3]], faces_i64: &[[i64; 3]]) -> Result<Vec<[i64; 2]>, GeometryError>;
```

Python/API target:
```python
from geometry_sdk.core.mesh import select_boundary_faces, select_boundary_edges
selection = InteractiveSelectionPayload(
    mode="faces",
    face_ids=select_boundary_faces(mesh),
    metadata={"source": "meshlib_select_boundary_tris"},
)
```

Validation:
```bash
cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend/geometry-rs
cargo test -p zennah-geometry-core select_boundary -- --nocapture

cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend
uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_boundary_faces_matches_meshlib_boundary_tris -q
uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_rust_boundary_face_selection -q
```

- [x] **Step 3: Add selector primitive: screen-polygon lasso face masks**

Rust API target:
```rust
pub fn select_faces_by_screen_polygon(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    view_projection_4x4: &[f64; 16],
    polygon_xy: &[[f64; 2]],
    include_backfaces: bool,
    visible_only: bool,
) -> Result<Vec<i64>, GeometryError>;
```

Validation:
```bash
cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend/geometry-rs
cargo test -p zennah-geometry-core select_faces_by_screen_polygon -- --nocapture

cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend
uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_lasso_mask -q
```

Remaining selector primitive gaps: point pick, paint/brush mask replay beyond closest-point brush resolution, rectangle payloads as a first-class Workbench selector, edge screen loop behavior, include-boundary edge expansion, and point-cloud primitive lasso selection.

- [x] **Step 3B: Add strict self-intersection face selector**

Rust/Python/backend behavior:
```text
Use MeshLib SelfIntersections::getFaces-style face selection by routing selector metadata `self_intersections` through the Rust `self_intersecting_faces` kernel. The selector supports the MeshLib touchIsIntersection flag as `selection.metadata.touch_is_intersection`.
```

Validation:
```bash
cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend
uv run --extra dev pytest tests/test_geometry_sdk_spatial.py::test_triangle_intersection_detects_crossing_faces -q
uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_selector -q
```

Remaining self-intersection selector gaps: Inside Part and Overlaps modes from the official Select Self-Intersections panel.

- [ ] **Step 4: Add graph-cut Select Region parity**

Rust API target:
```rust
pub fn graph_cut_select_region(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    source_face_ids: &[usize],
    sink_face_ids: &[usize],
    boundary_weight: f64,
) -> Result<Vec<i64>, GeometryError>;
```

Validation:
```bash
cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend/geometry-rs
cargo test -p zennah-geometry-core graph_cut_select_region -- --nocapture

cd /Users/harshit/Code/Zennah/meshinspector/meshinspector-backend
uv run --extra dev pytest tests/test_geometry_sdk_parity.py::test_graph_cut_select_region_matches_meshinspector_select_region_fixture -q
```

### 3. Repair and Mesh Healer parity

Reference:
- `MeshLib/source/MRMesh/MRMeshFillHole.*`
- `MeshLib/source/MRMesh/MRFillHoleNicely.*`
- `MeshLib/source/MRMesh/MRMeshFixer.*`
- `MeshLib/source/MRMesh/MRFixSelfIntersections.*`
- `MeshLib/source/MRMesh/MRMeshComponents.*`

Files:
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/repair.rs`
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/repair/`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-py/src/repair.rs`
- `meshinspector-backend/geometry_sdk/repair/`
- `meshinspector-backend/services/operations.py`
- `meshinspector-backend/tests/test_geometry_sdk_repair.py`
- `meshinspector-backend/tests/test_geometry_sdk_parity.py`

Steps:
- Split repair into modules: `degenerate`, `holes`, `stitch`, `normals`, `components`, `self_intersections`, `tunnels`, `auto_rebuild`.
- Port MeshLib-style hole representative edge discovery and fill metrics beyond planar fan fill.
- Add stitch-two-holes parity using ordered loop pairing and bridge triangulation.
- Add degeneracy fixer with max deviation and tiny edge length settings.
- Add component classification/pruning by area and connected component.
- Add self-intersection detection plus local fix lifecycle that matches MeshLib output metrics.
- Add tunnel detection/fix as a separate operation.
- Add Mesh Healer report schema: counts by issue type, large/small buckets, selected repair list, added/removed/deviation payload.

Acceptance:
- MeshLib oracle tests run official `mrmeshpy` repair operations against fixture meshes and compare closure, hole counts, self-intersection counts, area/volume envelope, and component counts.
- Local repair and auto repair are exposed in the hosted Mesh Repair tab and backend capabilities.

### 4. Boolean, collision, and exact topology parity

Reference:
- `MeshLib/source/MRMesh/MRMeshBoolean.*`
- `MeshLib/source/MRMesh/MRBooleanOperation.*`
- `MeshLib/source/MRMesh/MRMeshBooleanFacade.*`
- `MeshLib/source/MRMesh/MRMeshCollide.*`
- `MeshLib/source/MRMesh/MRMeshCollidePrecise.*`

Files:
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/spatial/exact_boolean*`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/spatial/exact_*`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-py/src/boolean.rs`
- `meshinspector-backend/geometry_sdk/spatial/boolean.py`
- `meshinspector-backend/tests/test_geometry_sdk_spatial.py`
- `meshinspector-backend/tests/test_geometry_sdk_parity.py`

Steps:
- Finish current MeshLib-style contour lifecycle parity before adding more boolean UI.
- Close the remaining prepared-base/copied-face ring and near-stitch parity gaps.
- Promote `exact_boolean_mesh` only after union/intersection/difference satisfy closed topology, manifoldness, volume envelope, and MeshLib source-map expectations.
- Add direct boolean and voxel boolean as separate exposed modes.
- Add collision and precise collision face-pair APIs.

Acceptance:
- Exact boolean parity fixtures pass for disjoint, contained, intersecting, coplanar, open, degenerate, and jewelry-region meshes.
- Collision selection tools can select colliding faces between two loaded models.

### 5. Offset, shell, and partial offset parity

Reference:
- `MeshLib/source/MRVoxels/MROffset.*`
- `MeshLib/source/MRVoxels/MRWeightedPointsShell.*`
- `MeshLib/source/MRMesh/MROffsetVerts.*`
- `MeshLib/source/MRMesh/MROffsetContours.*`

Files:
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/voxel_mesh_ops.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/hollow.rs`
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/offset.rs`
- `meshinspector-backend/geometry_sdk/voxel/mesh_ops.py`
- `meshinspector-backend/geometry_sdk/jewelry/hollow.py`
- `meshinspector-backend/tests/test_geometry_sdk_voxel.py`

Steps:
- Expose official offset modes:
  - entire model offset
  - thickening
  - shell
  - expand/shrink
  - selected-region/partial offset
  - weighted shell
- Add MeshLib sign detection mode equivalents, including hole winding rule.
- Add decimation-after-offset option once simplification is available.
- Add voxel-size suggestion parity and memory guardrails.

Acceptance:
- MeshLib `offsetMesh`, `generalOffsetMesh`, and `WeightedShell::meshShell` fixtures match Rust outputs by volume/area/bounds/topology envelopes.
- Hosted Offset tool supports all official modes and selected-region input.

### 6. Simplification, remesh, subdivision, smoothing, and deformation parity

Reference:
- `MeshLib/source/MRMesh/MRMeshDecimate.*`
- `MeshLib/source/MRMesh/MRMeshRelax.*`
- `MeshLib/source/MRMesh/MRMeshSubdivide.*`
- `MeshLib/source/MRMesh/MRLaplacian.*`
- `MeshLib/source/MRMesh/MRFreeFormDeformer.*`
- `MeshLib/source/MRViewer/MRSurfaceManipulationWidget.*`

Files:
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/simplify.rs`
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/remesh.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/deform.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/deform_smooth.rs`
- `meshinspector-backend/geometry_sdk/deform/`
- `meshinspector-backend/tests/test_geometry_sdk_brushes.py`
- `meshinspector-backend/tests/test_geometry_sdk_parity.py`

Steps:
- Add decimation settings: max deleted faces, max error, preserve boundary, preserve selected region, subdivide parts.
- Add remesh and subdivision operations.
- Add reduce/add noise.
- Expand smoothing to boundary smoothing, region-boundary smoothing, Laplacian, relax, and Taubin variants.
- Add freeform deformation and Laplacian handle/constraint deformation.
- Keep interactive brush operations backed by Rust replay payloads, not just MeshLib viewport state.

Acceptance:
- MeshLib decimation and smoothing fixtures match face-count, error, boundary preservation, and scalar envelope expectations.
- Hosted Mesh Edit tab exposes official editing operations with parameter panels.

### 7. Feature creation and measurement parity

Reference:
- `MeshLib/source/MRMesh/MRFeatures.*`
- `MeshLib/source/MRMesh/MRFeatureRefine.*`
- `MeshLib/source/MRMesh/MRFeatureHelpers.*`
- `MeshLib/source/MRMesh/MRMeshProject.*`

Files:
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/features.rs`
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-py/src/features.rs`
- new `meshinspector-backend/geometry_sdk/analysis/features.py`
- `meshinspector-backend/domain/schemas.py`
- `meshinspector-backend/api/routers/versions.py`
- `meshinspector-frontend/src/features/editor/panels/`

Steps:
- Add feature primitives: point, line, plane, circle, sphere, cylinder, cone.
- Add pick-points and fit-to-selection constructors.
- Add refine against mesh/point cloud with distance and normal tolerances.
- Add measure distance, angle, center distance, diameter/radius, and subfeature measurements.
- Persist features as version artifacts so measurements are reproducible.

Acceptance:
- MeshLib `Features::measure` parity tests cover supported feature pairs.
- Hosted Features tab can create, refine, select, and measure features.

### 8. Compare, distance, and QA reporting parity

Reference:
- `MeshLib/source/MRMesh/MRMeshMeshDistance.*`
- `MeshLib/source/MRMesh/MRPointsToMeshProjector.*`
- MeshInspector QA/reporting feature docs

Files:
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/analysis.rs`
- `meshinspector-backend/geometry_sdk/analysis/compare.py`
- `meshinspector-backend/geometry_sdk/analysis/artifacts.py`
- `meshinspector-backend/services/operations.py`
- `meshinspector-frontend/src/features/editor/panels/ComparePanel.tsx`

Steps:
- Add bidirectional surface distance fields.
- Add signed and unsigned modes.
- Add min/max/mean/RMS/Hausdorff-like report summary.
- Add tolerance thresholds and pass/fail result.
- Add deviation map artifact and downloadable report.

Acceptance:
- MeshLib `findSignedDistance` / `findSignedDistances` parity fixtures pass.
- Compare UI can inspect deviations, threshold failures, and report artifacts.

### 9. Point cloud, triangulation, ICP, and scan-to-mesh parity

Reference:
- `MeshLib/source/MRMesh/MRPointCloudTriangulation.*`
- `MeshLib/source/MRMesh/MRICP.*`
- `MeshLib/source/MRMesh/MRMultiwayICP.*`
- MeshLib point-cloud-to-mesh docs

Files:
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/point_cloud.rs`
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/registration.rs`
- new `meshinspector-backend/geometry-rs/crates/zennah-geometry-py/src/point_cloud.rs`
- new `meshinspector-backend/geometry_sdk/point_cloud/`
- `meshinspector-backend/domain/schemas.py`
- frontend Point Cloud panels

Steps:
- Add `PointCloudDocument` type and artifact storage.
- Add file import for supported point cloud formats.
- Add uniform/grid sampling.
- Add point cloud triangulation.
- Add fusion path where MeshLib exposes a matching workflow.
- Add pairwise ICP and multiway ICP with point-to-point and point-to-plane modes.
- Add scan/CAD alignment workflow in the app.

Acceptance:
- MeshLib `triangulatePointCloud`, `ICP`, and `MultiwayICP` fixtures pass transformation and output-mesh envelopes.
- Hosted Point Cloud tab can import, sample, triangulate, and align.

### 10. Voxels, CT, SDF, distance maps, lines, and G-code parity

Reference:
- `MeshLib/source/MRVoxels/MRVDBConversions.*`
- `MeshLib/source/MRVoxels/MRMarchingCubes.*`
- `MeshLib/source/MRMesh/MRDistanceMap.*`
- `MeshLib/source/MRMesh/MRExtractIsolines.*`
- `MeshLib/source/MRMesh/MROffsetContours.*`
- `MeshLib/source/MRMesh/MRObjectLoad.*`

Files:
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/sdf_grid.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/sdf_marching.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/distance.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/distance_tiff.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/lines.rs`
- `meshinspector-backend/geometry-rs/crates/zennah-geometry-core/src/gcode.rs`
- new `meshinspector-backend/geometry_sdk/volumes/`
- `meshinspector-backend/geometry_sdk/distance_map/`
- `meshinspector-backend/geometry_sdk/gcode/`

Steps:
- Extend current SDF grid into official dense/sparse/function-volume modes.
- Add DICOM/RAW/TIFF/VDB import metadata and conversion to mesh.
- Add volume rendering artifact payloads for the hosted viewer.
- Add voxel binary operations matching official Union/Intersection/Difference/Max/Min/Sum/Multiply/Divide/Replace.
- Keep distance map generation from mesh and contours aligned with MeshLib pixel-center sampling.
- Keep distance map iso-lines, merge operations, contour boolean composition, and TIFF IO aligned with MeshLib fixtures.
- Keep ObjectLines .mrlines/.ply/.pts/.dxf workflows Rust-backed; next PLY hardening after ASCII and binary little-/big-endian vertex/edge/color import and MeshLib-style binary little-endian color export is UV, texture, and broader third-party variant coverage.
- Keep G-code source/path workflows aligned with MeshLib GcodeLoad/GcodeProcessor fixtures.

Acceptance:
- MeshLib SDF, marching cubes, distance map, iso-line, ObjectLines, TIFF, and G-code fixtures pass.
- Hosted CT/Voxels and Distance Map tabs expose official workflows.

### 11. File format parity

Reference:
- MeshLib file format support docs
- `MeshLib/source/MRMesh/MRObjectLoad.*`
- `MeshLib/source/MRIOExtras/`

Files:
- `meshinspector-backend/geometry_sdk/io/`
- `meshinspector-backend/services/ingest.py`
- `meshinspector-backend/services/sdk_conversion.py`
- `meshinspector-backend/tests/test_geometry_sdk_artifacts.py`

Steps:
- Inventory supported formats by object type:
  - mesh: STL, OBJ, OFF, DXF, STEP/STP import, CTM, 3MF, MODEL, PLY, GLTF
  - point cloud: ASC, CSV, E57, LAS, LAZ, PTS, XYZ, TXT, PLY
  - voxel: DICOM, RAW, TIFF, VDB
  - polyline: DXF, GAV, PTS, SVG
  - distance map: PNG, JPEG
  - G-code: GCODE, NC
- Add import/export adapters or explicitly mark formats requiring MeshLib service fallback.
- Add conversion matrix endpoint and frontend format warnings.

Acceptance:
- Test matrix proves either implemented import/export or explicit unsupported rationale for every official format.
- App UI no longer implies STL/GLB-only parity.

## Cross-Cutting Validation Strategy

Use MeshLib as the oracle in tests, but keep production paths Rust-owned.

Required validation layers:
- Rust unit tests for each new kernel.
- PyO3 binding tests for NumPy shape/type/contracts.
- Python `geometry_sdk` parity tests using `geometry_sdk/adapters/meshlib_reference.py`.
- Backend operation contract tests for each exposed command.
- Frontend registry tests for every hosted workbench tool.
- Browser smoke tests against the hosted official workbench UI.

Core commands:

```bash
cd meshinspector-backend/geometry-rs
cargo fmt --all --check
cargo test -p zennah-geometry-core --lib
cargo test --workspace
```

```bash
cd meshinspector-backend
uv tool run maturin develop --manifest-path geometry-rs/crates/zennah-geometry-py/Cargo.toml
GEOMETRY_SDK_ACCELERATOR=rust uv run --extra dev pytest tests/test_geometry_sdk_parity.py -q
GEOMETRY_SDK_ACCELERATOR=rust uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py tests/test_geometry_sdk_architecture.py -q
uv run --extra dev pytest -q
```

```bash
cd meshinspector-frontend
npm run lint
npm run build
```

```bash
cd meshlib-workbench
./build_wasm.sh
```

## Rollout Order

1. Inventory plus manifest alignment.
2. Hosted UI expansion with disabled missing tools.
3. Repair/Mesh Healer parity.
4. Offset/shell and simplification parity.
5. Feature measurement and selection parity.
6. Compare/report parity.
7. Boolean exact parity promotion.
8. Point cloud/ICP parity.
9. Voxel/CT/SDF parity.
10. Remaining PLY UV/texture, distance data-object UI, and file-format parity.

This order gives users visible official UI parity early while preventing unsupported tools from pretending to run. Each stage should move tools from disabled to enabled only after Rust parity tests and backend operation contracts pass.

## Non-Negotiable Completion Criteria

- Every official feature row has a status, owner, source reference, and validation gate.
- Every enabled hosted UI command maps to a backend capability.
- Every geometry-mutating backend capability has Rust-owned implementation or a documented temporary MeshLib oracle-only test path.
- Every Rust implementation has MeshLib oracle tests or a written reason why MeshLib has no comparable API.
- The final parity claim is only valid after full Rust, backend, frontend, workbench, and browser validation pass.
