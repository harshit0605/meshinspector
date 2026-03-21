# MeshLib Jewelry Manufacturability Architecture

## Goal

Build a jewelry-focused mesh editing and manufacturability platform on top of the current `meshinspector` repo so that image-to-3D outputs from systems like Tripo can be turned into production-ready ring and jewelry meshes.

The product objective is not just "view a generated mesh". It is to give users control over:

- manufacturability
- ring sizing and dimensional correctness
- wall thickness and hollowing
- target weight by material
- repair of bad generator output
- local mesh edits without returning to the generator
- export of a final mesh suitable for casting, printing, and downstream CAD review

MeshLib is a strong fit because its public feature set directly covers repair, offsetting, booleans, hole filling, deformation, comparison, distance fields, and viewer workflows.

## Why This Matters For Image-To-3D Jewelry

Image-to-3D outputs are visually compelling but are usually weak in one or more manufacturing dimensions:

- non-watertight meshes
- self-intersections
- thin spikes and fragile ornament regions
- inconsistent units and scale
- incorrect inner diameter for rings
- no guarantee of minimum wall thickness
- no predictable metal weight
- ornamental detail that is too noisy or too dense
- topology that is hard to edit locally

For jewelry, this matters more than in generic 3D viewing because a ring has hard constraints:

- the inner diameter must match a target size
- the band must not violate minimum thickness
- the model must meet a weight class for the selected metal
- the mesh must be closed and printable/castable
- detail must survive finishing and polishing

MeshLib gives us the geometry kernel to enforce those rules after generation.

## Current Repo Baseline

The current repo already has the right overall shape:

- upload a model
- convert it to STL internally
- analyze basic geometry
- resize and hollow the mesh
- preview and download processed output

Relevant current implementation points:

- Backend analysis: `meshinspector-backend/services/analyze.py`
- Backend hollowing: `meshinspector-backend/services/hollow.py`
- Backend health and repair: `meshinspector-backend/services/health.py`
- Backend thickness: `meshinspector-backend/services/thickness.py`
- Processing endpoint: `meshinspector-backend/api/routes/process.py`
- Health endpoints: `meshinspector-backend/api/routes/health.py`
- Viewer page: `meshinspector-frontend/src/app/viewer/page.tsx`
- Tool panel: `meshinspector-frontend/src/components/ToolPanel.tsx`
- Hollow panel: `meshinspector-frontend/src/components/tools/HollowToolPanel.tsx`
- Inspect panel: `meshinspector-frontend/src/components/tools/InspectToolPanel.tsx`
- 3D viewer: `meshinspector-frontend/src/components/ModelViewer.tsx`

### What Already Works

- Upload and preview flow is present.
- Material-aware weight estimation exists.
- Fixed-thickness hollowing exists.
- Target-weight hollowing exists via binary search.
- Basic health inspection and auto-repair endpoints exist.
- Section clipping exists in the frontend viewer.

### Current Gaps

- Thickness analysis is a heuristic placeholder, not a real MeshLib-backed computation.
- Frontend inspect panel is not wired to live health and thickness data.
- Ring sizing currently assumes a default source ring size in processing.
- STL is the canonical processing format, which strips original scene structure and materials.
- There is no operation history, compare mode, or versioned processing pipeline.
- There are no local edit tools yet for partial offset, scooping, local thickening, smoothing, or deformation.
- There is no ring semantic analysis layer to distinguish band interior, outer ornament zone, gem seat, or unsafe carve regions.

## MeshLib Capability Map For This Product

The official MeshLib feature set maps well to jewelry workflows.

| Product Need | MeshLib Capability | Why It Matters |
|---|---|---|
| Fix generator output | Mesh healing, fill holes, stitch holes, tunnel fixing, degeneracy repair, self-intersection detection | Makes generated meshes printable and boolean-safe |
| Hollowing | Mesh offsetting, shell mode, weighted offset, partial offset, booleans, SDF workflows | Build hollow shells and preserve ornament where needed |
| Wall thickness control | Offset workflows, signed distance tooling, compare/distance maps, SDF | Enforce minimum castable thickness and visualize violations |
| Weight control | Volume analysis plus material density, adaptive shelling | Hit gold/silver/platinum target weight classes |
| Dimension control | Section planes, distance queries, bounding boxes, geodesic paths | Validate inner diameter, band width, head height, clearances |
| Local editing | Partial offset, free-form deformation, Laplacian deformation, extrusion, smoothing | Let users edit a generated mesh instead of regenerating it |
| Compare versions | Signed distances, compare with precision, collision/distance algorithms | Show how much a repair or hollowing step changed the form |
| Export safety | Boolean cleanup plus healing | Prevent broken STL delivery to manufacturing |

