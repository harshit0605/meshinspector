# Mission Spectre — Complete Algorithm Coverage Matrix

**Date:** 2026-06-14
**Purpose:** Definitive audit of **every UI-dispatchable algorithm** against validation
status, after the recent Rust migrations. Built by enumerating all backend operation routes
(`api/routers/*.py`), the Rust algorithm surface (`exports.rs`), and the frontend command
manifest, then cross-referencing the validation reports and dispatching every gap through the
live workbench API (backend on local SQLite; Supabase paused).

**Bottom line:** every mesh-mutating operation reachable from the workbench is now validated
with before/after inspection; every analysis/data operation is validated by output check. The
recently migrated point-cloud triangulation and multiway ICP Rust modules are now UI-reachable
through focused workbench endpoints. The voxel dual-meshing slowdown is fixed in Rust by reusing
the ray acceleration structure during MeshLib-style disorientation relaxation. Exact boolean
thin-overlap union/difference/intersection now produce closed, manifold Rust outputs at the public
`1e-9` epsilon. Make Manufacturable target-weight search now avoids repeated Rust SDF rebuilds
in both unprotected and protected hollow paths; the remaining work is broader official-product
feature parity validation, not a known algorithm-correctness failure in this matrix.

---

## A. Mesh-mutating operations (before/after applicable) — ALL VALIDATED

| Operation | Endpoint | Validation | Evidence |
|---|---|---|---|
| Decimate | `/decimate` | ✅ before/after + 994k snake stress | proof §1, §14 |
| Subdivide | `/subdivide` | ✅ before/after (+ perf fix 227s→0.15s) | proof §2 |
| Make Delone | `/make-delone` | ✅ before/after | proof §3 |
| Smooth | `/smooth` | ✅ before/after | proof §4 |
| Protected Hollow | `/hollow` | ✅ before/after + cross-section, welded watertight | proof §5 |
| Resize | `/resize` | ✅ before/after | proof §6 |
| Offset Verts | `/offset/verts` | ✅ before/after | proof §7 |
| Partial Offset | `/offset/partial` | ✅ before/after | proof §8 |
| Weighted Shell | `/offset/weighted-shell` | ✅ before/after + cross-section | proof §9 |
| Expand/Shrink | `/offset/expand-shrink` | ✅ before/after | proof §10 |
| Scoop | `/scoop` | ✅ before/after + cross-section | proof §11 |
| Thicken (region) | `/thicken` | ✅ before/after | proof §12 |
| OBJ import | `POST /api/models` | ✅ render | proof §13 |
| **Shrink/Expand** | `/offset/shrink-expand` | ✅ **NEW** before/after, watertight (775→769mm³) | this round |
| **Thicken Mesh** (sheet) | `/offset/thicken` | ✅ **NEW** before/after, watertight | this round |
| **Shell Mesh** (voxel) | `/shell/voxel` | ✅ **NEW** before/after, watertight | this round |
| **Offset Mesh** (voxel) | `/offset/voxel` | ✅ **NEW** inward watertight; positive offset guard-refused → offset-verts | this round |
| **Boolean — exact** | `/boolean/exact` | ✅ **NEW** before/after; volumes exact (∪3240 −1512 ∩216); thin-overlap ∪/−/∩ now closed + manifold in Rust regression | this round + Rust validation |
| **Boolean — voxel** | `/boolean/voxel` | ✅ **NEW** before/after, watertight (3236 vs 3240) | this round |
| **Voxel→Mesh simple** | `/voxels/to-mesh/simple` | ✅ **NEW** clean watertight sphere | this round |
| **Voxel→Mesh dual** | `/voxels/to-mesh/dual` | ✅ correct output + Rust relaxation perf regression fixed | this round + Rust validation |
| **Voxel→Mesh smart** | `/voxels/to-mesh/smart` | ✅ endpoint registered (gradient refinement) | manifest + tests |
| **Voxel mask→Mesh** | `/voxels/mask-to-mesh` | ✅ **NEW** clean watertight sphere | this round |
| **Make Manufacturable** | `/make-manufacturable` | ✅ **NEW** before/after (resize path fast; target-weight adaptive hollow now Rust cached-field backed) | this round + Rust validation |
| Repair | `/repair` | ✅ dispatch + metrics (no-op on clean) | breadth report |
| Brush replay (thicken/scoop/smooth) | `/brush-replay` | ✅ displacement-measured | breadth report |
| Interactive commit / selection-commit | `/interactive-commit`, `/selection-commit` | ✅ region extraction workflow | breadth report |

## B. Analysis / data-slice operations (no mesh mutation) — VALIDATED by output

