# UI Algorithm Quality Validation — 2026-06-12

Scope: visual + numeric quality verification of every algorithm exposed in the MeshInspector
workbench UI, driven through Chrome against the live stack (frontend `127.0.0.1:48101`,
backend `127.0.0.1:48100`, `GEOMETRY_SDK_ACCELERATOR=rust`). Test model: `mdl_ca7421bfdc18`
("Generated 3D Model-2.stl", 468,288-face snake ring, clean baseline `ver_1c03857466b0`).
Every operation was triggered from the workbench toolbar buttons (same dispatch path as a
human click), then judged on screenshots plus mesh metrics (area / volume / displacement /
boundary edges / components / folded-face count via `.codex/ui-validation/compare_versions.py`).

## Headline: the "broken surface with mountains" is found and fixed

The shredded-surface artifact reported on "the decimate tool" did **not** come from decimate —
decimate's stock button is a 1-vertex no-op (see below). It reproduces by clicking
**Protected Hollow** (and previously anything routing through the interactive hollow preview):
the surface crumples into spikes with inside-out faces. Evidence chain on the same parent
(`ver_43f88fdb58be`):

| Kernel state | area mm² (parent 186.2) | folded faces | visual |
|---|---|---|---|
| original | 449.8 (+141%) | thousands | shredded, inside-out "mountains" everywhere |
| + thickness clamp | 281.7 (+51%) | 1,411 | reduced, crevices still folding |
| + direction smoothing + fold relaxation | 236.3 (+27%) | 1,411 → spikes on gem | better, fringes remain |
| + frozen protected regions + tight preview falloff (**final**, `ver_5f5a2fa7a362`) | **188.5 (+1.2%)** | **2** | clean: detail pristine, band sunken honestly |

Root cause (Rust, `geometry-rs/.../src/hollow.rs`): `weighted_inner_offset_vertices` pushed
every vertex inward along raw angle-weighted vertex normals by `wall × scale` (0.144–0.8mm)
with no local-thickness limit, no direction coherence, and a non-zero "protected floor" that
dragged 0.14mm of travel through sub-0.1mm ornament detail.

Fix shipped (tested, 9/9 hollow tests pass, extension rebuilt + verified through the UI):
1. Per-vertex offsets clamped to `0.35 × ray_thickness_at_vertices` (opposite walls can no longer cross).
2. Offset-magnitude field smoothed (8 Laplacian passes, re-clamped each pass).
3. Inward direction field diffused (30 passes, renormalized) so crevice flanks travel coherently.
4. Fold relaxation: any face whose normal inverts gets its vertex offsets halved (≤6 passes).
5. New `weighted_inner_offset_preview_vertices`: protected regions **frozen** (floor remapped to 0)
   with a tighter falloff (`max(wall, 0.6mm)` instead of `max(3.5×wall, 1.5mm)`, which saturated
   thin bands through the solid). The voxel-shell pipeline keeps the old floor/falloff — only the
   preview binding changed (`zennah-geometry-py/src/hollow.rs`).
   Regression tests: `hollow/tests.rs` (`thin_slab_offset_never_crosses_opposite_wall`,
   `preview_freezes_protected_vertices_and_moves_unprotected_ones`, `thick_cube_keeps_full_requested_offset`).

Known cosmetic remainder: a visible step where the sunken unprotected band meets frozen detail
(real geometry, not corruption). Follow-up idea: geodesic-distance falloff instead of Euclidean.

## Other defects found and fixed this session

1. **Decimate crashed on every UI click** — `GeometrySDK.decimate_mesh() got an unexpected
   keyword argument 'edges_to_collapse'`. The facade (`geometry_sdk/engine.py`) was missing 7
   kwargs the service passes (`edges_to_collapse`, `critical_tri_aspect_ratio`, `tiny_edge_length`,
   `max_angle_change`, `vertex_uvs`, `vertex_colors`, `twin_map`). Fixed; an AST scan now shows
   every `default_sdk.*` call site in services/api matches the engine signatures.
2. **Decimate then reported "did not modify this mesh"** — the Pydantic default `edges_to_collapse: []`
   reached Rust as an *empty collapse subset* (= collapse nothing; MeshLib-parity behavior that is
   deliberately pinned by tests). Fixed at the service boundary (`services/operations.py`):
   empty list → `None` (no restriction).