## Recommended Product Architecture

The current architecture should stay backend-first.

MeshLib is strongest when used as the authoritative geometry engine on the server side. The frontend should remain highly interactive, but heavy geometry operations should execute in the backend first, with optional future movement of selected features to WASM.

### Recommended Split

| Layer | Responsibility |
|---|---|
| Frontend | Visualization, user intent capture, tool state, inspection overlays, selection UX, operation history UI |
| Backend | Mesh normalization, repair, measurement, offsetting, booleans, thickness analysis, weight optimization, export generation |
| Storage | Original upload, normalized mesh, derived versions, analysis snapshots, preview assets |

### Why Backend-First Is The Right First Move

- MeshLib integration is already started in Python.
- Jewelry operations like adaptive hollowing and repair are compute-heavy.
- Backend processing gives reproducibility and auditability.
- We can preserve a version graph of operations.
- It avoids forcing full MeshLib WASM packaging before the product model is stable.

### Future WASM Opportunity

After the backend feature set is stable, a narrowed WASM module can be introduced for:

- previewing local deformations before commit
- instant measurements
- interactive selection and segmentation assistance
- lightweight local edits with server confirmation

The backend should still remain the final authority for export-grade output.

## Proposed End-To-End Processing Pipeline

This is the recommended manufacturing pipeline for generated jewelry meshes.

### 1. Import Normalization

Input can be `glb`, `gltf`, `obj`, or `stl`.

Pipeline actions:

- load the original mesh or scene
- detect units and normalize to millimeters
- merge scene geometry when needed
- preserve original file and preview asset
- create a normalized processing mesh artifact

Implementation note:

- Keep STL only as one artifact, not the only canonical truth.
- Add a normalized mesh artifact concept so we can retain original hierarchy and create mesh-only derivatives separately.

### 2. Health Audit

Run an automatic manufacturability audit immediately after normalization.

Checks should include:

- watertight / closed status
- hole count
- self-intersections
- degenerate and near-degenerate triangles
- disconnected shell count
- bounding-box sanity
- unit plausibility
- topology warnings for tunnels and paper-thin regions

MeshLib features relevant here:

- mesh healing
- self-intersection detection
- fill holes
- tunnel fixing
- boolean-safe topology cleanup

Output should be a structured report, not just a pass/fail boolean.

### 3. Ring Semantic Analysis

This is the most important product-specific layer and is not provided directly by MeshLib.

We should infer jewelry semantics from geometry:

- inner ring surface
- band centerline / axis
- band width profile
- head or ornament region
- likely gemstone seat
- high-relief decorative zones
- safe carve volume

This semantic layer lets us avoid naive hollowing that destroys detail.

Recommended heuristics:

- principal-axis analysis to detect ring axis
- section sweeps along the axis to estimate inner diameter and band thickness
- curvature segmentation to separate ornament from the smooth band interior
- region tagging for inner band, outer band, bezel/head, and decorative protrusions

MeshLib feature relevance:

- segmentation by curvature
- section and distance-map workflows
- shortest geodesic paths for region boundaries

### 4. Auto Repair Pass

Before any offset or boolean work, repair the mesh.

Recommended operations:

- fill small holes automatically
- stitch facing boundaries where likely accidental
- remove or repair degenerate elements
- fix self-intersections where possible
- optionally simplify tiny noisy fragments

Important product rule:

- every destructive auto-repair should produce a new version and a delta report

### 5. Manufacturing Analysis Pass

Compute the metrics required for a manufacturable jewelry decision.

Required analysis outputs:

- volume in `mm^3`
- predicted weight by material
- inner diameter
- band width range
- band thickness range
- local minimum thickness heatmap
- unsupported thin ornaments
- hollowing feasibility window
- estimated post-polish safety margin

### 6. Editing / Optimization Pass

After audit and measurement, apply user-requested modifications:

- resize ring
- hollow with fixed wall thickness
- hollow to target weight
- thicken unsafe regions
- scoop or carve interior zones
- smooth noise without destroying ornament
- repair and re-check after each operation

### 7. Final QA And Export

Before export:

- re-run health checks
- re-run thickness checks
- recompute weight
- confirm target size and tolerances
- generate final preview and final STL/GLB

## Feature Plan By Product Surface

## 1. Manufacturability Dashboard

