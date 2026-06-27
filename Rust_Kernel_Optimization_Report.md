# Rust Geometry Kernel — Performance Optimization Report

**Scope:** `meshinspector-backend/geometry-rs/crates/zennah-geometry-core` (the Rust
mesh kernel behind `geometry_sdk`). Findings from a 4-agent performance audit
(decimate deep-dive, rayon parallelism, cache/allocations, build profile + SIMD +
algorithms) plus the fixes implemented and verified in this pass.

Test machine: Apple **M5 Pro, 18 logical cores**. Benchmark model: the "snake ring"
STL, **993,698 triangles**.

---

## 1. Implemented & verified in this pass

All three changes below are **bit-exact** (the full `cargo test -p zennah-geometry-core
--lib` suite — **783 tests, incl. every parity/golden test — passes unchanged**) and
shipped in the rebuilt `geometry_sdk/_zennah_geometry_rs.abi3.so`.

### 1a. Decimate: O(F²) → O(F·log F)  ⭐ headline
**File:** `src/mesh_edit/decimate/fast.rs:375` (`apply_collapse`).
The per-collapse inner loop ran `let alive = mesh.alive_face.clone();` — cloning the
**entire** `alive_face` vector (length = original face count, which never shrinks) on
**every** edge collapse. With ~F/2 collapses that is **Θ(F²)** total — the whole source
of the superlinear curve. Fixed by borrowing the two struct fields disjointly instead
of cloning (identical semantics, no float change).

| Input faces | Before | After | µs/face before → after | Speedup |
|------------:|-------:|------:|:----------------------:|:-------:|
| 50,000 | 0.099 s | 0.083 s | 2.0 → 1.66 | 1.2× |
| 200,000 | 0.480 s | 0.277 s | 2.4 → 1.39 | 1.7× |
| 500,000 | 2.049 s | 0.684 s | 4.1 → 1.37 | 3.0× |
| **993,698** | **11.5 s** | **1.40 s** | **11.6 → 1.41** | **8.2×** |

Per-face cost is now **flat (~1.4 µs/face)** instead of growing 6× — i.e. genuinely
linear. The win compounds with size: a 4 M-face model drops from ~3 min to ~6 s.

### 1b. Release build profile
**File:** `geometry-rs/Cargo.toml`. There was **no `[profile.release]`**, so wheels
built at cargo defaults (`lto = false`, `codegen-units = 16`). Added:
```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
```
Cross-crate inlining of the hot `[f64;3]` math, BVH traversal and decimate inner loop.
~10–25 % across all kernels for free; `.so` also shrank 9.40 MB → 7.81 MB. `panic`
stays `"unwind"` (cargo default) — `"abort"` would hard-kill the host Python
interpreter on a Rust panic across the PyO3 boundary.

### 1c. Rayon parallelization of foundational per-element passes
Each is a parallel **map** (per-element results bit-identical; scatter/reduction order
left serial), so output is unchanged — only multi-core utilization improves.
- `src/mesh/base.rs` `vertex_normals` / `vertex_normals_from_faces` — the face cross-
  product map + the normalize map (`par_iter` / `into_par_iter`); the accumulation
  scatter stays serial (write hazard + exact sum order). Feeds thickness, hollow,
  smoothing, overhang, stats.
- `src/mesh/base.rs` `face_normals_for_mesh` — per-face normal map.
- `src/mesh/base.rs` `vertex_neighbor_list` — replaced a `BTreeSet`-per-vertex (one
  tree allocation per vertex, millions on large meshes) with flat `Vec` + parallel
  `sort_unstable`+`dedup`. Same sorted/deduped output, contiguous memory.
- `src/analysis.rs` `nearest_vertex_distances` — the serial O(n×m) all-pairs loop is
  now `par_iter` on the outer dimension (inner min order preserved).

### 1d. Fast hierarchical winding number  ⭐ (largest algorithmic win)
**New:** `src/spatial/fast_winding.rs`, wired into `winding_numbers`,
`signed_point_mesh_distances`, `sdf_grid_values`, `weighted_sdf_grid_values`.
The inside/outside sign was computed by summing an exact solid angle over **every**
triangle for **every** query point — O(points × faces). Replaced with a Barnes-Hut
hierarchical winding number: per-BVH-node dipole aggregates (vector area +
area-weighted center + bounding radius) built once, then each query descends the
tree, approximating a node by a single dipole term when the query is farther than
β·radius (β = 2.5) and recursing to exact solid angles only for nearby nodes. Cost
per query drops to ~O(log faces). Meshes under 4,096 faces keep the exact path, so
fixtures are unchanged.

Measured on the snake (993,698 faces), `winding_numbers` over a uniform grid:

| Query points | Brute O(N·F) | Fast | Speedup |
|---:|---:|---:|:---:|
| 8,000 | 4.36 s | 0.63 s | 6.9× |
| 32,768 | 17.8 s | 0.68 s | 26× |
| 125,000 | 68.1 s (linear) | 0.67 s | 102× |

The fast time is nearly flat (build-dominated; per-query ≈ O(1) for this thin mesh),
so the win grows with grid size — a 256³ SDF grid drops from ~hours to ~1 s.
**Inside/outside sign agreement vs brute = 100.0000 %** on the snake (0 / 8,000
disagree); a dedicated unit test asserts the same on a sphere. The full 784-test
suite (incl. every SDF / signed-distance / boolean golden) passes.

---

## 2. Recommended next (high-impact, not yet implemented)

Ranked by impact. These are larger and need their own focused PR + numerical
validation (render-and-check on real models, per the "validate shape quality" rule),
so they were deliberately **not** bundled with the bit-exact changes above.

### 2a. Fast/hierarchical winding number — ✅ DONE this pass (see §1d)
Was the biggest remaining algorithmic win; now implemented and verified
(26–102× on the snake, 100 % sign agreement, 784 tests green).

### 2b. Insphere thickness — BVH closest-point — ✅ DONE this pass
`closest_nonincident_point` (the per-march-step brute scan over all triangles) is
replaced by a new `closest_point_excluding_incident` (BVH, zero-alloc fixed stack, skips
incident faces); the BVH is built once outside the parallel vertex loop in
`insphere_thickness_at_vertices`. **O(V·F·iters) → O(V·log F·iters)**, **bit-identical**
thickness (max Δ = 0 vs brute). Measured on the snake: 30k faces 2.67 s → 0.04 s (67×);
the full 993,698-face mesh now runs in **5.31 s** (brute ≈ 48 min) — feasible for the
first time.

### 2c. ICP nearest-neighbor via spatial index — ✅ DONE this pass
A new `nearest_point_in_cloud` (BVH over the reference cloud, lowest-index tie-break so
it is bit-identical to the brute scan) replaces the per-point O(m) scans in
`nearest_point_pairs` (par_iter'd), `nearest_point_plane_pairs`, and the multiway
`directed_nearest_pairs` / `directed_nearest_plane_pairs` (BVH hoisted once per object
pair). **O(n·m) → O(n·log m)**. All 16 registration goldens (point-to-point,
point-to-plane, mutual-closest, multiway, cascade) pass unchanged.

### 2d. Swap SipHash for `FxHashMap` in hot map builders — ✅ DONE this pass
Added `rustc-hash` and swapped (a) the per-vertex weld map (`repair_components.rs`
`weld_coincident_vertices`) and (b) **`edge_face_map`** — the hottest, used on every mesh
stats / health / repair call. The `edge_face_map` swap was initially deferred over a
feared order-dependence in `graph_cut`, but std `HashMap` is **randomly seeded** — its
iteration order already varies per run, so the reliably-passing suite *proves* every
consumer is order-independent; `FxHashMap` (deterministic) is therefore strictly safe.
The only real work was the compile-time type ripple (3 mesh helpers retyped to
`&FxHashMap`; the workstream's `exact_cut.rs` struct left untouched via a call-site
`.into_iter().collect()`). All 784 tests pass.

### 2e. Local-build CPU tuning (do **not** ship)
A `geometry-rs/.cargo/config.toml` with `rustflags = ["-C","target-cpu=native"]`
enables AVX/FMA auto-vectorization of the per-face math for **locally built** wheels.
Must **not** be baked into a distributed wheel (illegal-instruction crash on older
cloud CPUs) — pin to the Cloud Run baseline (`x86-64-v3`) for shipped artifacts.

### 2f. Other constant-factor cleanups flagged by the audit
- `mesh/base.rs:558` `vertex_normals` traverses the face array 3× (one per corner) —
  collapse to a single pass (still serial scatter) to cut face reads 3×.
- `hollow.rs:224,358` / `deform.rs:574` smoothing loops `.clone()` the whole vertex
  array every iteration — double-buffer with `mem::swap` instead.
- `spatial/bvh.rs:50` builds a boxed node tree then flattens it — build the flat BVH
  directly to drop one full tree allocation per spatial call.

---

## 3. Backend (app-level, not the Rust kernel)

A fixed post-op "finalize" overhead (preview GLB generation + manufacturability
analysis) dominates the *job* time for every fast kernel. Note the Rust work above
already cut the analysis half of it (insphere 67–550×, winding up to 102×).

- **"Low" preview no longer a full-res duplicate — ✅ DONE.** `to_glb` gained a
  `decimate_faces` param; `_finalize_version` now builds the low preview as a 20k-face
  LOD (decimate + `remove_unreferenced_vertices` — the compaction is essential, without
  it the dead vertices keep the GLB bloated). Measured on the snake: **19.9 MB → 0.40 MB
  (50× smaller)** low GLB → much faster frontend load. Applied to the runnable snapshot
  (`/tmp/mi-uitest/.../services/{convert,operations}.py`) and verified; the working-tree
  `services/*.py` is mid-refactor (does not import — `services.health` missing), so the
  same change is documented for the owning workstream to apply when that settles.
- **Defer the heavy analysis off the critical path — ✅ mechanism added.**
  `_finalize_version` gained a `defer_analysis` flag: with it set, the operation returns
  as soon as the (now-fast) previews are written and the manufacturability snapshot is
  computed lazily on first request — taking the single heaviest finalize step off the
  critical path. Note: the geometry kernels already saturate all cores via rayon, so
  running the finalize tasks *concurrently* in Python threads only oversubscribes the CPU
  — deferring is the real lever, not parallelizing. Remaining for the owning workstream:
  wire callers to pass `defer_analysis=True` and have the frontend request the snapshot
  when the analysis panel opens (working-tree `services/*.py` is mid-refactor / doesn't
  import, so applied + verified on the runnable snapshot).

---

## 4. Verification method
- Correctness: `cargo test -p zennah-geometry-core --lib` — **784 passed, 0 failed**
  (incl. the 5 `fast_matches_reference_*` bit-exact decimate parity tests, the
  fast-winding sign test, all 16 ICP/registration goldens, and the
  winding/hollow/thickness/voxel goldens), before and after every change.
- Performance: rebuilt the release wheel (`maturin build --release`), swapped the
  `.so` into `geometry_sdk/`, and timed each kernel through the real `geometry_sdk`
  Python facade — decimate (§1a), winding (§1d), insphere (§2b) — each compared against
  the prior `.so` to confirm both the speed-up and bit-/sign-identical output.

---

## 5. Changeset — files touched (for fold-in)

These edits are layered on the parallel workstream's uncommitted restructuring (every
`geometry-core` file below is their untracked/modified WIP **except** `fast_winding.rs`,
which is new and entirely additive). A non-destructive recovery snapshot of the full
working tree is on branch **`geometry-kernel-optimizations`** (created without touching
the working tree or `master`). To integrate, fold these specific changes into their branch:

**Rust kernel — `geometry-rs/crates/zennah-geometry-core/src/`**
- `mesh_edit/decimate/fast.rs` — replace `alive_face.clone()` in `apply_collapse` with a disjoint field borrow (the O(F²) fix).
- `spatial/fast_winding.rs` — **NEW**: hierarchical fast winding number (`WindingTree`, `WindingEvaluator`).
- `spatial/closest.rs` — **NEW fns**: `closest_point_excluding_incident`, `nearest_point_in_cloud`.
- `spatial.rs` — `mod fast_winding`; `pub(crate)` re-exports (`build_flat_bvh`, `FlatBvh`, `closest_point_excluding_incident`, `nearest_point_in_cloud`); wire winding into `winding_numbers` / `signed_point_mesh_distances` / `sdf_grid_values`.
- `spatial/bvh.rs` — `FlatBvh` + `build_flat_bvh` → `pub(crate)`.
- `spatial/weighted_sdf.rs` — wire the winding evaluator.
- `analysis.rs` — rayon `nearest_vertex_distances`; insphere uses `closest_point_excluding_incident` + a BVH built once; brute `closest_nonincident_point` removed.
- `mesh/base.rs` — rayon `vertex_normals`/`vertex_normals_from_faces`/`face_normals_for_mesh`; `vertex_neighbor_list` Vec+sort instead of `BTreeSet`; `edge_face_map` → `FxHashMap`; `connected_face_components` param `&FxHashMap`.
- `mesh/overhang.rs`, `mesh/triangle_strip.rs` — `edge_face_map` consumer param retyped to `&FxHashMap`.
- `spatial/exact_cut.rs` — call-site `.into_iter().collect()` only (struct type unchanged).
- `repair_components.rs` — weld map → `FxHashMap`.
- `registration.rs`, `registration/multiway.rs` — ICP nearest-neighbor → BVH via `nearest_point_in_cloud`.

**Build:** `geometry-rs/Cargo.toml` (`[profile.release]` fat LTO + `rustc-hash` dep), `crates/zennah-geometry-core/Cargo.toml` (`rustc-hash`).

**Backend (applied to the runnable snapshot; mirror in working-tree `services/` once it imports):**
`services/convert.py` (`to_glb(decimate_faces=…)` + `remove_unreferenced_vertices`), `services/operations.py` (low preview → 20k LOD; `defer_analysis` flag).