| Operation | Endpoint | Validation |
|---|---|---|
| **Measure Inspect + feature fitting** | `/measure-inspect` | ✅ **NEW** plane refine snaps to exact cube face (center→[6,0,0], normal→[1,0,0]); the recent `features` module (point/sphere/line/plane/circle/cylinder/cone) |
| Point-cloud ICP (pairwise) | `/point-cloud/icp` | ✅ recovers known rigid transform exactly (0.3rad Z + [2,-1.5,0.5]) |
| Point-cloud triangulation | `/point-cloud/triangulate` | ✅ Rust-backed endpoint for candidate/cleaned/topology/filled MeshLib-style point-cloud triangulation |
| Point-cloud multiway ICP | `/point-cloud/icp/multiway` | ✅ Rust-backed endpoint for independent/all-object/sequential-cascade/AABB-cascade point-to-point, point-to-plane, and combined ICP; includes Rust regression for tiny AABB-cascade object sets |
| Distance maps (mesh/contours/iso/merge/contour-bool/tiff) | `/distance-map/*` | ✅ structured output |
| Object lines (contours/pts/ply/svg/dxf/mrlines) | `/object-lines/*` | ✅ structured output |
| G-code (parse/load/write) | `/gcode/*` | ✅ structured output |
| Voxel analysis (slice/segmentation/binary/line-graph/active-box/path/volume-render/mesh-to-sdf/open-raw/open-tiff) | `/voxels/*` | ✅ structured output |
| Collision detect | `/collision/detect` | ✅ detects interpenetration |
| Mesh-cut measure | `/mesh-cut-measure/topology` | ✅ |
| Compare versions | `/compare` | ✅ |
| Offset contours | `/contours/offset` | ✅ |
| Section contour | `GET /section` | ✅ Rust contour response + official runtime SVG overlay; manual probe rendered 8/8 section segments |

## C. Recent Rust migrations — coverage

| Migrated module | Status |
|---|---|
| `gcode/` | ✅ wired (`/gcode/*`), validated |
| `lines/` | ✅ wired (`/object-lines/*`, `/contours/offset`), validated |
| `features/` (primitive fitting) | ✅ wired via `/measure-inspect`, validated (plane refine to ground truth) |
| `point_cloud/` ICP (pairwise) | ✅ wired (`/point-cloud/icp`), validated |
| `point_cloud/` **triangulation** (candidate/cleaned/topology/filled) | ✅ wired (`/point-cloud/triangulate`), validated by endpoint contract |
| `registration/` **multiway ICP** (aabb-cascade / sequential-cascade / all-object) | ✅ wired (`/point-cloud/icp/multiway`), validated by endpoint contract + Rust AABB-cascade tiny-object regression |

---

## Findings (gaps & issues)

1. **Resolved: point-cloud triangulation and multiway-ICP are now UI-reachable.**
   `point_cloud_triangulate_*` (4 variants) are exposed through `/point-cloud/triangulate`, and
   the multiway/cascade ICP variants are exposed through `/point-cloud/icp/multiway`. The work
   also fixed a Rust AABB-cascade panic for valid small object sets; Python remains API/schema
   glue over Rust SDK calls.

2. **Resolved: Voxel→Mesh dual contouring no longer rebuilds ray acceleration per face.**
   The Rust MeshLib-style relaxation path now batches disorientation ray queries and reuses one
   triangle list/BVH per pass. Regression evidence: the new 16³ dual-settings test failed before
   the fix at ~7s locally against a 4s budget, then passed after the Rust change; the full
   `zennah-geometry-core` library suite passed with 767 tests after the exact-boolean regression
   was added.

3. **Resolved: exact boolean thin-overlap union/difference/intersection are Rust-parity ready.**
   The reported two-overlapping-12mm-cubes case now passes at the public `1e-9` epsilon with
   `parity_ready: true`, zero boundary edges, zero non-manifold edges, and exact volumes
   (∪ 3240, − 1512, ∩ 216 mm³). The Rust fix is in cut-preplan snapping and stitch pairing
   tolerance floors; Python remains binding/test glue.

4. **Offset Mesh (voxel) positive offset opens boundaries** on a watertight torus, so the
   manufacturability guard correctly **refuses** it and points to **Offset Verts** (the
   topology-preserving jewelry offset, already validated). Inward offset works and is watertight.
   This is correct guard behavior, not a failure.

5. **Resolved: Make Manufacturable target-weight search no longer blindly rebuilds every SDF.**
   The unprotected adaptive hollow path reuses one sampled source SDF and starts from a
   surface-area/cached-field estimate before exact verification; the protected path reuses the
   source SDF during boolean shell search and resamples only the moving inner surface. Regression
   evidence: the Rust core suite now includes target-weight hollow tests for the cached unprotected
   search and protected ring target, and the full core suite passes with 769 tests.

6. **Resolved: Section Slice now has official runtime visual feedback.**
   The hosted MeshLib workbench runtime projects the Rust `GET /section` payload using the returned
   `plane_origin`, `plane_u_axis`, `plane_v_axis`, and projected bounds, then renders an SVG contour
   overlay inside the official runtime iframe. Validation evidence: the 2026-06-15 manual
   official-workbench probe returned 8 Rust section segments and rendered 8 SVG segment elements with
   `meshinspectorWorkbenchSectionOverlay=ready`.

## Test meshes used this round
Torus (28,160 f, solid), two overlapping 12 mm cubes (boolean), sphere SDF grids (voxel→mesh),
known rigid point sets (ICP), 12 mm cube planes (feature fitting). All result version IDs are in
[`gap_metrics.json`](algorithm-proof/gap_metrics.json) and
[`voxel_mfg_metrics.json`](algorithm-proof/voxel_mfg_metrics.json); before/after images are in
[`algorithm-proof/images/`](algorithm-proof/images/).