This should become the first screen after upload.

### What The User Should See

- health score
- printability / castability score
- watertight status
- self-intersection count
- hole count
- disconnected shells count
- min wall thickness
- predicted weight for selected metal
- ring size and inner diameter estimate
- recommended actions

### Backend Work

- extend the analysis response beyond volume and bbox
- combine `analyze`, `health`, and `thickness` into a single manufacturability snapshot
- store each analysis snapshot with the model version

### Frontend Work

- wire `InspectToolPanel` to live backend queries
- present health and thickness in one decision-oriented panel
- highlight critical blockers before processing

## 2. Hollowing And Weight-Class Optimization

This is the strongest existing feature direction in the repo.

### Where MeshLib Helps

- exact offsetting
- shell generation
- weighted and partial offset
- boolean subtraction
- SDF-based robust carve workflows

### Recommended Product Modes

#### Uniform Hollow

Use for simple pieces or quick estimates.

- user specifies wall thickness
- system builds inner shell
- system validates min thickness and closure after shelling

#### Target Weight Hollow

Use for production optimization.

- user chooses material and target weight
- system binary-searches wall thickness or offset parameters
- system stops when within tolerance or when manufacturability constraints would be violated

#### Protected Detail Hollow

This should be the differentiating jewelry feature.

- preserve the snake head, gemstone seat, and decorative relief
- hollow mostly from the interior band and safe interior volumes
- keep a stronger safety margin near high-curvature decorative zones

Recommended implementation:

- segment safe and protected regions
- use weighted or partial offset instead of one global shell
- apply boolean difference only to approved carve volumes

#### Drain / Escape Hole Planning

For casting or resin workflows, hollow parts often need escape holes.

Recommended future tool:

- let the user place one or more drain holes
- validate minimum distance to ornament and visible surfaces
- boolean-subtract cylindrical or tapered escape channels

## 3. Wall Thickness Analysis And Local Thickening

This is currently the biggest technical gap.

### Product Requirement

The system must answer:

- what is the minimum local wall thickness?
- where are the violations?
- can we auto-thicken only the unsafe areas?

### MeshLib-Relevant Direction

Public MeshLib materials clearly support distance, SDF, offsetting, compare, and map-based geometry analysis. Even if the current repo uses a placeholder heuristic, the production implementation should be based on actual geometric distance queries rather than random estimates.

Recommended backend approach:

- compute signed or unsigned local thickness from opposing surfaces or from SDF/raycast-based methods
- return per-vertex or per-face scalar thickness values
- return clusters of violating regions, not just a total count

Recommended frontend approach:

- heatmap overlay on the mesh
- user-set threshold presets for jewelry, SLA, FDM, and custom
- click a hot region to zoom and inspect
- allow "auto-thicken this region" actions

Recommended product actions:

- thicken all violations to threshold
- thicken selected region
- smooth after thickening
- compare before and after

## 4. Ring Sizing And Dimension Control

The repo already exposes ring size input, but sizing needs to be manufacturing-grade.

### Current Limitation

`process.py` currently assumes a default current ring size instead of measuring it from the mesh.

### Required Upgrade

Measure the actual inner diameter from geometry.

Recommended approach:

- detect the ring axis
- slice the mesh near the inner band
- estimate the inner circular or elliptical contour
- derive inner diameter and band width from sections
- resize from measured geometry, not assumed input

### Frontend Product Features

- display measured inner diameter
- map it to standard ring sizes
- show delta to target size
- allow constrained resizing with or without ornament preservation

### Additional Dimension Controls

- band width target
- top/head height target
- maximum stone seat envelope
- underside clearance for comfort fit

## 5. Local Edit Suite For Generated Meshes

This is where the product moves from "processor" to "mesh editing software".

MeshLib has enough building blocks to support a focused local-edit suite for jewelry.

### Recommended First Local Tools

#### Partial Offset

Use when only a selected zone should be thickened or thinned.

Examples:

- strengthen the underside of a band
- reduce bulk under a head
- soften an overbuilt shoulder

#### Scoop / Carve

There is no single public "scoop" tool, but MeshLib's offset, boolean, SDF, and local deformation features are enough to build one.

Product interpretation:

- user selects an interior or underside region
- user defines a scoop depth and falloff
- system builds a carve volume or applies a local inward deformation
- system preserves minimum thickness constraints

Good jewelry uses:

- carve the underside of a heavy ring head
- reduce mass behind decorative relief
- create comfort-fit inner curvature