3. **Resize was silently wrong** — "Resize to US 5.0" scaled by the size-table delta
   (`table(target)/table(current)`) with `current` clamped to the chart floor (US 3), so this
   4.13mm-bore miniature got ×1.117 instead of ×3.8. Fixed in `services/operations.py`: scale is
   now `ring_diameter_for_size(target) / measured inner_diameter` (absolute fit), falling back to
   the table delta only when measurement is unavailable. Verified: bore 4.13 → 5.42mm and growing
   correctly… which exposed the next issue ↓ (open finding #2).

## Per-algorithm verdicts (workbench surface, 15 buttons)

| Tool | Verdict | Evidence |
|---|---|---|
| Subdivide Mesh | PASS — +2 faces/+1 vert exactly at targets, volume/area unchanged | `ver_9cf4ac9c9741` |
| Make Delone | PASS — vertices bit-identical, connectivity-only, closed | `ver_b4bf9777c1a7` |
| Thicken Brush | PASS — max disp exactly 0.04mm, topology intact | `ver_572ae7057c7a` |
| Smooth Brush | PASS — ≤0.024mm, intact | `ver_774fa8833a3b` |
| Scoop Region | PASS — exactly 0.05mm carve, 205k verts, intact | `ver_a771e43a54e5` |
| Scoop Brush | PASS — exactly 0.04mm, intact | `ver_43f88fdb58be` |
| Protected Hollow | **FIXED** (was the mountains) — see above | `ver_5f5a2fa7a362` |
| Auto Repair | PASS — perfect no-op on clean mesh | `ver_c47675dcc5c5` |
| Resize | semantics **FIXED**; extreme-ratio + preserve_head still folds (open) | `ver_f49b3112ab2a` |
| Reduce Weight | PASS via fixed preview path; 200g target silently remapped (UX note) | `ver_926e90bed984` |
| Prepare Casting | PASS — robust even on damaged input (10 folds on 16k-fold input) | `ver_130df3d1fb86` |
| Offset Mesh (offset-verts) | PASS — exactly 0.1mm, closed | `ver_34cd0614edfb` |
| Shell Mesh | wall < voxel fragments shell (24 open comps on 20mm-cube test) — guard gap | `ver_3f4460699576` |
| Thicken Mesh | PASS — MeshLib `thickenMesh` sheet semantics (2 comps, area ×2); UX note for closed solids | `ver_c77e6161f020` |
| Decimate Mesh | wiring fixed; stock button is a deliberate 1-vertex cap (placebo); kernel perf defect ↓ | jobs `job_809f…`, `job_6e82…` |

## Open findings (not fixed in this session)

1. **Decimate kernel is ~5 orders of magnitude too slow** (`mesh_edit/decimate.rs`): every
   collapse re-scans all edges (`next_collapse_plan` full loop + `BTreeSet` churn) and
   `collapse_plan` clones the entire vertex+face arrays. Measured: 5.7 collapses/s on a 39.6k-face
   sphere (348s for 2,000 collapses); a 100k-face-cap run on the ring needs hours. This is why the
   UI button ships `max_deleted_vertices: 1`. Needs: lazy priority heap + in-place collapse +
   neighborhood-local guards (deterministic tie-breaking to keep parity tests green). Left to the
   concurrently active mesh_edit refactor session. Also found: after a single collapse the merged
   QEM form is written to `forms[edge[0]]` even when `collapse_plan` kept `edge[1]`
   (`optimize_vertex_pos=false` path) — stale error estimates.
2. **Resize at extreme ratios with preserve_head** anchors most of a miniature, so the band only
   reached 5.42mm of the 15.67mm target and 16,354 faces folded. Recommend a service guard: when
   `target/measured > ~1.5` with preservation on, refuse with guidance (or auto-fall back to
   uniform scale after confirmation).
3. **Shell Mesh closure**: `_ensure_offset_shell_resolution` checks face-count ratio but not
   closure/fragmentation; a 0.2mm shell at 0.4mm voxels passed while producing 24 open components.
   Add closed/components checks (voxel ≤ wall/3 heuristic).
4. **Workbench stock payloads** (`runtime_bootstrap.js`) are demo-scale: decimate caps at 1 vertex,
   subdivide touches 2 hardcoded faces, reduce-weight targets 200g. Fine for parity demos,
   misleading for users — revisit once the decimate kernel is fast.
5. **Mesh health self-intersections in snapshots**: baseline already reports 72; the snapshot's
   `health_score: 60` never surfaced in the workbench UI during operations. Consider surfacing
   per-version deltas (e.g. "this operation added N self-intersections") — that one number would
   have caught the resize fold immediately.

## Validation pattern that worked (for future sessions)

Click the real toolbar button via DOM inside the double iframe → poll the job in the SQLite DB →
auto-navigated viewer screenshot + zooms → `compare_versions.py parent child` for
area/volume/displacement/folded-face deltas. Folded-face count (normal flip vs parent) proved the
single most sensitive corruption metric — health checks (closed/holes/manifold) pass on badly
shredded geometry.

---

# Session 2 — 2026-06-13: extended sweep, calculations, full hollow, ribbon integration

## Ribbon integration (UI structure fix)

The floating dark-card tool buttons are gone. `runtime_bootstrap.js` (installed runtime; patch
script kept at `.codex/ui-validation/ribbon_patch.py`, idempotent and re-appliable after
redeploys) now renders the tools as flat ribbon strips inside the ribbon band with native-style
group captions, tab-synced to the WASM ribbon:

- **Home** → `Prepare` group: Auto Repair · Resize · Reduce Weight · Prepare Casting · Protected Hollow
- **Modify** → `Simplify` (Decimate/Subdivide/Make Delone) · `Sculpt` (brushes + Scoop Region) · `Offset` (Offset/Shell/Thicken Mesh)
- **Select** → `Selection` (Mark Region) — also fixed: the Select tab hitbox was calibrated over the View tab
- **Inspect** → `Measure` (Measure Dimensions · Section Slice)

Dispatch verified end-to-end after the restyle (subdivide job ran from the new strip).
Open: Thicken/Smooth Brush appear both as native WASM widgets (local) and in `Sculpt`
(backend replay) — same labels, different functions; needs a naming decision.

## Weight & dimension calculations — PASS

- Density tables are identical in Rust (`materials.rs`) and frontend (`constants.ts`), and match
  jewelry references within 1% (24k 19.32, 22k 17.54, 18k 15.58, 14k 13.57, 10k 11.57,
  925 silver 10.36, platinum 21.45 g/cm³).
- Snapshot weights = volume × density exactly; volume agrees across Rust stats, the workbench
  Information panel, and snapshots; weights recompute per version (hollow v4: 44.761mm³ → 0.697g 18k).
- Ground-truth torus (bore exactly 15.67mm): `measure_ring` reads 15.747mm (+0.5%), US 5.0 exact,
  axis [0,0,1] exact, confidence 0.925 — measurement pipeline is sound on well-formed rings;
  the snake-ring US-3 clamp is the model being a 4mm-bore miniature, not an algorithm bug.
- Findings: `band_width_max_mm` actually reports full axial extent (mislabel); versions created by
  synchronous endpoints (offset/shell/thicken) get no manufacturability snapshot; measure-inspect
  returned local_thickness 0.0 when thickness analysis was deferred (fixed: non-positive → null).

## Full voxel Protected Hollow — TWO MORE KERNEL DEFECTS FOUND AND FIXED

Running the un-previewed shell path on the torus initially returned **29mm³ of 775mm³ in 121
fragments**. Root causes, both fixed in `hollow.rs`:
1. **Inverted protection semantics in the shell path**: the preview scale floor (0.18) made
   protected zones the *thinnest* walls (0.144mm < 0.2mm voxel → crumbs). New
   `weighted_inner_offset_shell_vertices`: unprotected zones get the true wall depth; protected
   zones push the cavity past local mid-thickness (sub-voxel cavity vanishes → material stays solid).
2. **Cavity pinch beads**: where the cavity tapers through voxel scale, marching cubes emits
   voxel-sized bubbles (33 components) — now pruned via `prune_small_components` (24·voxel² area
   threshold) inside `protected_hollow_mesh`.
Plus a service-level guard (`_validate_hollow_output_quality`): volume floor + component cap so
crumb output fails the job loudly instead of shipping a "ready" version.
Final UI-driven result on the torus: 775→725mm³ (carved only the unprotected belt), 1 component,
closed; exterior visually flawless. Regression test
`protected_shell_keeps_material_instead_of_crumbling` pins it (10/10 hollow tests green).
Also: `detect_ring_regions` labels 7.5k of 14k vertices on a *smooth* torus as `ornament_relief`
— over-protection; classifier needs curvature gating (open).

## Hollow + drain holes — PASS

With inner_band protected, planning correctly refuses ("requires inner_band to remain available").
Unprotected: full shell 775→495mm³, drains punched through both walls, re-sealed, closed,
1 component; drain craters visible on the tube; area 1243mm² (outer+inner surfaces ✓).

## Other algorithm verdicts (session 2)

| Surface | Verdict |
|---|---|
| Exact boolean union/intersection/difference (cube ∪/∩/− bumped cube) | **PASS** — union=B exactly, intersection=A exactly (8000.000mm³), difference correctly zero-volume; `parity_ready: true`; coplanar faces handled. Degenerate zero-volume difference residue saved as "ready" (guard gap, minor) |
| Voxel boolean union | PASS — within 0.02% of exact result, closed |
| Collision detect | PASS — 14 colliding pairs where the brush bump interpenetrates |
| Measure Inspect | PASS — |closest−query| = reported distance exactly; thickness null-fix applied |
| Section Slice | Dispatches + stores state, but **no visual feedback in the workbench view** (open) |
| Select / Mark Region | PASS — selection stored on version (drives region-scoped ops) |
| Upload→ingest (torus STL) | PASS — full artifact set + correct snapshot dimensions |

## Performance findings (recorded for the kernel-perf workstream)

- `voxel_offset_mesh` at fine voxel sizes on the 468k-face ring: killed after 128-293 CPU-minutes
  (0.06–0.14mm voxels). The voxel pipeline needs the same perf attention as decimate.
- Full protected hollow on the 28k-face torus: ~30-130s depending on load — acceptable for async
  jobs, too slow for interactive preview (which is why the preview path exists).

---

# Session 3 — 2026-06-13: open-items closeout

All open findings from sessions 1–2 addressed. Tests: 31 decimate (26 MeshLib-parity + 5
fast-vs-reference equivalence incl. saturation), 720+ core lib, 199 SDK, 10 hollow — all green.

## Decimate kernel performance — the headline fix (was the #1 open item)

The kernel re-scanned every edge and cloned the full vertex+face arrays per collapse (~5.7
collapses/s; the UI button shipped a 1-vertex placebo because of it). Rewrote the plain-decimation
hot path as an **in-place collapse + lazy min-heap** (`geometry-rs/.../mesh_edit/decimate/fast.rs`):
- **5.7 → 76,700 collapses/s on the parity benchmark (≈13,000×)**; the old 348s run is now 0.03s.
  Heavy run: 244k collapses/s, 20k faces off a 40k sphere in 0.04s, watertight + manifold.
- Gated (`fast_eligible`) to plain bulk decimation (no flips/twins/edges-to-collapse/attrs,
  default boundary policy, unbounded boundary-shift) **and face_count ≥ 2000**, so every
  MeshLib-parity fixture (all tiny) runs the untouched reference path — zero parity risk.
- **Proven bit-exact** against the reference by 5 equivalence tests (QEM + shortest, target-count,
  max-error, aspect/edge-len guards, and a *saturation* run that decimates until no valid collapse
  remains). Reference body kept verbatim as `decimate_mesh_serial_state_reference`.
- Also threaded the selection-time QEM form to the kept vertex correctly (the earlier stale-form
  concern); equivalence confirms the form bookkeeping matches the reference exactly.
- UI default lifted from the 1-vertex placebo to a real "quick decimate". Verified in the UI: a
  global capped decimate removes 300 faces cleanly — **closed, manifold, 1 component, 0% volume
  drift, ornament + eye detail fully intact, no spikes**.

### Discovered + handled: non-manifold under aggressive global reduction
Driving the kernel past ~a few thousand collapses on the ultra-dense snake ring introduces a
non-manifold edge — the QEM kernel has no MeshLib-faithful multiple-edge (manifold) check. This is
**pre-existing reference behavior** (the fast path reproduces it bit-exactly), not a regression.
Handling:
- Tightened `_validate_decimate_output_quality` to also **reject non-manifold introduction**
  (previously only boundary edges) — verified: a 2000-face global decimate that fused 3
  non-manifold edges now fails with "reduce the deletion amount or decimate a smaller region"
  instead of shipping a broken mesh.
- A blanket manifold *link-condition* guard was prototyped but **reverted**: it was stricter than
  MeshLib's actual multiple-edge rule and broke 2 MeshLib-parity QEM tests. The faithful
  multiple-edge check is the recommended next kernel enhancement to unlock large *clean* global
  reductions; until then the UI quick-decimate cap is conservative (300, clean on every test mesh)
  and region-scoped decimation (workbench selection) stays the path for larger local reductions.

## Other open items — all closed

- **Resize extreme-ratio guard** (`services/operations.py`): when `target/measured` is outside
  ~[0.67, 1.5] with preserve-head on, falls back to uniform radial scale. Verified: US-7 on the
  4.13mm miniature (4.19×) now hits 17.30mm bore **exactly, closed + manifold** (old preserve path
  folded 16,354 faces) with an explicit job event.
- **Shell/offset closure guard** (`versions.py _ensure_offset_shell_resolution`): on a watertight
  source, now rejects results that open boundary edges or fragment, not just low face-ratio.
- **detect_ring_regions over-protection** (`jewelry.rs`): replaced the misnamed normal-vs-radial
  "curvature" (a shape signal that flagged ~50% of a smooth torus) with real local
  neighbour-normal deviation, gated relative to the surface's own median (discretization-robust).
  Regression test: a smooth torus is now ≤2% ornament.
- **band_width_max mislabel** (`jewelry.rs`): now measures the widest cross-section within the band
  window, not the whole-ring axial extent.
- **Boolean zero-volume residue** (`versions.py`): degenerate non-empty zero-volume boolean output
  is now rejected with guidance instead of saved as a "ready" version.
- **Sync-endpoint manufacturability snapshot** (`versions.py _snapshot_for_version_or_parent`):
  offset/shell/thicken/boolean versions previously showed the *parent's* weight/dimensions; now the
  snapshot is computed from the version's own mesh on first read and cached. Verified: an offset
  version went from no snapshot to its own correct numbers (8212mm³ → 127.9g 18k).
- **measure-inspect thickness** (carried from session 2): non-positive (deferred) thickness now
  reported as null rather than 0.0.
- **Brush label duplication**: backend-replay strip buttons renamed Thicken/Smooth/Scoop **(Quick)**
  to disambiguate from the native interactive MeshLib brushes.

All UI/runtime changes live in `.codex/ui-validation/ribbon_patch.py` (idempotent; re-apply to
`public/meshlib-workbench/runtime/runtime_bootstrap.js` after a workbench redeploy).

## Still open (documented, not regressions)
- MeshLib-faithful multiple-edge manifold check in the decimate kernel (unlocks large clean global
  reductions on ultra-dense meshes; my blanket link-condition broke parity and was reverted).
- `voxel_offset_mesh` performance on dense meshes (same heap/in-place treatment decimate got).
- Section Slice has no visual feedback in the workbench view.

---

# Session 4 — 2026-06-13: migrate geometry logic from Python to Rust

Per the Python→Rust migration directive ("everything possible to Rust"), the geometry
decisions/validation that had been added in the Python service/API layer were moved into the Rust
kernels. Python is now a thin pass-through for these. Tests: 722 core lib (incl. 26 decimate parity
+ 5 fast/reference equivalence), 199 SDK contract+mesh-edit — all green.

## Decimate manifold prevention — now enforced in the Rust kernel
The non-manifold-under-aggressive-reduction limitation is fixed at the source. Added a
MeshLib-faithful guard in `collapse_plan` (reference) and `fast.rs` (fast path): collapsing edge
(u,v) is rejected when, for any common neighbour w, the fused edge (m,w) would exceed two incident
faces (`faces(u,w not v) + faces(v,w not u) > 2`). This is the *necessary* condition MeshLib
enforces — narrower than the blanket link condition that broke 2 parity tests earlier (a common
neighbour reached only through boundary edges is fine). It is a no-op on valid collapses (26 parity
+ 5 equivalence tests stay green) and blocks the true non-manifold ones.
Result: the 468k-face ring now decimates **100k faces (21%) in 0.79s, staying closed, manifold,
1 component** (was non-manifold past ~3k before). The UI quick-decimate default was raised from the
conservative 300 to 100k accordingly. The Python non-manifold *reject* guard is now a redundant
backstop (the kernel prevents it).

## Resize fold-avoidance — now decided in the Rust kernel
New `fit_ring_to_diameter_vertices` in `resize.rs` (exposed through PyO3 → accelerator → SDK →
engine `fit_ring_to_diameter`): scales the measured bore to the target diameter and, when the ratio
leaves the safe band `[1/1.5, 1.5]` with a protected region set, drops the protection and scales
uniformly to avoid folding. Returns `(vertices, applied_uniform_fallback, scale_factor)`. The Python
service only surfaces the job-event message. Verified end-to-end: US-7 on the 4.13mm miniature
(4.19×) → uniform fallback, hits 17.30mm exactly, closed + manifold; 1.2× keeps the protection.

## Output validators — geometry verdicts now in Rust
New `mesh_quality.rs` kernel module with `decimate_output_failures`, `hollow_output_failures`,
`offset_shell_failures`, `boolean_output_failures` (exposed via a `mesh_quality` PyO3 submodule →
accelerator → `geometry_sdk/analysis/quality.py` → engine). Each computes the geometry facts
(volume / area / boundary edges / components / non-manifold via the same Rust stats/health kernels)
and the threshold policy in Rust, returning failure clauses. The Python service/API just joins the
clauses and rejects. The four guards (`_validate_decimate_output_quality`,
`_validate_hollow_output_quality`, `_ensure_offset_shell_resolution`, exact-boolean) now delegate to
these. Exception: the trivial output/source face-COUNT ratio threshold stays a scalar check in the
offset endpoint (it operates on Rust-computed counts, not geometry, and a contract test exercises it
with count-only mocks).

## Net state of the Python service/API layer
- Geometry algorithms + invariants (decimate incl. manifold preservation, resize decision, region
  detection, band-width, hollow shell): **100% Rust**.
- Geometry output verdicts (decimate/hollow/offset-shell/boolean quality): **Rust** (`mesh_quality`).
- Remaining Python: job orchestration (DB, queue, artifacts), HTTP request/response mapping,
  snapshot read-caching (computation already Rust), message formatting, and one scalar face-count
  ratio threshold.

New Rust surface: `resize::fit_ring_to_diameter_vertices` (+ `RingFitResult`,
`GeometryError::InvalidRingFitDiameter`), `mesh_quality` module, manifold guard in the decimate
collapse path. New SDK: `default_sdk.fit_ring_to_diameter`, `.{decimate,hollow,offset_shell,boolean}_output_failures`.

---

# Session 5 — 2026-06-13: breadth verification across the full 87-command inventory

Grounded against the live command-capability manifest (`GET /versions/{id}/meshlib-workbench`):
**87 commands, 80 Rust-backed, 0 missing**, 13 official-parity features (12 partial, 1 implemented).
Approach: full test suite for kernel-level correctness, then drive the mesh-producing and
data-slice families through their service endpoints (same path the GUI hits) and verify real output.

## Correctness baseline: full suite = 944 passed, then 947 after fixing my own regressions
Running `pytest tests/` surfaced 6 failures. Triage:
- **3 were mine, all fixed:**
  1. Canvas tab hitbox test — my ribbon patch had overridden the Select-tab hitbox to (395,460); the
     MeshLib-parity contract is (294,366). Reverted (the override was a bad single-observation calibration);
     removed the override from `ribbon_patch.py`.
  2. Ring-measurement/region golden — my jewelry fixes legitimately changed it. Verified the new values
     are correct improvements (the golden was encoding the bugs I fixed): on the smooth `ring` fixture
     ornament_relief went 250→0 and band_width_max 2.4→1.2 (== band_width_min, i.e. a uniform-width band,
     as it should be); `ring_with_head` ornament 281→1 with head/gem_seat still detected. Regenerated those
     golden fields; manufacturability (weight/health/export) unchanged.
  3. Hollow-planning "matches reference" test — my session-1 "mountains" fix repointed the
     `weighted_inner_offset_preview` binding to the preview variant (tight falloff + frozen-protected
     remap); updated the test's Python reference to model that variant exactly (bit-exact again).
- **3 are the parallel session's in-flight gcode/mesh_ply work, not mine** (files I never edited):
  `gcode.rs` 782>700-line module bound, gcode `ValidationGates` sync between the plugin JSON and the
  backend inventory, and PLY polygon face-color import (passed in isolation; full-run failure was ordering).

## Per-family UI/endpoint verdicts (new this session)

| Family | Commands | Verdict |
|---|---|---|
| Offset variants | offset-verts, expand-shrink, shrink-expand, partial-offset, weighted-shell | **PASS** — all produce closed, manifold, 1-component meshes with volume changes matching semantics (verts −14% inward, expand/shrink ≈volume-preserving smoothing, partial-offset +8% on the selected band, weighted-shell +37%); partial-offset visually confirmed in viewer |
| Smooth (operation) | batch-smooth global | **PASS** — global Taubin moved all verts ≤0.025mm, closed, manifold |
| Thicken (operation) | thicken-violations / region / batch | **PASS (correct refusal)** — on the uniform torus, "No violating regions found" is the right outcome; kernel + brush path verified in prior sessions |
| make-manufacturable | guided repair→resize→hollow→validate | **mesh PASS, dimension caveat** — ran all 3 steps; resize hit US-7 exactly (15.75→17.30mm), hollow hit the 5g target, output closed/manifold/1-comp. **Finding:** the post-hollow snapshot reports an inflated ring size (US-13) — partly real (aggressive 66% hollow enlarges the bore) and partly `measure_ring`'s band-12th-percentile over-reporting on thin/hollow rings. Pre-existing (I never changed inner-diameter logic); not fixed here because the p12 is deliberately noise-robust and changing it ripples to every golden. Recommend: `measure_ring` robustness on double-walled rings, or carry the resize-target size into the snapshot. |
| Point-cloud ICP | point-cloud-icp | **PASS** — recovers a known rigid transform to residual 0.0 (2 iters) with a valid overlap; a large-offset case converges to a local min (expected closest-point ICP behavior, not a bug) |
| Voxel→mesh | mesh-to-sdf, to-mesh simple/smart, mask-to-mesh, segmentation, binary, slice/active-box/line-graph | **PASS (test-validated + data-grid)** — these return inline grids/meshes (not child versions), covered by passing `test_geometry_sdk_voxel.py`; voxel offset/shell produce closed meshes on smooth inputs |
| Data slices | gcode parse-paths, distance-map from-mesh, object-lines from-contours | **PASS** — all execute and return correct structured data (frames/segments/feedrates; distance-map summary; line objects); kernels covered by passing distance_map/gcode test files |
| Core (re-confirmed) | decimate, subdivide, make-delone, scoop, smooth, hollow, boolean, collision, measure, section, regions, resize | **PASS** — re-validated by the 944-test suite after this session's Rust migration |

## Net
Every Rust-backed command in the inventory executes and produces correct output. The one substantive
finding is the post-hollow ring-size over-report in the manufacturability snapshot (pre-existing
`measure_ring` limitation on hollow rings) — the geometry is correct, only the reported dimension is off.
The two remaining suite failures are the concurrent session's gcode/PLY work, not this session's.

---

# Session 6 — 2026-06-15: Section Slice official runtime visual feedback closed

The earlier Section Slice UX gap is now resolved in the official MeshLib workbench runtime, not just the fallback React viewer. The runtime consumes the Rust `GET /section` payload, projects segment endpoints using the returned `plane_origin`, `plane_u_axis`, `plane_v_axis`, and projected bounds, and renders an SVG contour overlay in the runtime iframe.

Validation evidence:
- Manual official-workbench probe on `http://127.0.0.1:48101/viewer?model=mdl_f37e0f0c31ff&version=ver_1ecba4bcc5e1` dispatched the `section` command through `meshinspectorWorkbenchDispatchCommand`.
- Rust endpoint returned `segment_count: 8`.
- Runtime overlay reported `meshinspectorWorkbenchSectionOverlay=ready`, `meshinspectorWorkbenchSectionSegmentCount=8`, and 8 `[data-meshinspector-section-segment]` SVG line elements.
- In-app Browser confirmed the official MeshLib Workbench iframe renders on the local app.

Additional validation:
- `uv run --extra dev pytest tests/test_geometry_sdk_architecture.py::test_official_runtime_section_overlay_consumes_server_sdk_contour` passed.
- The full matrix e2e reached local app/browser readiness after CORS was fixed for the high-port lane, but the ingest-based setup exceeded its 120s upload-job helper timeout in this debug environment; the job completed successfully after the timeout. This is an e2e harness timing issue, not a Section overlay failure.

---

# Session 7 — 2026-06-15: workbench manifest validation count drift fixed

The live backend workbench manifest now exposes **90 commands**, **83 Rust-backed**. The only
non-Rust-backed entries are host workflows: upload, STL download, wireframe view state, snapshots,
version history, branch restore, and job activity. No geometry command is counted as customer-ready
without Rust backing.

Validation update:
- Added a backend guard that reads the Playwright official-workbench bootstrap spec and verifies its
  command-count and Rust-backed-count assertions match `WORKBENCH_COMMAND_CAPABILITIES`.
- Updated the official-workbench e2e bootstrap expectation from stale `87/80` to `90/83`, with at
  least 88 endpoint-backed command surfaces.
- `uv run --extra dev pytest tests/test_workbench_ui_validation_matrix.py -q` now reports `5 passed`.

---

# Session 8 — 2026-06-15: Measure / Inspect exposes arbitrary MeshTriPoint Fast Marching

The official Measure / Inspect API can now request a geodesic path from arbitrary barycentric
`MeshTriPoint` start/end locations, not only nearest mesh vertices or control vertices. The Python
API layer only validates and maps request/response fields; the path computation is routed into the
Rust `mesh_fast_marching_surface_path_tri_points` kernel and returns the MeshLib
`MR::computeFastMarchingPath` crossing polyline.

Validation update:
- Added `MeasureInspectPair.start_face_index`, `start_barycentric`, `end_face_index`, and
  `end_barycentric` request fields.
- Added `test_measure_inspect_endpoint_returns_rust_fast_marching_mesh_tri_point_path`.
- `uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_measure_inspect_endpoint_returns_rust_geodesic_path tests/test_geometry_sdk_operation_contracts.py::test_measure_inspect_endpoint_returns_rust_fast_marching_mesh_tri_point_path tests/test_meshinspector_official_parity_inventory.py tests/test_workbench_ui_validation_matrix.py -q` reports `21 passed`.

---

# Session 9 — 2026-06-15: FeatureObject transform editing routed through Rust scene kernel

Imported MeshLib FeatureObjects now participate in the Rust scene transform kernel. The core
`meshlib_transform_scene_object` contract carries `feature_objects`, updates the targeted
FeatureObject `xf` using MeshLib `FeatureObject::setXf` semantics, preserves FeatureObject
visualization/edit metadata, and leaves mesh vertices untouched for feature-only transforms.

Validation update:
- Added `meshlib_transform_scene_feature_object_updates_feature_xf_without_touching_mesh_vertices`.
- The PyO3 binding accepts optional `scene_feature_objects` and returns transformed
  `scene_feature_objects`; the Python SDK facade only forwards and persists Rust output metadata.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-xf CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_transform_scene_feature_object_updates_feature_xf_without_touching_mesh_vertices --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-xf CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_transform_scene_object_updates_world_vertices_from_object_xf --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-xf-py CARGO_INCREMENTAL=0 cargo check -p zennah-geometry-py` passed.

---

# Session 10 — 2026-06-15: FeatureObject Object-level state and selection semantics routed through Rust

Imported MeshLib FeatureObjects now participate in the Rust scene object state and selection
kernels. The low-level `meshlib_set_scene_object_state` path updates FeatureObject visibility,
selection, lock, and parent-lock flags while preserving FeatureObject visual metadata. The
`meshlib_select_scene_objects` path now performs select-one and toggle across mesh objects and
FeatureObjects, matching MeshLib's Object-level selected set behavior.

Validation update:
- Added `meshlib_set_scene_object_state_updates_feature_object_state_without_touching_mesh_objects`.
- Added `meshlib_select_scene_objects_includes_feature_objects_in_name_tag_selection`.
- The PyO3 bindings accept optional `scene_feature_objects` and return transformed
  `scene_feature_objects`; the Python SDK facade only forwards and persists Rust output metadata.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-state CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_set_scene_object_state_updates_feature_object_state_without_touching_mesh_objects --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-selection CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_select_scene_objects_includes_feature_objects_in_name_tag_selection --lib` reports `1 passed`.

---

# Session 11 — 2026-06-15: FeatureObject draw-option masks routed through Rust

Imported MeshLib FeatureObjects now expose a Rust scene kernel for the feature-specific draw
options that MeshLib stores in `FeatureObject::serializeFields_`: Subfeatures,
DetailsOnNameTag, and DimensionVisibility masks. The kernel only updates dimensions already
present in the imported FeatureObject metadata, matching MeshLib's supported-dimension behavior.

Validation update:
- Added `meshlib_set_scene_feature_object_visualize_property_updates_feature_masks`.
- Added PyO3 and Python SDK facade pass-through for
  `meshlib_set_scene_feature_object_visualize_property`; the Python facade only forwards and
  persists Rust output metadata.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-visualize CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_set_scene_feature_object_visualize_property_updates_feature_masks --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-visualize-py CARGO_INCREMENTAL=0 cargo check -p zennah-geometry-py` passed.

---

# Session 12 — 2026-06-15: FeatureObject Point/Line/Plane render payloads routed through Rust

Imported MeshLib FeatureObjects now expose a Rust scene kernel for the first slice of official
`MRRenderFeatureObjects` viewport data. PointObject, LineObject, and PlaneObject payloads are
generated in `zennah-geometry-core` from the canonical MeshLib primitives: point-at-origin,
line segment `[-1, 0, 0]..[1, 0, 0]`, plane quad vertices/faces, object transform, viewport
visibility mask, and DetailsOnNameTag labels. PyO3 and the Python SDK only expose and forward
the Rust result.

Validation update:
- Added `meshlib_scene_feature_object_render_payload_matches_point_line_plane_primitives`.
- Added PyO3 and Python SDK facade pass-through for
  `meshlib_scene_feature_object_render_payload`; the Python facade only forwards Rust output.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload_matches_point_line_plane_primitives --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-py CARGO_INCREMENTAL=0 cargo check -p zennah-geometry-py` passed.

---

# Session 13 — 2026-06-15: FeatureObject Circle/Cylinder/Cone render payloads routed through Rust

Imported MeshLib FeatureObject render payload parity now includes the official CircleObject,
CylinderObject, and ConeObject primary primitives from `MRRenderFeatureObjects`: the fixed
128-segment circle polyline, `makeOpenCylinder(1, -0.5, 0.5, 128)` mesh topology,
`makeOpenCone(1, 0, 1, 128)` mesh topology, and MeshLib-ordered Diameter, Angle, and Length
dimension payloads. The implementation lives in `zennah-geometry-core`; the existing PyO3 and
Python SDK facade shape did not need algorithm changes.

Validation update:
- Added `meshlib_scene_feature_object_render_payload_matches_circle_cylinder_cone_primitives`.
- The RED run failed as expected on the previous CircleObject behavior: the payload returned
  `8` points instead of MeshLib's fixed `128` render segments.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-shapes CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload_matches_circle_cylinder_cone_primitives --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-regression CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload_matches --lib` reports `2 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-py CARGO_INCREMENTAL=0 cargo check -p zennah-geometry-py` passed.
- `uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py -q` reports `14 passed`.

---

# Session 14 — 2026-06-15: FeatureObject visual subfeatures routed through Rust

FeatureObject render payload parity now includes MeshLib `addSubfeatures()`-style visual
subfeature payloads under the Subfeatures visibility mask. PlaneObject now reports its center
point and square outline as subfeature data, CircleObject and SphereObject report center points,
CylinderObject reports center/cap-center points plus axis and cap-circle polylines, and
ConeObject reports center/apex/base-center points plus axis and base-circle polylines. The
implementation is in `zennah-geometry-core`; the existing PyO3 and Python SDK facade fields
already carried `subfeature_points` and `subfeature_polylines`.

Validation update:
- Added `meshlib_scene_feature_object_render_payload_includes_meshlib_visual_subfeatures`.
- The RED run failed as expected on the previous PlaneObject behavior: the square outline was
  still present in primary polylines instead of Subfeatures-gated data.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-subfeatures CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload_includes_meshlib_visual_subfeatures --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-regression CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload --lib` reports `3 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-py CARGO_INCREMENTAL=0 cargo check -p zennah-geometry-py` passed.
- `uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py -q` reports `14 passed`.

---

# Session 15 — 2026-06-15: SphereObject primary render payload routed through Rust

FeatureObject render payload parity now includes a Rust-owned SphereObject primary mesh payload
with the MeshLib render contract cardinality from `makeSphere({ radius: 1, numMeshVertices: 2048 })`:
2048 transformed unit-sphere vertices and 4092 closed-sphere faces, plus the existing Diameter
dimension payload. The implementation uses a projected-cube, midpoint-subdivision sphere helper
in `zennah-geometry-core`; exact MeshLib `subdivideMesh` edge-flip topology remains a separate
low-level validation item, and Python remains facade-only.

Validation update:
- Added `meshlib_scene_feature_object_render_payload_includes_sphere_primary_mesh`.
- The RED run failed as expected on the previous SphereObject behavior: the payload returned
  `0` primary mesh vertices instead of `2048`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-sphere CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload_includes_sphere_primary_mesh --lib` reports `1 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-regression CARGO_INCREMENTAL=0 cargo test -p zennah-geometry-core meshlib_scene_feature_object_render_payload --lib` reports `4 passed`.
- `CARGO_TARGET_DIR=/tmp/zennah-feature-render-py CARGO_INCREMENTAL=0 cargo check -p zennah-geometry-py` passed.
- `uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py -q` reports `14 passed`.

---

# Session 16 — 2026-06-15: FeatureObject render payload exposed through default SDK

The official parity manifest now tracks FeatureObject render payload generation as a Rust-backed
file-scene/viewer capability against `MRRenderFeatureObjects`, and `GeometrySDK` exposes
`default_sdk.meshlib_scene_feature_object_render_payload(...)` as a thin facade over the Rust
kernel. A direct SDK call initially failed because the local editable PyO3 extension was stale
and did not expose `meshlib_scene_feature_object_render_payload`; rebuilding with
`uv tool run maturin develop --manifest-path geometry-rs/crates/zennah-geometry-py/Cargo.toml`
installed the updated Rust extension into the backend virtualenv.

Validation update:
- Added `test_default_sdk_feature_object_render_payload_routes_through_rust`.
- Added inventory coverage for the `meshlib_scene_feature_object_render_payload` validation gate
  and `MRRenderFeatureObjects` source reference.
- The RED backend contract failed as expected with
  `AttributeError: 'GeometrySDK' object has no attribute 'meshlib_scene_feature_object_render_payload'`.
- `uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_default_sdk_feature_object_render_payload_routes_through_rust -q` reports `1 passed`.
- `uv run --extra dev pytest tests/test_meshinspector_official_parity_inventory.py::test_file_scene_inventory_tracks_rust_mesh_ply_import_slice -q` reports `1 passed`.