#### Free-Form Deformation

Use for broader shape correction.

Examples:

- adjust silhouette after generator artifacts
- refine band flow
- improve symmetry

#### Laplacian Deformation

Use for local edits that preserve surrounding curvature.

Examples:

- pull a protrusion inward
- nudge a head region without wrecking adjacent scale detail

#### Smoothing And Simplification

Use selectively.

Examples:

- reduce noise from generative meshes
- smooth the inner band only
- decimate dense regions before expensive processing

Important product rule:

- smoothing must be region-limited and versioned
- never apply blind global simplification to jewelry ornament

## 6. Auto Repair And "Make It Manufacturable"

This should become a one-click product promise.

### Target User Experience

User uploads a generative mesh and clicks:

- Analyze
- Auto Repair
- Make Manufacturable

### What "Make Manufacturable" Should Do

- repair mesh health blockers
- normalize size and units
- compute thickness and weight
- apply recommended hollowing mode
- keep protected detail zones
- verify final closure and thickness
- produce a manufacturability report

### Why This Is Valuable

It turns the app from a visualization layer into a post-generation manufacturing copilot.

## 7. Compare And Audit Tools

Every geometry operation should be measurable.

Recommended compare features:

- original vs repaired
- repaired vs hollowed
- before vs after thickness fix
- before vs after resize

Recommended data:

- signed distance map between versions
- volume delta
- weight delta
- bounding-box delta
- min-thickness delta

This is especially important in jewelry because small geometric changes can change:

- seat integrity
- polish allowance
- comfort fit
- casting success
- final metal weight

## Specific Repo Extensions

## Backend Additions

### 1. Create A Manufacturability Snapshot Service

Add a service that aggregates:

- base analysis
- health analysis
- thickness analysis
- measured ring dimensions
- weight table by material
- recommended actions

Suggested endpoint:

- `GET /api/manufacturability/{model_id}`

### 2. Replace Placeholder Thickness Logic

Current `services/thickness.py` should be replaced with real geometry-driven analysis.

Required outputs:

- `min_thickness`
- `max_thickness`
- `avg_thickness`
- `violations`
- `region_clusters`
- `vertex_scalars` or `face_scalars`

### 3. Introduce Versioned Mesh Artifacts

Instead of overwriting the working result conceptually, store:

- original upload
- normalized mesh
- repaired mesh
- hollowed mesh
- thickened mesh
- exported mesh

Suggested model shape:

- `model_id`
- `parent_model_id`
- `operation`
- `parameters`
- `analysis_snapshot`
- `artifact_paths`

### 4. Add Region-Aware Operations

Introduce new endpoints:

- `POST /api/repair/{model_id}`
- `POST /api/hollow/{model_id}`
- `POST /api/thicken/{model_id}`
- `POST /api/scoop/{model_id}`
- `POST /api/resize/{model_id}`
- `POST /api/smooth/{model_id}`
- `POST /api/compare`

The current `/api/process` endpoint can remain as a convenience macro endpoint.

### 5. Add Ring Measurement Service

Needed outputs:

- ring axis
- measured inner diameter
- estimated current ring size
- band width min/max
- band thickness min/max
- top head height

### 6. Add Async Job Support

Many of these operations will become too heavy for synchronous request handling.

Recommended future shape:

- submit job
- poll job status
- stream progress events
- attach operation logs to the resulting version

## Frontend Additions

### 1. Turn Inspect Into A Real Control Surface

`InspectToolPanel` should show live data from backend endpoints:

- health
- thickness
- manufacturability score
- repair recommendations

### 2. Add Heatmap Rendering In `ModelViewer`

The viewer currently supports clipping well. Extend it with:

- per-vertex thickness heatmap
- violation overlay
- selected region highlighting
- before/after compare toggle

### 3. Add Version History

Mesh editing is iterative. The UI should expose:

- operation history
- branch or duplicate version
- compare current against previous
- restore previous state

### 4. Add A Manufacturing Dashboard

Recommended cards:

- material weight table
- ring size fit
- wall-thickness health
- repair blockers
- export readiness

### 5. Add Guided Workflows

Recommended user flows:

- `Quick Repair`
- `Fit To Size`
- `Reduce Weight`
- `Fix Thin Areas`
- `Prepare For Casting`

These should orchestrate backend operations rather than expose low-level parameters only.

## Product Strategy: Best Next Features

The product should not try to become Blender in the browser. It should become the best post-generation jewelry mesh preparation tool.

### Highest-Value Feature Sequence

1. Real manufacturability dashboard
2. Real thickness analysis
3. Measured ring sizing
4. Better hollowing with protected-detail logic
5. Compare and history
6. Local thickening and scoop tools
7. Guided one-click "make manufacturable" flow

## Proposed Phase Plan

## Phase 1: Harden Existing Foundations

Target:

- make current features trustworthy

Deliver:

- live inspect panel wiring
- real health + thickness snapshot endpoint
- measured ring sizing
- improved repair reporting
- better hollowing validation after offset and boolean

## Phase 2: Manufacturing Intelligence

Target:

- let users decide based on actual production metrics

Deliver:

- thickness heatmap
- weight table by material
- violation clustering
- export readiness scoring
- suggested fixes

## Phase 3: Jewelry-Specific Optimization

Target:

- optimize for weight without damaging design intent

Deliver:

- protected-detail hollowing
- partial offset
- local thickening
- scoop/carve tools
- comfort-fit interior shaping

## Phase 4: Editing Platform

Target:

- make the product feel like a focused mesh editor

Deliver:

- operation history
- compare mode
- local deformation tools
- selection-driven workflows
- async long-running jobs

## Practical Notes For The Current Codebase

### Keep Trimesh Where It Is Useful

Trimesh is still useful for:

- file loading and conversion
- lightweight analysis
- preview asset generation

But production geometry authority should move more heavily toward MeshLib for:

- repair
- hollowing
- booleans
- thickness
- local edits

### Do Not Depend On STL Alone

STL is fine for manufacturing export and some internal processing, but it discards:

- scene hierarchy
- materials
- texture relationships
- named subcomponents

Recommendation:

- preserve original upload as source
- generate normalized mesh artifacts for processing
- generate STL only when needed for manufacturing or mesh-only stages

### Make Every Operation Re-Analyzable

Every processing result should immediately trigger:

- health analysis
- weight recomputation
- dimension recomputation
- thickness recomputation

That closes the loop and keeps the UI trustworthy.

## Risks And Mitigations

| Risk | Why It Matters | Mitigation |
|---|---|---|
| Generator output is topologically chaotic | Offsetting and booleans can fail | Always run repair and health audit first |
| Naive hollowing destroys detail | Jewelry head and ornament regions are sensitive | Add protected-detail segmentation and weighted/partial offset workflows |
| Thickness analysis is expensive | Real-time UX can degrade | Use async jobs and cached scalar fields |
| Ring axis detection can be wrong | Dimension control becomes unreliable | Add user-confirmable axis and section previews |
| Global smoothing ruins ornament | Jewelry detail is high-value | Restrict smoothing to tagged regions and compare before/after |
| Frontend-only editing becomes too complex too early | Slows delivery | Keep heavy geometry server-side first |

## Recommended Immediate Next Engineering Tasks

1. Replace the current thickness placeholder with a real geometry-based implementation.
2. Add a single manufacturability snapshot endpoint and wire it into `InspectToolPanel`.
3. Replace assumed current ring size with measured inner diameter and estimated ring size.
4. Refactor processing to produce versioned artifacts and store operation metadata.
5. Extend hollowing from uniform shelling to protected-detail hollowing.
6. Add compare and history before introducing more aggressive local edit tools.

## Conclusion

MeshLib is not just a library that can "repair a mesh". It is the geometry core that can turn this repo into a jewelry-specific post-generation manufacturing platform.

The right product direction is:

- keep the frontend as a highly visual editing and inspection workspace
- keep MeshLib-backed geometry authority in the backend
- add a manufacturability intelligence layer specific to jewelry
- evolve from global processing tools to region-aware editing tools

If executed well, the resulting product gives users a better workflow than relying on a generator alone:

- generate once
- inspect deeply
- repair automatically
- optimize for size, weight, and thickness
- locally edit problem areas
- export a production-ready version with confidence

## Source References

- MeshLib feature index: https://meshlib.io/feature/
- Mesh healing: https://meshlib.io/feature/mesh-healing/
- Mesh offsetting: https://meshlib.io/feature/precision-mesh-offsetting-with-meshlib/
- Fill holes: https://meshlib.io/feature/how-to-fill-holes-in-meshes-with-the-meshlib-library/
- 3D boolean operations: https://meshlib.io/feature/3d-boolean-operations/
- Mesh to SDF: https://meshlib.io/feature/mesh-to-sdf/
- Documentation index: https://meshlib.io/documentation/index.html
- Code samples index: https://meshlib.io/documentation/Examples.html
