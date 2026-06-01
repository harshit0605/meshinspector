# In-House Geometry SDK Migration Plan

## Decision

Build an internal geometry SDK that preserves the current MeshInspector product contract while using MeshLib, trimesh, and scipy as references/benchmarks during development. Do not replace current production integrations yet. The first useful milestone is a parallel SDK with parity tests; production migration can then happen module by module once gates are green.

Working name: `zennah_geometry`.

Initial home:

- `meshinspector-backend/geometry_sdk/`

Longer-term home, once the API stabilizes:

- separate `zennah-geometry` package, with Python bindings for backend usage and optional WASM/native bindings for interactive tools.

Rust performance direction:

- keep the Python SDK facade only as the product/service compatibility API while ported algorithm ownership moves to Rust
- move complete geometry modules into Rust behind a PyO3 binding module
- after a module is ported, keep Python files as thin wrappers only; do not add new Python-owned algorithm logic there
- keep MeshLib/trimesh/scipy out of the Rust core; those remain reference adapters only
- preserve a NumPy-array ABI at the Python boundary so current backend services and tests can migrate incrementally

## Goals

- Preserve every current backend operation and frontend-visible behavior.
- Make MeshLib/trimesh usage an implementation detail over time, after the parallel SDK proves parity for each operation.
- Create in-house algorithm ownership for jewelry-specific geometry: ring sizing, semantic regions, protected hollowing weights, drain planning, local deformation, thickness interpretation, and manufacturability decisions.
- Keep product constants and deterministic reductions, such as material density tables and volume/weight conversions, in the same Rust-owned SDK core once ported so downstream product reports do not accumulate Python-only logic.
- Use MeshLib's topology-first shape as directional input for Rust ports: directional-area normals, edge-incidence components, and explicit boundary traversal should live in Rust core modules while Python remains an API compatibility facade.
- Before each Rust geometry port, inspect the corresponding MeshLib SDK/open-source implementation first and use it as the directional algorithm reference and parity benchmark; do not copy MeshLib internals line by line.
- For SDF and voxel work, use MeshLib's FloatGrid/OpenVDB-style value-transform architecture as the directional model: boolean min/max composition, level-set offset shifts, and shell-band extraction belong in Rust-owned value kernels before higher-level mesh extraction is migrated.
- Keep SDF grid value access and reductions Rust-owned: cell averaging, occupancy, volume estimates, interpolation, and gradients should sit beside the field kernels instead of staying as Python NumPy algorithms.
- Keep conservative grid extraction, raw marching extraction, and finalized marching mesh cleanup as separate Rust kernels, mirroring MeshLib's separation between volume/value preparation, mesh generation, and topology repair.
- Keep extracted-surface refinement in Rust as a staged smooth/relax plus SDF projection pipeline, matching the MeshLib direction of improving voxel surfaces after iso-surface extraction rather than mixing refinement into Python orchestration.
- Replace generic geometry algorithms in risk order, starting with low-risk analysis/deformation code and ending with booleans and voxel offsets.
- Build golden-mesh regression coverage before replacing behavior.

## Non-Goals For The First Milestone

- No line-by-line port of MeshLib. Use MeshLib as behavioral reference and architectural study, not source to copy.
- No production service rewiring in the first implementation slice. Current MeshLib-backed app behavior remains untouched while the SDK matures.
- No immediate removal of all third-party file parsers. Algorithm independence and app-level control matter first; file-format IO can be isolated and replaced later.
- No immediate replacement of Three.js viewer infrastructure. The backend geometry authority should move first.

## Current External Dependency Surface

| Product Area | Current Files | External Geometry APIs | Current Contract To Preserve |
|---|---|---|---|
| Import, normalization, export | `meshinspector-backend/services/convert.py`, `services/ingest.py` | `trimesh.load`, scene merge, PLY/STL/GLB export | Accept `glb`, `gltf`, `obj`, `stl`, `ply`; preserve original; produce normalized PLY, GLB previews, STL export |
| Baseline analysis | `services/analyze.py`, `services/manufacturability.py` | `trimesh` bounds, volume, connected components | Volume, material weights, bbox, vertex/face counts, disconnected shells |
| Health | `services/health.py` | MeshLib `findSelfCollidingTrianglesBS`, hole boundary loops | Closed status, hole count, self-intersection count/faces, health score |
| Repair | `services/health.py`, legacy `services/repair.py` | MeshLib `fillHole`, `saveMesh`; trimesh `fill_holes`, normals, merge vertices | Versioned repair job with repair log and regenerated artifacts |
| Thickness | `services/thickness_meshlib.py`, legacy `services/thickness.py` | MeshLib ray thickness and insphere thickness | Per-vertex scalar NPZ, min/avg/max, violation count |
| Ring measurement | `services/measure_ring.py` | NumPy/trimesh geometry arrays | Axis, confidence, inner diameter, ring size, band width, head height |
| Semantic regions | `services/regions.py` | NumPy/trimesh vertex normals | `inner_band`, `outer_band`, `head`, `ornament_relief`, `unknown` regions with allowed operations |
| Resize | `services/resize.py` | `trimesh`, `scipy.spatial.cKDTree` | Axis-aware radial scaling with optional head/detail preservation |
| Hollow and drain holes | `services/hollow.py` | MeshLib `offsetMesh`, `boolean`; trimesh cylinder/cutter generation; scipy voxel fallback | Fixed thickness, target weight, protected-region hollowing, drain-hole subtraction |
| Thicken | `services/operations.py` | MeshLib `generalOffsetMesh`; NumPy/trimesh local displacement | Global thickening and local selected/violating-region thickening |
| Scoop | `services/operations.py` | NumPy/trimesh local displacement and smoothing | Region-constrained inward deformation with thickness guard |
| Smooth | `services/operations.py` | `trimesh.smoothing.filter_taubin`, custom local Laplacian-like smoothing | Local, batch, and global smoothing |
| Compare | `services/operations.py` | MeshLib `findSignedDistances` | Signed-distance scalar artifact and summary stats |
| Interactive workbench | `meshlib-workbench/*`, `MeshLibWorkbenchHost.tsx` | MeshLib viewer, `SurfaceManipulationWidget` | Select/mark, thicken brush, scoop brush, smooth brush, measure/inspect, interactive commit |

Important cleanup note: `meshinspector-backend/api/routers/versions.py` imports `meshlib.mrmeshpy as mm` but does not currently call it. That should disappear once imports are routed through the SDK.

## Product Contract To Keep Stable

The SDK must preserve the current versioned pipeline rather than older compatibility routes:

- upload creates a model and initial version
- every geometry operation creates a new immutable version
- artifacts remain per-version
- heavy geometry runs through async jobs
- snapshots and overlays are generated after each operation

Primary operation types:

- `repair`
- `resize`
- `hollow`
- `thicken`
- `scoop`
- `smooth`
- `compare`
- `make_manufacturable`
- `interactive_commit`

Primary artifact types:

- `original_upload`
- `normalized_mesh_ply`
- `preview_glb_low`
- `preview_glb_high`
- `manufacturing_stl`
- `analysis_thickness_npz`
- `analysis_regions_json`
- `analysis_compare_npz_<other_version_id>`
- `interactive_edit_payload_json`

## MeshLib Architecture Takeaways

These are the useful architectural lessons from the local `MeshLib/` checkout:

### Core Mesh Model

MeshLib centers on a mesh object with explicit topology plus vertex coordinates. Topology is half-edge based and supports stable face, edge, and vertex IDs, boundary loop traversal, region bitsets, and cached acceleration structures.

In-house implication:

- Build an internal mesh representation around typed IDs and adjacency, not only raw `vertices` and `faces` arrays.
- Keep conversion helpers to and from NumPy arrays for analytics and Python integration.
- Store optional attributes: normals, scalar fields, region masks, material/color data, and source-index maps.

### Boolean Operations

MeshLib's exact boolean path cuts both input meshes along intersection contours, stitches or merges the cut topology, classifies components as inside/outside, then assembles the requested operation. It also has a voxel/SDF boolean path for volume-style operations.

In-house implication:

- Exact booleans are one of the hardest parts and should be late-stage.
- Start with a robust voxel/SDF boolean for hollow drain subtraction and conservative manufacturing cuts.
- Keep exact CSG behind an adapter until our BVH, triangle intersection, contour cutting, and component classification are proven.
- Port exact CSG in the staged shape MeshLib uses: centered float-to-int coordinate conversion, exact integer orientation predicates, triangle-segment intersection classification, ordered intersection contours, cut topology, inside/outside component classification, then operation assembly.
- Keep the exact-predicate layer independent from the existing SDF boolean backend. SDF booleans remain the production-oriented V0 path for hollowing/cutters while exact booleans mature behind separate tests.

### Offsets, Shells, Hollowing

MeshLib's production offsets are primarily distance-field and voxel based: mesh to signed/unsigned distance volume, iso-surface extraction through marching cubes or dual marching cubes, and optional sharpening/reference projection.

In-house implication:

- The SDK needs a `voxel` module before we can own hollowing fully.
- Current global thickening should mirror the app's MeshLib path first: `GeneralOffsetParameters` default mode routes `generalOffsetMesh` to standard marching-cubes offset, with service voxel size `max(bbox_diagonal * 0.0025, min_target_thickness_mm / 4)` and offset `min_target_thickness_mm / 2`.
- Current fixed hollowing should mirror the app's MeshLib path first: negative `offsetMesh` creates the inner surface, `boolean(DifferenceAB)` subtracts that inner surface from the outer mesh, and the service voxel size is `max(bbox_diagonal * 0.005, wall_thickness_mm / 4)`.
- Protected hollowing should be owned by our jewelry layer first: region weights, preserve masks, target-weight search, drain placement, and final validation.
- The offset engine can initially be MeshLib-backed behind `OffsetEngine`.

### Repair And Healing

MeshLib repair is a collection of focused topology algorithms: boundary loop discovery, hole filling/stitching, degeneracy handling, self-intersection detection, local region repair, and full rebuild through voxelization when topology is too damaged.

In-house implication:

- Do not build a vague `repair()` monolith.
- Build composable passes: `fix_normals`, `merge_close_vertices`, `remove_degenerate_faces`, `find_holes`, `fill_small_holes`, `detect_self_intersections`, `repair_self_intersections`, `rebuild_via_sdf`.
- Current service-style hole filling should follow MeshLib `fillHole` direction: build a triangulation plan over the existing boundary vertices, prefer compact triangles through a metric, and keep centroid-fan filling as a separate simple planar helper.
- Current service-style health should follow MeshLib `findSelfCollidingTrianglesBS`: compute the union of self-intersecting face IDs, expose only the bounded face list used by the app, count holes from boundary loops, and keep the exact production health-score arithmetic as a separate Rust-owned service contract.

### Thickness And Distance

MeshLib combines AABB/BVH closest-point traversal, signed distance estimation, inward ray thickness, and maximal inscribed sphere thickness.

In-house implication:

- A high-quality BVH is foundational for self-intersection, compare, ray thickness, closest point, picking, and signed distance.
- Thickness should preserve the current service contract by composing finite ray thickness with shrinking-sphere inspired in-sphere thickness, using MeshLib's `MRMeshThickness` architecture as direction while keeping the implementation independent.
- Compare overlays should preserve MeshLib `findSignedDistances(refMesh, mesh)` direction explicitly: the service field is sampled on the compared/other mesh vertices against the source/reference mesh, then clamped and summarized with source-minus-other volume and bbox deltas.

### Decimation And Remeshing

MeshLib uses QEM-style edge collapse with boundary/aspect/dihedral constraints and a separate remesh pass that splits long edges and collapses short ones.

In-house implication:

- Decimation is useful but not on the app's current critical path.
- Add it after BVH/repair/thickness unless preview simplification becomes urgent.

### IO

MeshLib uses a registry-based IO layer dispatching by extension.

In-house implication:

- Mirror the registry pattern so app code calls `geometry_sdk.io.load_any()` and `save_any()` rather than directly calling `trimesh`.
- Keep third-party parsers inside adapters until replacing IO is worth the cost.

## Rust Performance Architecture

The in-house SDK should become a hybrid Python/Rust package rather than a pure Python rewrite. Python remains the orchestration, test, artifact, and service layer; Rust owns the high-volume kernels that need predictable performance.

Recommended structure:

```text
meshinspector-backend/
  geometry_sdk/                  # Python facade, dataclasses, adapters, tests
  geometry-rs/
    Cargo.toml                   # Rust workspace
    crates/
      zennah-geometry-core/      # pure Rust mesh, topology, BVH, SDF kernels
      zennah-geometry-py/        # PyO3/maturin Python extension module
      zennah-geometry-bench/     # Criterion and corpus benchmarks
```

Longer-term extracted package:

```text
zennah-geometry/
  python/zennah_geometry/        # Python facade and compatibility wrappers
  rust/crates/zennah-geometry-core/
  rust/crates/zennah-geometry-py/
  tests/parity/
  benches/
```

### Rust/Python Boundary

- Use PyO3 for Python extension bindings and expose a private module such as `_zennah_geometry_rs`.
- Use maturin as the build backend for local development, CI wheels, and eventual package publishing.
- Use Rust NumPy bindings for input/output arrays:
  - vertices: C-contiguous `float64[:, 3]`
  - faces: C-contiguous `int64[:, 3]` at the Python boundary, converted to `u32`/`usize` internally after validation
  - scalar outputs: `float32[:]`
  - index outputs: `int64[:]` or `int64[:, 2]`
- The Python `GeometrySDK` should attempt to call Rust kernels for ported modules. Transitional Python fallbacks are allowed only for modules that have not yet been ported, and they should be removed or quarantined once the Rust module owns the behavior.
- Keep conversions explicit: no Rust function should accept MeshLib or trimesh objects.

Initial PyO3 function shape:

```rust
#[pyfunction]
fn self_intersecting_faces(
    vertices: PyReadonlyArray2<f64>,
    faces: PyReadonlyArray2<i64>,
    epsilon: Option<f64>,
) -> PyResult<Py<PyArray1<i64>>> { ... }
```

Python wrapper shape:

```python
try:
    from geometry_sdk import _zennah_geometry_rs as rs
except ImportError:
    rs = None

def self_intersecting_faces(mesh: MeshDocument) -> set[int]:
    if rs is not None:
        return set(rs.self_intersecting_faces(mesh.vertices, mesh.faces))
    return python_self_intersecting_faces(mesh)
```

For Rust-owned modules, the wrapper shape should be stricter:

```python
def measure_ring(mesh: MeshDocument) -> RingMeasurement:
    result = rust.measure_ring(mesh)
    if result is None:
        raise RuntimeError("Rust kernel measure_ring is required")
    return result
```

### Rust Crate Roles

`zennah-geometry-core` should be pure Rust and free of Python dependencies:

- mesh validation and typed IDs
- adjacency and boundary loop extraction
- flat BVH/R-tree broad phase
- exact predicate kernels for future MeshLib-style booleans: centered float-to-int conversion, integer tetrahedron volume signs, symbolic orientation fallback, and triangle-segment intersection classification
- ray/triangle, segment/triangle, triangle/triangle tests
- closest point and signed distance
- self-intersection detection
- ray-thickness and compare scalar fields
- SDF sampling, offset, shell, boolean field operations
- isosurface extraction and projection/refinement

`zennah-geometry-py` should be a thin binding crate:

- validate NumPy shape/dtype/contiguity
- convert indices to internal types
- release the GIL for long-running kernels
- return NumPy arrays and structured error codes
- expose only stable kernel functions used by the Python facade

`zennah-geometry-bench` should hold:

- generated fixture benchmarks
- real app sample benchmarks
- MeshLib parity benchmark runners
- regression thresholds for time and output drift

### Candidate Rust Dependencies

Use these as starting points, not irreversible commitments:

| Area | Candidate | Use |
|---|---|---|
| Python binding | `pyo3` | CPython extension module exposed to the existing backend |
| Build/package | `maturin` | local `develop`, wheel builds, CI packaging |
| NumPy interop | `numpy` / rust-numpy | zero/low-copy NumPy array inputs and outputs |
| Parallel loops | `rayon` | parallel triangle/ray/SDF loops and reductions |
| Linear algebra | `nalgebra` or small custom `Vec3` | vector math for geometry kernels |
| Spatial index | custom flat BVH first; evaluate `bvh` and `rstar` | ray queries, nearest surface, broad phase |
| Benchmarks | `criterion` | kernel-level performance regression tests |

Recommendation: start with a custom flat BVH in `zennah-geometry-core` rather than making `rstar` or `bvh` the core abstraction. Existing crates are useful references and possible accelerators, but the SDK needs predictable face IDs, source maps, and pair traversal for booleans/repair. Keep the trait boundary narrow enough that an external crate can be swapped in later.

### Rust Kernel Priority

Move kernels in this order:

1. Array validation, bbox, face areas, volume, adjacency, boundary edges.
2. Flat BVH build and broad-phase pair traversal.
3. Raycast, closest point, and triangle/triangle intersection.
4. Self-intersection detection and signed compare fields.
5. Ray-thickness and violation clustering.
6. SDF sampling, offset/shell fields, and marching extraction.
7. Voxel boolean/hollow operations.
8. Exact boolean predicate layer, contour cutting, component classification, and stitching.

This keeps low-risk kernels first and puts the expensive correctness-sensitive work behind parity gates.

### Rust Rollout Gates

Do not use a Rust kernel in production merely because it is faster. Each Rust-backed operation needs:

- legacy behavior parity test from the pre-port Python implementation or a stored golden/reference artifact.
- MeshLib/trimesh reference parity test where an external reference exists.
- deterministic fixture golden test.
- real app sample benchmark.
- artifact contract test when the operation produces overlays or meshes.
- memory ceiling check on large samples.
- performance threshold compared with the Python implementation.
- feature flag: `GEOMETRY_SDK_ACCELERATOR=python|rust|auto`.
- structured Rust errors and an explicit adapter/reference fallback only when a module has not yet been fully ported.

Initial benchmark targets:

- self-intersection on 100k faces: complete or safely skip under a documented time budget.
- closest-point/signed-distance for 100k query points: at least 5x faster than Python V0.
- SDF sampling on moderate jewelry fixtures: at least 3x faster than Python V0.
- no production Rust kernel can allocate more than 2x input mesh memory without a documented reason.

## Proposed SDK Layout

```text
meshinspector-backend/geometry_sdk/
  __init__.py
  engine.py
  types.py
  errors.py
  io/
    registry.py
    meshlib_adapter.py
    trimesh_adapter.py
    ply.py
    stl.py
    glb.py
  core/
    mesh.py
    topology.py
    attributes.py
    normals.py
    adjacency.py
    transforms.py
  spatial/
    aabb_tree.py
    raycast.py
    closest_point.py
    intersections.py
  analysis/
    volume.py
    connected_components.py
    health.py
    thickness.py
    compare.py
  repair/
    holes.py
    degenerates.py
    self_intersections.py
    rebuild.py
  booleans/
    exact.py
    voxel.py
  offsets/
    offset.py
    shell.py
    hollow.py
  jewelry/
    ring_measurement.py
    regions.py
    resize.py
    protected_hollow.py
    drain_holes.py
    manufacturability.py
  deform/
    thicken.py
    scoop.py
    smooth.py
    brushes.py
  adapters/
    meshlib_engine.py
    trimesh_engine.py
    scipy_spatial.py
  testing/
    parity.py
    fixtures.py
  ../geometry-rs/
    Cargo.toml
    crates/
      zennah-geometry-core/
      zennah-geometry-py/
      zennah-geometry-bench/
```

## Public API Sketch

The service layer should call an engine interface, not MeshLib or trimesh directly.

```python
class GeometryEngine:
    def load_mesh(self, path: Path) -> MeshDocument: ...
    def normalize_to_mm(self, source: Path) -> MeshDocument: ...
    def save_mesh(self, mesh: MeshDocument, path: Path, format: str) -> Path: ...

    def analyze(self, mesh: MeshDocument, material: str, threshold_mm: float) -> ManufacturabilityResult: ...
    def health(self, mesh: MeshDocument) -> MeshHealthResult: ...
    def repair(self, mesh: MeshDocument, params: RepairParams) -> RepairResult: ...
    def thickness(self, mesh: MeshDocument, params: ThicknessParams) -> ThicknessField: ...
    def compare(self, a: MeshDocument, b: MeshDocument) -> CompareField: ...

    def resize_ring(self, mesh: MeshDocument, params: ResizeParams) -> MeshDocument: ...
    def hollow(self, mesh: MeshDocument, params: HollowParams) -> MeshDocument: ...
    def thicken(self, mesh: MeshDocument, params: ThickenParams) -> MeshDocument: ...
    def scoop(self, mesh: MeshDocument, params: ScoopParams) -> MeshDocument: ...
    def smooth(self, mesh: MeshDocument, params: SmoothParams) -> MeshDocument: ...
```

Implementation rule:

- app services import `geometry_sdk`
- only `geometry_sdk/adapters/*` may import `meshlib`, `trimesh`, or `scipy`
- Rust kernels are imported only through `geometry_sdk.accelerators` or the `GeometrySDK` facade
- Rust core crates must not depend on Python, MeshLib, trimesh, or scipy
- tests may import adapters to assert parity

## Migration Phases

### Phase 0: Parallel SDK And Parity Harness

Expected value: in-house algorithms can be developed, benchmarked, and hardened without changing active app behavior.

Tasks:

- Add `geometry_sdk` package with core data classes and in-house V0 algorithms.
- Keep MeshLib/trimesh integrations in the app unchanged during this phase.
- Add MeshLib/trimesh reference adapters under `geometry_sdk/adapters` and `geometry_sdk/io` for tests and benchmarks only.
- Build golden fixture set: simple cube, open cube with hole, self-intersecting mesh, ring fixture, hollow ring fixture, thin-wall ring fixture, dense generated ring, pendant/non-ring fixture.
- Add parity tests around current outputs: health, thickness stats, ring measurement, regions, resize, hollow, thicken, smooth, compare.

Definition of done:

- current app behavior remains unchanged
- SDK tests compare in-house outputs to deterministic fixtures and MeshLib/trimesh reference outputs
- golden tests establish tolerances before any production service migration

### Phase 0R: Rust Accelerator Skeleton

Expected value: performance work starts behind a clean boundary before any service migration.

Tasks:

- Add `geometry-rs/` workspace with `zennah-geometry-core`, `zennah-geometry-py`, and benchmark crates.
- Add maturin/PyO3 build wiring without making it a required production dependency.
- Add a Python accelerator loader that exposes `available()`, `backend_name()`, and per-kernel feature checks.
- Port one low-risk kernel first, such as bbox/stats or boundary-edge counting.
- Add parity tests that run both Python and Rust implementations when the Rust extension is installed.
- Add CI job or local command for `cargo test`, `cargo clippy`, `cargo fmt`, and maturin wheel smoke.

Definition of done:

- `GeometrySDK` can run with `GEOMETRY_SDK_ACCELERATOR=python|rust|auto`.
- ported modules fail loudly when Rust is unavailable instead of silently reintroducing Python algorithms.
- Rust smoke tests and Python parity tests pass on the same fixture corpus.
- No production service imports the Rust module directly.

### Phase 1: Own The Jewelry Layer

Expected value: the most product-specific algorithms become customizable quickly.

Move these into `geometry_sdk/jewelry` and `geometry_sdk/deform` as in-house code:

- ring axis and size measurement
- semantic region detection
- protected region weighting
- ring resize with preserve-head falloff
- drain-hole planning
- local selected-region thicken/scoop/smooth logic
- manufacturability recommendation rules

These started as NumPy logic, but the migration target is Rust-owned modules with Python wrappers only. `jewelry.ring_measurement` is the first Rust-owned product module in this group.

Current Rust-owned compatibility modules:

- `geometry_sdk.analysis.compare`
- `geometry_sdk.analysis.health`
- `geometry_sdk.analysis.manufacturability`
- `geometry_sdk.analysis.stats`
- `geometry_sdk.analysis.thickness`
- `geometry_sdk.deform._distance`
- `geometry_sdk.deform.brushes`
- `geometry_sdk.deform.local`
- `geometry_sdk.deform.resize`
- `geometry_sdk.jewelry.hollow`
- `geometry_sdk.jewelry.ring_measurement`
- `geometry_sdk.repair.basic`
- `geometry_sdk.repair.holes`
- `geometry_sdk.repair.voxel`
- `geometry_sdk.spatial.aabb_tree`
- `geometry_sdk.spatial.closest_point`
- `geometry_sdk.spatial.intersections`
- `geometry_sdk.spatial.raycast`
- `geometry_sdk.spatial.signed_distance`

### Phase 2: Core Mesh And Topology

Expected value: internal algorithms can stop passing opaque trimesh/MeshLib objects around.

Tasks:

- Implement `MeshDocument` with vertices, faces, attributes, unit metadata, source maps, and optional region masks.
- Implement adjacency: vertex-to-face, vertex-to-vertex, face-to-face, boundary edges.
- Implement normals, bbox, volume, face area, edge lengths, connected face components.
- Implement deterministic mesh validation and ID-stable selections.

Definition of done:

- analysis, regions, resize, and local deformation run on `MeshDocument`
- only IO adapters convert to/from external mesh objects

### Phase 3: Spatial Kernel

Expected value: self-intersection, thickness, compare, and picking can move in-house.

Tasks:

- Implement AABB/BVH over triangles, with Rust as the intended production implementation.
- Implement ray-triangle intersection and triangle-triangle intersection.
- Implement closest-point queries and signed/unsigned point-to-mesh distance.
- Implement face-pair collision traversal and self-intersection detection.
- Add Python/Rust cross-parity for every spatial query.

Definition of done:

- health self-intersections can run with in-house BVH
- compare can generate signed-distance scalar fields without MeshLib
- thickness ray pass can run without MeshLib
- Rust spatial kernels meet benchmark thresholds on generated and real app samples

### Phase 4: Repair Passes

Expected value: auto repair becomes inspectable and customizable.

Tasks:

- Boundary loop extraction.
- Hole classification by perimeter, area, planarity, and region.
- Simple hole fill, planar fill, and metric-based fill.
- Degenerate triangle removal, duplicate vertex merge, isolated component cleanup.
- Normal/winding repair.
- First self-intersection handling strategy: detect and report precisely; then repair safe local cases; use rebuild fallback for severe cases.

Definition of done:

- `repair` job returns a structured repair report by pass
- simple hole/degen fixtures repair without MeshLib
- severe exact/local self-intersection repair can still fall back to the MeshLib adapter while the Rust-owned SDF rebuild fallback is hardened against larger real meshes

### Phase 5: Thickness And Compare

Expected value: manufacturability heatmaps become our own algorithm surface.

Tasks:

- Ray thickness along inward normals.
- Opposite-surface closest distance filters.
- Thickness invalid-value handling and smoothing.
- Violation clustering by connected components.
- Signed-distance compare fields with robust bounds filtering.

Definition of done:

- current `analysis_thickness_npz` and compare NPZ contracts are preserved
- fixtures match MeshLib-derived stats within agreed tolerances
- violation clusters are available for better UI and auto-thicken targeting

### Phase 6: Voxel/SDF Engine

Expected value: hollowing, drain subtraction, and broad repair can stop depending on MeshLib.

Tasks:

- Dense grid SDF prototype for small/medium meshes.
- Sparse/tiled grid design for larger generated jewelry, implemented in Rust once the dense Python prototype is stable.
- Mesh-to-SDF using BVH closest point plus sign detection.
- Marching cubes extraction.
- SDF offset, shell extraction, union/difference/intersection.
- Rebuild-via-SDF repair fallback.
- Rayon-backed SDF sampling and extraction benchmarks.

Definition of done:

- fixed-thickness hollow and drain-hole subtraction work through in-house voxel engine
- target-weight search works on in-house hollow results
- protected hollowing uses our region weights and our offset backend

### Phase 7: Exact Boolean Kernel

Expected value: high-fidelity cuts without voxel artifacts.

Tasks:

- Triangle intersection contours.
- Mesh cutting along contours.
- Topology stitching and face/edge source mapping.
- Component inside/outside classification.
- CSG assembly for union, intersection, difference.

Recommendation:

- Do this only after voxel booleans are production-acceptable for current jewelry workflows.
- Keep MeshLib as a fallback for exact booleans during development.

### Phase 8: Interactive Runtime Replacement

Expected value: MeshLib Workbench can be replaced or narrowed.

Tasks:

- Define a browser-side SDK bridge for selection masks, brush strokes, and measurement probes.
- Port local brush math to a lightweight native/WASM or TypeScript runtime.
- Keep server as final authority through `interactive_commit`.
- Preserve current interactive tools: select/mark, thicken brush, scoop brush, smooth brush, measure/inspect.

## Recommended Replacement Order

1. Adapter boundary and tests.
2. Jewelry layer and local deformations.
3. Core mesh/topology.
4. Spatial BVH/raycast/closest point.
5. Health, compare, and thickness.
6. Repair.
7. Voxel/SDF hollowing and drain booleans.
8. Exact booleans.
9. Interactive Workbench replacement.

This order gives customization early while delaying the hardest generic geometry kernels until there is a test harness and internal mesh representation.

## Test Strategy

The SDK should be developed with golden fixtures, not only unit tests.

Required fixture families:

- watertight cube/sphere/torus
- open mesh with one small hole
- mesh with multiple holes
- self-intersecting triangles
- disconnected shells
- thin-wall ring
- dense generated ring with ornament relief
- protected-head ring
- pendant/non-ring mesh
- hollowed ring with drain holes

Required assertions:

- geometry validity: vertex/face counts, closure, holes, self-intersections
- manufacturing stats: volume, weight, bbox, thickness min/avg/max, violation counts
- semantic stats: ring axis, inner diameter, regions, selected masks
- operation outputs: new artifact exists, output is loadable, no unexpected unit scale, post-operation snapshot generated
- visual/overlay contracts: scalar arrays align with normalized mesh vertex count

Tolerances:

- exact topology parity is not required for voxel/offset operations
- manufacturing metrics need numeric tolerances by operation
- destructive repair and hollowing should prioritize validity and manufacturing constraints over identical triangle layout

## Risks

| Risk | Practical Mitigation |
|---|---|
| Exact booleans are hard and failure-prone | Keep behind adapter; build voxel booleans first for manufacturing cuts |
| Python may be too slow for BVH/SDF | Prototype in Python/NumPy, then move hot loops to Rust or C++ with Python bindings |
| Replacing file IO distracts from algorithm control | Isolate IO now; replace parsers later only if licensing/deployment requires it |
| MeshLib behavior changes across versions | Golden tests lock current app behavior before replacing calls |
| Algorithm rewrites can break production jobs silently | Require every operation to regenerate manufacturability snapshots and pass post-operation validation |
| Copying MeshLib internals creates licensing/maintenance risk | Use public behavior and architecture ideas only; write independent implementations |

## Current Implementation Slice

Implemented in `meshinspector-backend/geometry_sdk/`:

- SDK-owned `MeshDocument` and typed result dataclasses.
- Public `GeometrySDK` facade for future service migration.
- Optional Rust accelerator skeleton: `geometry-rs/` workspace, pure Rust core crate, PyO3/maturin binding crate, benchmark crate, and Python loader with `GEOMETRY_SDK_ACCELERATOR=python|rust|auto`.
- Rust V0 kernels: mesh stats, boundary-loop extraction, mesh-health summary assembly, manufacturability report assembly/recommendations/export readiness, cached AABB tree broad-phase candidate traversal, flat-BVH self-intersecting face detection, flat-BVH closest-point queries with face IDs, unsigned point-to-mesh distances, signed point-to-mesh distances, topology-aware nearest/signed compare fields and summaries, service-clamped version compare fields, production-style version compare summaries, solid-angle winding numbers, flat-BVH one-shot and batched raycasting, bidirectional ray thickness, shrinking-sphere inspired in-sphere thickness, service-style combined thickness, thickness summary reduction, MeshLib-directed exact integer predicate groundwork for future exact booleans including triangle-triangle edge/face pair extraction, triangle-segment coordinate extraction, exact rational ordering for non-degenerate contour intersections, two-mesh exact candidate extraction, topological contour grouping with MeshLib's same-owner `prev(curr.sym())` successor shape, one-mesh primitive/coordinate contour extraction with MeshLib-style singleton face-lone contour merging, MeshLib-style face-lone subdivision groundwork that splits owning triangles at the first lone contour centroid, bounded MeshLib-style pre-cut retry for face-lone contours paired with edge-lone contours including open lone contours plus repeated-lone removal and cut-edge/skipped-face regression fallback while this retry is partial, MeshLib-style pre-cut topology planning with tolerance-based duplicate cut-vertex reuse, near-endpoint edge-hit snapping, undirected edge primitive canonicalization, degenerate lone segment filtering, and closed two-point contour de-duplication, simple boundary-segment, shared original-edge contour segments across adjacent faces, same-edge contour pieces preserved in output face topology, MeshLib `resultCut`-style directed cut-edge paths per contour with closure metadata downgraded when surviving path edges do not form an edge loop, non-crossing multi-boundary-chord, boundary-edge pieces mixed with interior spokes, interior-to-boundary, interior-to-interior, shared-interior multi-spoke, and three-point interior closed-cycle triangle cut mutation with skipped complex-face reporting, planar cut-hole fill-plan preparation/execution groundwork, MeshLib-style cut-boundary hole discovery/fill application with source-face and added-face-range metadata, MeshLib-style directed cut-path left/right component classification with sampling fallback and not-dividable overlap diagnostics, no-stitch boolean part assembly with output-face provenance, contour stitch compatibility planning that prefers MeshLib `resultCut`-style paths before adjacency grouping, stitch-endpoint vertex remapping, surviving stitched-edge source mapping, selected output-edge and stitch-topology edge metadata, MeshLib `connectPreparedParts`-style assembly ordering for intersection left-hole paths versus union/difference first-operand paths, MeshLib `optionalOutCut`-style source and mapped-output result-cut path diagnostics with stitched-counterpart fallback when selected-side source edges are excluded, seam face-count diagnostics during pipeline assembly, Rust topology-splice readiness planning, internal MeshLib-style half-edge splice/contour-stitch mutation groundwork, topology-splice apply planning/verification/materialization with ordered path-level verification/materialization, closed-loop end-to-start checks, and synthetic stitch-side accounting, mutable output half-edge topology with linked triangle face rings, left-face-ring export validation, exported edge-incidence diagnostics, MeshLib-style active/deleted half-edge counts, output/exported mesh stats and health diagnostics, coplanar triangle-overlap region polygons plus closed one-mesh contour diagnostics for MeshLib-style coplanar/vertex-contact degenerations, Rust-only cut/fill/classify/assemble exact boolean pipeline wrapper with stitch-plan and parity-readiness diagnostics, no-cut disjoint/contained solid parity gates, and a bounded tetra overlap parity-ready gate that requires closed, manifold, non-duplicated, fully exported materialized topology before MeshLib geometry parity is claimed, SDF-grid sampling, aligned SDF boolean value composition, resident aligned-SDF boolean/offset/shell marching extraction, product-facing voxel mesh offset/shell/boolean orchestration, service-parity global thickening, service-parity fixed hollowing, service-style triangulated hole filling, SDF rebuild repair orchestration with before/after topology reports, SDF-gradient vertex projection, resident SDF refinement, radial resize and ring-size resize, semantic ring region classification, protected hollow scale fields, inward hollow preview displacement, protected hollow mesh generation, adaptive and protected-adaptive target-weight hollow search, drain-hole planning and cutter generation, vertex-target nearest distances, Gaussian seed falloff weights, resident local thicken/scoop offset displacement, resident local thicken-to-minimum deficit displacement, resident masked/protected local brush composition, Rust-owned region-derived brush-mask planning, service-parity global Taubin smoothing, weighted/resident-seeded Laplacian smoothing, raw marching-tetrahedra extraction, and face-orientation consistency. Rust-owned modules now require the Rust extension instead of using Python algorithm fallbacks.
- Exact boolean coplanar update: generated overlap polygons are now kept as raw diagnostics, merged per source face into one-mesh boundary contours, and converted into paired global overlap-boundary contours that preserve equal A/B loop point counts. Following MeshLib's `findMeshPart`/`connectPreparedParts` direction, the Rust paired candidate now applies candidate-only coplanar policies for generated loops: union keeps same-oriented coplanar exterior components from the first operand, intersection keeps first-operand inside overlap surfaces and drops duplicate same-oriented overlap surfaces from the second operand, and difference keeps a non-promoted clean mathematical candidate for diagnostics. The cube-overlap union and intersection probes now promote paired candidates with two paired contours, sixteen contour edges, 16/16 first/second cut edges, zero unmatched stitch edges, `[1, 1]` first/second cut-path side roots, one connected component, zero boundary edges, zero nonmanifold edges, and zero duplicate output faces, matching stored MeshLib volume/area envelopes within tolerance. Bounded self-intersection diagnostics are now part of the exact-boolean Rust output gate, and the tests distinguish envelope readiness from full MeshLib topology parity: Rust union reports zero self-intersections versus MeshLib's stored 13, and Rust intersection reports three versus MeshLib's stored zero. Topology-splice export deltas are now exposed directly: `topology_splice_export_changed_faces` is false for the cube probes, confirming the exporter currently validates assembled face rings without changing them into MeshLib-style topology output. Live MeshLib targets are pinned as union 18 vertices / 32 faces, intersection 12 / 20, and difference 15 / 26, while current Rust output is union 24 / 44, intersection 16 / 28, and difference 20 / 36. The earlier Rust MeshLib topology rewrite-readiness planner first exposed the contour-record gap before any record mutation was attempted: cube union mapped 8 of 16 contour edges and was missing 8 incoming-side contour records, while cube intersection mapped 8 of 16 and was missing 8 base-side contour records. Difference remains a tracked gap: MeshLib's cube-overlap difference keeps the 2x2x2 source envelope with self-intersecting coplanar topology, while the clean Rust paired candidate is closed/manifold but has the mathematical slab envelope, so it is not promoted. A Rust MeshLib-reference fixture now pins that target as 15 vertices, 26 faces, one closed/manifold connected component, surface area 24, volume 4, and 11 self-intersecting faces; the current paired candidate is separately pinned at two self-intersections, so topology parity is still incomplete. The promotion branch remains guarded by stitch compatibility, both prepared-part dividability checks, mapped result-cut completeness where applicable, boundary/non-manifold health, duplicate-face gates, and active volume/surface-envelope preservation; exact booleans remain unexposed as Python production APIs until broader MeshLib geometry parity gates are in place.
- MeshLib topology planner refinement: union/intersection cube probes now separately report stitched-contour pairing readiness. Both probes map 16 of 16 stitched contour pairings with zero missing stitched contours and zero stitch-direction mismatches, but 8 stitched sides remain synthetic in each probe. A follow-on Rust materialization-readiness layer now uses MeshLib-style directed cut-path orientation rather than arbitrary output face-edge direction: both cube probes materialize 16 of 16 stitch contour pairs with zero unmaterialized pairs, 8 materialized synthetic sides, and zero materialization-direction mismatches. Rust core now includes MeshLib-style stitched-edge record rewrite matching the `addPartByMask` update shape (`toHe.next`, `toHe.left`, and `toHe.sym().prev`) plus the guarded open-contour near-stitch update primitive that mirrors `edges_[ePr].next = eNx; edges_[eNx].prev = ePr` after validating shared origin, open left side, and open right side. The topology planner now extracts concrete record-rewrite payloads that pin MeshLib `thisContour`/`fromContour` operand order, paired contour edges, synthetic-side flags, and the emitted stitch-pair index before mutation. It also mirrors the readiness signal for MeshLib's open-contour `prevNextEdges` stage by counting open stitch paths, endpoint near-edge updates, and blocked endpoint updates separately from closed-loop record rewriting; Rust/PyO3 diagnostics now expose these counters, and the cube union/intersection probes pin five open stitch paths, ten near-edge updates, and zero blocked near-edge updates after retaining contiguous matched subpaths across indexed cut-path length mismatches. The Rust near-stitch planner now uses the already prepared incoming part topology used by MeshLib `connectPreparedParts`, lives behind a dedicated topology helper module, carries source-edge identity on commands, and still derives cube endpoint update commands from stitch-pair indices plus materialized-output fallback. Cube union now applies four of ten commands using MeshLib-order target-side half-edge IDs captured before record rewrite and retaining longer surviving path segments; cube intersection still rejects all ten. Diagnostics now report exact translated source-halfedge failures instead of the older vertex-pair fallback result: union is ten derived commands, four applied commands, six failures split 3 start / 3 end, with buckets `origin=4`, `previous-left=1`, `next-right=1`, and failed-other zero; intersection is ten derived, zero applied, ten failures split 5 / 5, with `origin=10`, `previous-left=0`, `next-right=0`, and failed-other zero. A separate Rust rewrite-apply module now pre-registers contour `emap` targets before copied-edge materialization, materializes incoming copied-edge identity before stitched-record rewrite, then feeds record-rewrite commands and explicit near-stitch update commands into the output half-edge topology, reporting applied/failed command counts, missing-edge failures, guard failures, and export readiness. Exact-boolean diagnostics expose 16 planned record-rewrite commands, zero blocked record-rewrite edges, 8 synthetic sides, and zero record-rewrite direction mismatches for the cube union/intersection probes. The apply layer now mirrors MeshLib's mapped-face `edgePerFace_` translation step and prefers mapped source-edge candidates for near-stitch updates when command identity is available: cube union applies 16/16 record-rewrite commands, prepares 8 synthetic target contour edges, translates 8 face records, derives 10/10 near-stitch commands, applies 4/10 guarded near-stitch updates, and exports 44/44 rings; cube intersection applies 16/16 record-rewrite commands, translates 13 face records through prepared source `fromHe.left` records, derives 10/10 near-stitch commands that fail guard validation, and exports 28/28 rings. The packed rewrite-export stats/health diagnostic confirms that pack-style referenced-vertex compaction does not close the gap by itself: the rewritten export still reports Rust's 24/44 union and 16/28 intersection topology. The Rust assembly now keeps MeshLib `preparePart` masks separate from coplanar-adjusted output masks, which gives the upcoming topology-copy implementation the same raw cut-side regions MeshLib feeds into `addMeshPart`. Source-face diagnostics now pin the face-selection mismatch against that raw MeshLib `findMeshPart`-style baseline: union raw selection `[20, 20]` becomes final `[30, 14]`, intersection raw `[16, 16]` becomes final `[22, 6]`, and difference stays `[22, 14]` with zero coplanar-selection delta; all three probes expose `[13, 12]` same-oriented coplanar overlap faces. Selected-region boundary diagnostics now isolate the copied-edge blocker against MeshLib's `preparePart` remapped cut-path contract: raw union/intersection masks are boundary-clean, but final coplanar-adjusted masks report `[[0, 0], [9, 9]]`; difference remains a non-promoted topology gap at `[[20, 22], [20, 22]]`. The next exact-boolean topology task is reconciling the remaining copied/source-side near-stitch guard failures with MeshLib `preparePart` region selection and coplanar same-oriented face inclusion/deletion so command application produces MeshLib vertex/face/self-intersection parity, instead of only validating exported face rings.
- MeshLib rewrite-apply source topology update: the Rust rewrite-apply path now has a face-source-aware `OutputFaceTopology` constructor that keeps first/second operand edges separate for MeshLib `addPartByMask` simulation, while the general splice verification path keeps the existing global merge behavior. Command candidate lookup prefers operand-scoped edges and falls back to unscoped lookup for synthetic or stitch-metadata-derived contours. The apply layer now records MeshLib-style mapped contour edges before copied-edge materialization, materializes copied half-edge records through a source-aware `emap`, registers mapped/copied edges by incoming source edge for near-stitch lookup, translates copied records with the same `translateNoFlip_` next/prev walk MeshLib uses after `copyEdge`, translates mapped contour source records for the `fromHe.next`, `fromHe.left`, and `fromHe.sym().prev` stitched-edge update payload, preserves exact shared-origin/open-left/open-right assertion failures internally, scopes copied face-record translation to prepared incoming faces so the Rust `fmap` does not leak unrelated assembled output faces into copied records, and has a prepared-base-only mode that appends incoming copied `edgePerFace_` records after edge translation.
- Prepared-base rewrite scaffold: the Rust prepared-base topology builder now maps raw MeshLib `preparePart` face masks through the current output vertex map, appends virtual vertex coordinates from the raw cut mesh at the MeshLib-style `firstNewVert` boundary, preserves operand/cut/source face provenance, and feeds that base topology into the existing record-rewrite apply harness. The harness can append copied incoming face records, translate mapped contour source records, apply stitched contour commands without requiring the incoming contour edge to exist as a normal output face edge, and export base-plus-incoming prepared rings in isolation. The MeshLib diagnostics path now exposes a compact prepared-base record-rewrite summary through Rust/PyO3 while keeping the existing public counters on the final-`assembly.faces` simulation. That summary now carries bucketed record/source-map, copied-record, near-stitch guard, and export failures. The latest MeshLib-source-aligned updates key prepared-base contour/source records by raw cut-edge index/source edge, carry MeshLib's `flipOrientation` rule through copied edge records, stitched contour records, prepared-base face orientation, and copied `edgePerFace_` selection, resolve reversed prepared-base contour targets through the symmetric half-edge when MeshLib's directed `mapEdge` would address the open side of a reverse face edge, make near-stitch source walks stop on MeshLib's first left/right `fmap` hit instead of preferring region-boundary edges, switch those walks to MeshLib's flipped `prev/right` and `next/left` traversal when `flipOrientation` is active, carry exact prepared-source half-edge IDs through near-stitch commands so the apply layer first looks up the translated `emap` candidate for MeshLib's `eNx` / `ePr` before falling back to source vertex-pair matching, preserve pre-rewrite near-stitch target edges so later stitched-record rewrites do not overwrite MeshLib's stored `prevNextEdges`, use a MeshLib-style fresh incoming `vmap` for prepared-base copied records and prepared incoming near-stitch topology, start that map from contour `setVmap` endpoints instead of reusing final-assembly non-contour vertex mappings, orient incoming contour `vmap` endpoints to the actual open MeshLib target contour edge while keeping prepared `this` topology in face-edge direction, scan the source topology's actual `leftRing(from, f)` when selecting copied incoming face records, use MeshLib's direct stitched-record write shape on the prepared-base path while leaving the older final-assembly diagnostic shim stable, seed prepared `this`-operand contour vertex maps before deriving prepared-base `prevNextEdges`, compare prepared-source mapped-neighbor exclusion in source-edge space like MeshLib's `fromMappedEdges`, merge strict half-edge and source-edge `emap` candidates before applying MeshLib's near-stitch guards, and keep copied `edgePerFace_` records from the MeshLib-style face-record translation pass instead of overwriting them during direct stitched-record rewrite. This closes prepared-base missing-source and missing-target rewrite failures for cube union/intersection/difference and moves the blocker to near-stitch/copy-face left-ring closure after more stitched-record commands apply. Current probes pin union at 16 applied / 0 failed with nine export failures, intersection at 16 applied / 0 failed with eleven export failures, and difference at 20 applied / 0 failed with twelve export failures after DifferenceAB's inverted B-part handling; the latest contour-orientation pass moves one intersection near-stitch failure from origin mismatch to the next open-side guard and leaves a flipped DifferenceAB previous-edge registration gap for the next slice. The next implementation slice is closing the remaining MeshLib guard failures by reconciling source/target half-edge origin identity and copied-face left-ring closure before replacing the final-assembly counters.
- MeshLib copied-edge planning update: `ExactBooleanAssemblyResult` now persists first/second cut-vertex to output-vertex maps, and a Rust copied-edge planner computes MeshLib-style incoming prepared vertices/edges, mapped contour edges, copied edges, and copied-edge output-map readiness from raw `preparePart` masks. The planner overlays MeshLib contour `setVmap` endpoint mappings before allocating virtual copied vertices for raw prepared vertices that the coplanar-adjusted output omitted. The cube probes still pin union at 20 incoming prepared faces / 20 prepared vertices / 8 virtual copied vertices / 38 prepared edges / 16 mapped edges / 22 copied edges with all copied edges mapped, and intersection at 16 / 16 / 0 / 32 / 16 / 16. Target-side near-stitch derivation now captures MeshLib-order `prev(e1.sym())` / `next(e1)` half-edge IDs before record rewrite, source-side derivation carries exact prepared-part source half-edge IDs through the command, and stitch planning retains contiguous matched subpaths across cut-path length mismatches; together these apply four of ten cube-union endpoint updates. Remaining failures are now bucketed by the exact MeshLib-style `emap` identity path rather than fallback vertex pairs: union has `origin=4`, `previous-left=1`, `next-right=1`, intersection has `origin=10`, `previous-left=0`, `next-right=0`, and failed-other is zero. The next blocker is reconciling source/target origin identity and copied-face left-ring closure on the MeshLib-prepared base topology.
- DifferenceAB flipped-contour update: the Rust near-stitch source topology now selects MeshLib's actual source contour side before walking neighbors: non-flipped `fromContours` use the side with no right face, while flipped `fromContours` use the side with no left face, matching `addPartByMask` before `prevNextEdges` generation. The copied-edge translation map now also seeds mapped contour `emap` entries from that actual directed source side, so flipped DifferenceAB maps `e` to `e1` and `e.sym()` to `e1.sym()` like MeshLib's `mapEdge` contract. This raises prepared-base DifferenceAB near-stitch attempts from twelve to sixteen, closes one copied-face left-ring export failure, and moves the remaining failures into previous-edge registration, origin, previous-left, and next-right guard buckets.
- Prepared-base `fromMappedEdges` update: after rechecking MeshLib `MeshTopology::addPartByMask`, the Rust prepared-base near-stitch planner now tracks mapped incoming contour edges by raw cut-edge occurrence/source half-edge id instead of ordered vertex pair. This matches MeshLib's undirected edge-id bitset and prevents repeated same-vertex contour edges from falsely excluding each other. Current cube pins move union/intersection missing-neighbor failures into origin/guard buckets while preserving zero record/source-map failures; DifferenceAB now pins remaining prepared-base near-stitch failures at missing-previous=3, origin=8, previous-left=4, and next-right=1, with copied-face left-ring export failures still the next blocker.
- Copied-face left-ring update: after rechecking MeshLib `mapEdge` and `edgePerFace_` selection, copied-edge `emap` seeding now chooses the actual MeshLib source contour side by open-side topology for both flipped and non-flipped reversed contour edges instead of trusting raw cut-edge direction. The record-apply harness also splits copied-edge work into MeshLib-order prepare/finalize phases, so contour targets and prepared source records are seeded before stitched-record rewrite while copied edge records and copied `edgePerFace_` records are finalized afterward. The copied-face selector now models MeshLib's undirected `mapEdge` parity restoration when scanning each source face `leftRing`, prefers the first mapped candidate that already satisfies the Rust MeshLib face-left validator, and falls back to the first mapped candidate so remaining invalid records stay visible. The Rust exporter now mirrors MeshLib's `edgePerFace_[f].left == f` plus full same-face `leftRing` validation, and Rust/PyO3 diagnostics expose face-record-left and same-face-left-ring buckets separately from generic export failures. A guarded face-record refresh now mirrors MeshLib `edgePerFace_` maintenance by repointing stale face records only when an already-valid same-face left ring exists; current cube probes pin zero refreshes, proving the remaining failures are true ring/near-stitch gaps rather than stale face-record handles. This closes prepared-base copied-face export for cube intersection entirely (32/32 exported, zero failures), reduces cube union prepared-base failures from nine to four left-ring closures plus two face-record-left invariant failures (34/40 exported), and reduces DifferenceAB failures from eleven to eight left-ring closures plus one face-record-left invariant failure (27/36 exported). The remaining exact-boolean topology blocker is now narrower: union/DifferenceAB copied-face ring closure plus near-stitch origin/left-guard identity, not missing source maps or global copied-face export.
- Target-side near-stitch candidate update: after rechecking MeshLib `MeshTopology::addPartByMask` `prevNextEdges` generation, the Rust rewrite apply layer now keeps the captured MeshLib-order target half-edge first but also tries same directed face-edge candidates when that captured handle is too narrow. Every candidate still passes the MeshLib shared-origin/open-left/open-right guards before mutation, so failures remain visible instead of being hidden by vertex-pair fallback. Current cube probes move top-level intersection from `origin=5, previous-left=4` to `origin=0, previous-left=9`, proving same-origin target discovery is closed there and the remaining blocker is opening the correct left rings. DifferenceAB prepared-base also moves from `origin=8, previous-left=5, next-right=0` to `origin=6, previous-left=7, next-right=0`; no extra near-stitch command applies yet, so the next slice should focus on MeshLib's target/source ring state before and after copied-record finalization.
- Prepared near-stitch source-index update: the prepared-source `prevNextEdges` path now uses the same MeshLib open-side contour selector for source-edge indices that the vertex-pair path already used. Non-flipped `fromContours` pick the side with no right face and flipped contours pick the side with no left face before walking `from.prev`/`from.next`, matching the MeshLib header contract for `addPartByMask`. The rewrite apply layer also retains all duplicate Rust target endpoint candidates for a MeshLib contour edge behind the primary MeshLib-order target edge, so duplicated triangle-derived half-edges remain available to the guarded near-stitch application without changing the existing aggregate cube pins.
- Prepared-base export diagnostics now expose failed face indices through the Rust/PyO3 parity payload. This keeps the MeshLib guard failures actionable without temporary debug output: current cube union failures are all in the appended incoming copied-face range (`34..39`), while DifferenceAB has mixed base/incoming failures (`5, 7, 14, 19, 23, 25, 28, 30, 32`). The next implementation target is therefore copied incoming face-ring construction plus near-stitch ring opening, not a broad exporter fallback.
- MeshLib contour `setVmap` parity update: after rechecking `MeshTopology::addPartByMask`, Rust contour vertex maps now preserve the first source-vertex assignment instead of overwriting repeated contour endpoints, matching MeshLib's `fromMappedVerts.test_set` behavior. This is Rust-only SDK work and does not touch production Python algorithm integrations. The final-assembly cube union near-stitch path now applies 6 of 10 updates instead of 4 of 10, and the remaining top-level union failures move from `origin=4, previous-left=1, next-right=1` to `origin=0, previous-left=2, next-right=2`. Prepared-base export pins remain unchanged for union and DifferenceAB (`34/40` and `27/36` exported), while intersection still exports `32/32`; the next exact-topology slice should focus on opening the correct source/target left/right rings for `prevNextEdges` and then replacing diagnostic final-assembly counters with MeshLib-prepared topology output.
- Near-stitch failure-detail contract: after rechecking MeshLib `prevNextEdges` generation and final guarded application, the Rust/PyO3 exact-boolean parity payload now exposes structured failure details for both final-assembly and prepared-base record-rewrite paths. Each failed update carries the stitch-pair index, start/end endpoint, incoming operand, source half-edge/source edge identity, previous/next output edge keys, strict-source-identity flag, selected MeshLib guard error, candidate counts, and each candidate pair's origin/left/right guard state. The cube parity tests now pin that the top-level failed-detail list length equals the failed near-stitch count for union/intersection and that prepared-base details stay present, keeping the next ring-opening slice actionable without temporary debug output.
- Captured-open near-stitch target update: after rechecking MeshLib `addPartByMask`, Rust near-stitch candidate application can now use the stored pre-rewrite open-side target snapshot when the later Rust topology state has closed that target edge before the MeshLib `prevNextEdges` update. The normal MeshLib shared-origin/open-left/open-right guards still run before mutation, and failed attempts roll back the temporary open-side state. This moves the cube-overlap top-level union probe from 6/10 to 8/10 applied near-stitch updates while preserving the MeshLib volume/area envelope, exported face rings, and the prepared-base counters. Source candidate lookup also has a guarded same-source-edge fallback for face-key drift, covered by a focused Rust unit test.
- Copied-edge prepass guard: after rechecking MeshLib `addPartByMask` ordering, the Rust rewrite-apply layer keeps the MeshLib-order target/source map prepass scoped to copied-edge translation where it is needed before copied-record finalization. A broader no-copied prepass was tested and rejected because it regressed a two-edge open-contour export case. Source-synthetic commands now seed only target maps during that copied-edge prepass, avoiding duplicate synthetic source half-edges; Rust unit coverage pins both the copied-edge idempotency case and the multi-edge open-contour near-stitch application.
- Copied-face export failure details: after rechecking MeshLib `edgePerFace_` translation (`for fromE : leftRing(from, f) if mapEdge(emap, fromE)`), the Rust/PyO3 prepared-base payload now carries structured export failure details for each failed face: face index, selected face-record edge id, operand, export error, traced left-ring edge ids, origins, and left-face ids. Current cube union still fails the appended incoming faces `34..39`: face `34` is a face-record-left miss with both traced edges having no left face, while face `35` traces a non-closing ring across copied incoming and base face ids. DifferenceAB still has nine mixed base/incoming failures whose traced rings cross several face ids. The next behavior slice should repair MeshLib-style copied-face ring construction and near-stitch ring opening, not add exporter fallbacks.
- Export diagnostic correction: the prepared-base failure-detail trace now walks the actual MeshLib `leftRing` successor (`prev(edge.sym())`) rather than the origin-ring `next(edge)` direction. Rust unit coverage pins this traversal, and PyO3 parity tests assert the emitted left-ring edge/origin/left-face arrays stay aligned. This keeps future copied-face work grounded in the same ring MeshLib uses for `edgePerFace_` validation.
- Paired-coplanar DifferenceAB diagnostic split: after rechecking MeshLib `connectPreparedParts` and `addPartByMask`, the Rust/PyO3 boolean diagnostics now expose paired-coplanar candidate preparePart flags, cut-path side components, output faces, output area/volume, self-intersection count, active-volume delta, and closed/manifold guard buckets from a dedicated binding helper instead of expanding the main diagnostics module. The cube DifferenceAB parity test now pins that the paired candidate is closed/manifold and volume-preserving but has the mathematical slab surface area (`16`) instead of MeshLib's self-intersecting coplanar envelope area (`24`), so it must not be promoted for exact MeshLib parity. A release-like near-stitch experiment that ignored MeshLib's open-left/open-right debug guards was also rejected: it applied seven prepared-base updates but regressed export from `27/36` to `24/36`, confirming the remaining work is source/target ring identity, not a guarded-update bypass.
- Prepared-base rejected experiments: two additional MeshLib-first probes were tested and not kept. First, broadening target fallback candidates to include copied-edge topology handles did not change cube prepared-base counters and only weakened the current candidate contract. Second, building the copied-edge source topology from the full cut mesh before restricting copied records to prepared faces regressed cube union export from `34/40` to `30/40` and DifferenceAB from `27/36` to `23/36`. Keep the next slice focused on the exact MeshLib target/source ring identity around copied-record finalization rather than broad fallbacks or whole-source topology swaps.
- Near-stitch ring-state diagnostics: after rechecking MeshLib `prevNextEdges` and guarded application, failed Rust near-stitch candidate pairs now carry the selected previous/next half-edge ids, candidate source labels (`target-registered`, `target-face-fallback`, `source-halfedge`, `source-edge`, or `topology-fallback`), existing reciprocal links, direct left/right labels, and traced MeshLib left-ring snapshots for the previous edge and the next edge's right side. The PyO3 parity payload asserts those ring arrays stay aligned, giving the next ring-opening slice concrete MeshLib-style evidence without temporary debug output or guard bypasses. A MeshLib-first probe that cleared the whole target left ring instead of the selected target half-edge was also rejected: it did not improve prepared-base near-stitch application or export counts and shifted DifferenceAB failures into worse face-record-left buckets.
- Prepared-base near-stitch source identity update: after rechecking MeshLib `MeshTopology::addPartByMask`, the Rust prepared-source walks now prefer the directed `fromContour[j]` half-edge before falling back to raw source cut-edge index lookup, matching MeshLib's `prevNextEdges` derivation more closely. The rewrite-apply path keeps target near-stitch candidates registered from the pre-rewrite contour state because MeshLib computes `prevNextEdges` before stitched-record mutation, and failed source-derived near-stitch candidate diagnostics now include optional source-halfedge keys so repeated same-vertex contours remain debuggable without temporary logging. A MeshLib-first source vertex-pair exclusion experiment was tested and rejected because MeshLib excludes by mapped undirected edge id, not vertex pair; the probe regressed cube union prepared-base near-stitch application from one update to zero and shifted DifferenceAB failures into worse copied-face guard buckets.
- Prepared-base first-new-vertex and source-side hardening: after rechecking MeshLib `addPartByMask`, the prepared-base Rust apply path now starts incoming copied vertices after the already-built prepared base vertex table, matching MeshLib's `firstNewVert = edgePerVertex_.endId()` during `connectPreparedParts`. The near-stitch source topology and copied-edge source-record topology also prefer the actual MeshLib contour side with the kept prepared face adjacent (`right` open plus `left` prepared for non-flipped copies, `left` open plus `right` prepared for flipped copies) before falling back to the older open-side-only rule. A focused regression now pins that incoming copied vertices do not collide with base virtual vertices. The cube prepared-base counters are unchanged for union/intersection/DifferenceAB, so the remaining blocker is still copied-face ring closure and near-stitch source/target origin identity rather than vertex allocation overlap.
- Copied-edge source-topology modularization and parity hardening: after rechecking MeshLib `addPartByMask`, the copied-edge source topology now lives in its own Rust submodule instead of growing the copied-edge facade, keeping both modules under the architecture line-count guard. The source topology also keeps directed half-edge lookup orientation-specific, explicitly falls back through the symmetric side only when needed, skips copied-edge allocation when either half-edge of the undirected source edge is already in MeshLib's `emap`, and prefers source-index contour records that carry the prepared face side before falling back to raw directed-edge records. Focused Rust, workspace, clippy, and Python architecture/parity gates are green. The cube prepared-base counters remain unchanged, confirming this slice hardened MeshLib identity handling and module structure without masking the remaining copied-face ring closure plus near-stitch source/target origin blocker.
- Near-stitch source-edge diagnostics and module split: after rechecking MeshLib `addPartByMask` `prevNextEdges`, the Rust near-stitch candidate diagnostics now include the optional MeshLib source edge attached to each source-halfedge/source-edge candidate. The PyO3 parity payload exposes those fields for the existing test harness, while the production Python SDK algorithms remain untouched. This makes source identity drift visible without temporary logging: current prepared-base cube failures show both source-halfedge key mismatches, such as a command expecting `[2, 3]` while the registered candidate carries `[2, 1]`, and cases where the source edge agrees but resolves to a different output origin. The near-stitch unit tests now live in a sibling Rust test module, reducing the production `near_stitch.rs` file size and keeping the Rust SDK architecture modular. Focused near-stitch tests, full Rust workspace tests, clippy, maturin rebuild, and Python architecture/golden/parity tests are green.
- Stable prepared-source half-edge key update: after rechecking MeshLib `addPartByMask`, the Rust near-stitch plan now carries a stable source face/edge key in addition to the local source half-edge id. The copied-edge translation stage registers translated `emap` candidates under the same key, and near-stitch lookup tries that MeshLib-style prepared-source key before falling back to the older local-id/source-edge/topology candidates. This fixes the SDK architecture issue where two Rust source-topology builders could assign different local half-edge ids for the same MeshLib `from` occurrence while preserving the old ids for diagnostics. Unit coverage pins that the stable key wins over a stale local-id candidate. The near-stitch candidate helper structs/functions now live in their own Rust submodule, keeping the production near-stitch file under the architecture line-count guard. Current cube prepared-base counters are unchanged (`union 1/9 near-stitch, 34/40 export`; `intersection 0/10, 32/32`; `difference 0/16, 27/36`), but the parity payload now shows stable-key candidates participating in the remaining failures, so the next slice can focus on the true ring-opening/left-ring closure blocker rather than source-id drift. Rust workspace tests, clippy, maturin rebuild, and Python architecture/golden/parity gates are green.
- Copied face-record parity cleanup and command-key diagnostics: after rechecking MeshLib `translate_`, `mapEdge`, and copied `edgePerFace_` translation, the Rust copied-face selector now returns the first mapped edge from the source face `leftRing`, matching MeshLib's `edgePerFace_[fmap[f]] = flip ? e.sym() : e` order instead of preferring a currently valid Rust ring first. Current cube prepared-base counters remain unchanged (`union 1/9 near-stitch, 34/40 export`; `intersection 0/10, 32/32`; `difference 0/16, 27/36`), confirming this was a parity cleanup rather than a guard bypass. The near-stitch failure payload now also exposes the command's stable source-halfedge key face and edge through Rust/PyO3, and Python parity tests assert that shape. A MeshLib-first target source-identity probe was tested and rejected because it regressed prepared-base export (`union 34/40` to `28/40`, `difference 27/36` to `26/36`); the next slice should continue from target/source ring state around copied-record finalization, not broad target identity fallback.
- Prepared mapped-contour record replay: after rechecking MeshLib `addPartByMask` record ordering, the Rust copied-edge finalization now explicitly replays prepared mapped contour source records before appending copied face records, using the Rust `mapEdge` equivalent to preserve MeshLib half-edge parity instead of trusting indexed target orientation. This mirrors the MeshLib requirement that every `fromContour` target record has its `next`, `left`, and symmetric `prev` fields applied before `edgePerFace_` selects the first mapped `leftRing` edge. The prepared-base union probe now exports all copied incoming faces (`40/40`, zero export failures) instead of failing the appended incoming range (`34/40`, six export failures). Intersection remains export-complete at `32/32`; DifferenceAB improves to `28/36` and removes the previous face-record-left mismatch, but still has mixed base/incoming left-ring closure failures. The corrected union topology moves the remaining prepared-base blocker entirely to near-stitch guards (`0/10` applied, buckets `origin=5`, `previous-left=5`, `next-right=0`), which is a cleaner next target than copied-face export fallback. Rust workspace tests, clippy, maturin rebuild, and Python architecture/golden/parity gates are green.
- Prepared fallback source-identity hardening: after rechecking MeshLib `addPartByMask` `prevNextEdges`, prepared-base fallback near-stitch commands no longer trust topology-local numeric source half-edge ids when the true prepared-source walk is missing. The command keeps stable face/edge keys and source-edge identity, but clears the split-topology-local id so apply does not accidentally bind an `emap` candidate from a different Rust source-topology builder. Current cube probes stay export-stable (`union 40/40`, `intersection 32/32`, `DifferenceAB 28/36`) and keep zero record/source-map failures. The diagnostic shift is narrower and more accurate: union prepared-base failures move from `origin=5, previous-left=5, missing-previous=0` to `origin=2, previous-left=5, missing-previous=3`; intersection moves from `origin=8, previous-left=1, missing-previous=1` to `origin=6, previous-left=1, missing-previous=3`; DifferenceAB remains `origin=6, previous-left=7, missing-previous=3`. This confirms the next MeshLib-first slice should repair true target/source ring opening rather than chase stale local half-edge ids.
- Target materialized-topology candidate coverage: after rechecking MeshLib's `prev(e1.sym())` / `next(e1)` target-side near-stitch derivation, the Rust target candidate lookup now includes materialized topology/copy-edge candidates for the same directed endpoint key in addition to registered MeshLib-order target edges and normal face-edge fallbacks. Every candidate still goes through the MeshLib shared-origin/open-left/open-right guards before mutation. Focused coverage pins the copied-edge target candidate path, while cube probes remain unchanged (`union 0/10 near-stitch, 40/40 export`; `intersection 0/10, 32/32`; `DifferenceAB 0/16, 28/36`). That no-op counter result is evidence that the active blocker is not missing materialized target candidates; it is the target/source ring state after stitched-record and copied-record finalization.
- Target registration snapshot diagnostics: after rechecking MeshLib `addPartByMask`, Rust now records the target half-edge state at the moment `prev(e1.sym())` / `next(e1)` candidates are captured, before stitched-record and copied-record finalization. The PyO3 parity payload exposes that snapshot on failed target-registered near-stitch candidates. Current cube union evidence shows a start candidate captured with `left=None` and later failing with `previous_left=34`, which confirms the active blocker is a copied-record/finalization ring mutation after MeshLib-style `prevNextEdges` capture, not missing target discovery or a stale local source half-edge id.
- Copied source-face side key hardening: after rechecking MeshLib `addPartByMask` `prevNextEdges`, Rust now registers stable source-halfedge keys for both valid face sides of a copied source half-edge instead of keeping only the first available side. This preserves MeshLib's start/end and flip-dependent face-side identity for future guarded near-stitch candidate lookup without changing cube prepared-base counters. The code was split into `source_topology/keys.rs` plus focused tests so the source-topology module stays below the architecture line-count guard. A narrow experiment that skipped mapped-contour replay when it would close a previously captured target edge was rejected because it improved one union near-stitch update but regressed prepared-base export (`40/40` to `34/40`) and DifferenceAB export (`28/36` to `27/36`); the next slice should repair target/source ring construction rather than skip MeshLib record replay.
- Copied face-record selector correction: after rechecking MeshLib `edgePerFace_` assignment and the current Rust exporter invariant, the copied-face selector now scans the MeshLib-style source `leftRing` with undirected `mapEdge` parity restoration, prefers the first mapped edge whose current Rust left-ring validator already matches the translated output face, and falls back to MeshLib's first mapped edge only when no valid translated ring exists. Focused Rust coverage pins that validator-first choice. Cube prepared-base counters are unchanged (`union 40/40`, `intersection 32/32`, `DifferenceAB 28/36`), confirming this is a copied-face record hardening step rather than an exporter fallback. A broader source-snapshot near-stitch experiment was tested and rejected: it improved the top-level intersection near-stitch counter by one but regressed prepared-base DifferenceAB export from `28/36` to `26/36`, so the remaining work stays focused on true MeshLib target/source ring state during copied-record finalization.
- Missing near-stitch candidate diagnostics: after rechecking MeshLib `prevNextEdges` and the current failure payload, Rust near-stitch application now records candidate-count diagnostics even when one side of the update has no target/source candidates. Guard failures still carry per-candidate ring snapshots, while missing-candidate failures now expose `previous_candidates` / `next_candidates` with an empty failure list. The PyO3 parity assertions separate guarded failures from missing-side failures and pin that every empty failure list has a zero candidate side. Current cube prepared-base counters remain unchanged (`union 40/40`, `intersection 32/32`, `DifferenceAB 28/36`), but the three missing-previous failures in each prepared-base cube probe are now visible as missing target/source candidate coverage rather than opaque `None` diagnostics. A stricter source-identity-only candidate experiment was tested and rejected because it preserved exports but worsened final-assembly intersection diagnostics and shifted union prepared-base failures back toward origin mismatches.
- Near-stitch identity-attempt preservation: after rechecking MeshLib `addPartByMask` `prevNextEdges`, Rust near-stitch diagnostics now preserve the MeshLib-style target/source lookup attempt when vertex-pair fallback also fails. The fallback diagnostic stays focused on the final failed attempt, but it carries a `fallback_from` summary with the identity attempt label, error, candidate counts, and guard-failure count. This keeps the next behavioral port pointed at the exact missing target/source side instead of losing that evidence to the raw vertex-pair fallback. Focused Rust coverage pins the fallback summary, and PyO3 parity assertions require missing-side prepared-base failures to expose `vertex-pair-fallback` plus the original `identity-target-source` summary.
- Undirected stable source-key fallback: after rechecking MeshLib `mapEdge(emap, eNx)` parity restoration in `addPartByMask`, Rust source candidate lookup now treats stable source-halfedge key edge identity as undirected after the exact face/edge lookup misses. The oriented candidate extender still selects the output half-edge direction that matches the requested near-stitch origin, so this mirrors MeshLib's undirected `emap` with parity restoration instead of broad vertex-pair matching. Focused Rust coverage pins a reversed stable-key source candidate resolving through `sym()`. Current cube prepared-base counters remain unchanged (`union 40/40`, `intersection 32/32`, `DifferenceAB 28/36`), which means the active missing-source failures are not just reversed stable-key drift; the next slice should inspect why the copied source edges for `[6, 7]`, `[10, 8]`, `[11, 14]`, and their intersection/difference analogues are never registered in the prepared-source candidate maps.
- Near-stitch source lookup stage diagnostics: after rechecking MeshLib `mapEdge` behavior and the prepared-base misses, Rust now carries per-side source lookup counts through candidate diagnostics and the PyO3 parity payload. Each identity source lookup records the requested local halfedge, stable face/edge key, source edge, fallback output edge, and candidate counts for exact stable key, same-edge stable key, local halfedge, source-edge map, topology fallback, and final deduped total. Current cube prepared-base misses prove all source lookup stages are zero for the missing source edges: union `[6, 7]`, `[10, 8]`, `[11, 14]`; intersection `[3, 11]`, `[0, 8]`, `[9, 13]`; DifferenceAB `[18, 17]`, `[4, 19]`, `[17, 15]`. The next behavioral slice should focus on the MeshLib `fromCopiedEdges` / `emap` construction parity rather than target registration, source key orientation, or topology fallback.
- Copied-source `emap` status diagnostics: after rechecking MeshLib `addPartByMask` `fromMappedEdges`, `fromCopiedEdges`, `copyEdge`, and `mapEdge`, the Rust copied-edge preparation now records a structured status for every prepared source edge it sees: mapped contour edge, copied edge, missing output vertices, or not present in the prepared source topology. Near-stitch source lookup diagnostics and the PyO3 parity payload now attach this copied-source status to every requested source edge. The cube prepared-base misses all classify as `not-prepared-source-edge` with zero matching statuses, proving the current blocker is not an `emap` insertion/orientation failure for copied prepared edges; the requested near-stitch source edges are absent from the incoming prepared source topology being used for the prepared-base rewrite. The next slice should reconcile MeshLib `fromFaces`/prepared-region selection for open-contour near-stitch source walks before changing candidate fallbacks.
- Prepared source half-edge identity hardening: after rechecking MeshLib `prevNextEdges.emplace_back(... mapEdge(emap, eNx/ePr))`, the Rust prepared source-index endpoint path now carries the local source half-edge id whenever that MeshLib-style source path succeeds, in addition to the stable face/edge key. Focused coverage pins that successful prepared source-path commands retain those ids. The cube prepared-base misses remain unchanged and still show `requested_halfedge=None`, which proves those misses are not losing a valid source half-edge during command construction; they are commands produced by the output-topology fallback after the prepared source-index path cannot find a MeshLib source edge.
- Prepared-source fallback removal: after rechecking MeshLib `addPartByMask` `prevNextEdges`, Rust now skips prepared-base endpoint updates when the prepared source-index walk cannot find an unmapped MeshLib source edge, matching MeshLib's behavior when the source walk returns to an already mapped contour edge. The previous output-topology fallback produced non-MeshLib commands whose source lookup failed as `not-prepared-source-edge`; removing that fallback drops cube prepared-base near-stitch failures from union `10 -> 4`, intersection `10 -> 3`, and DifferenceAB `16 -> 6`, with zero missing previous/next source-edge buckets in all three probes. The remaining failures are real MeshLib guard buckets (`origin` and `previous-left`), so the next slice should continue with target/source ring-state parity instead of adding candidate fallbacks.
- Prepared-source `fromMappedEdges` parity: after rechecking MeshLib `addPartByMask` `fromMappedEdges.test(eNx/ePr.undirected())`, Rust now builds the prepared-source mapped-edge set from the same source-index plus directed-edge contour candidates used by the MeshLib-style source walk, instead of only the narrow raw cut-edge id. This prevents near-stitch commands from being emitted for source edges already classified as mapped contours. Cube prepared-base counters now pin zero near-stitch failures for union and intersection (`40/40` and `32/32` exported), while DifferenceAB drops to two real guard failures (`origin=1`, `previous-left=1`) with its existing eight left-ring export failures unchanged.
- DifferenceAB copied-source output-record diagnostics: after rechecking MeshLib `preparePart`, `connectPreparedParts`, `addPartByMask`, and flipped `translate_`, Rust/PyO3 near-stitch diagnostics now expose the copied source edge's current output half-edge record (`origin`, `left`, `right`, `next`, and `prev`) alongside the existing source-edge lookup status. A MeshLib release-assert style direct near-stitch apply was tested and rejected: it applied the two prepared-base DifferenceAB endpoint updates but worsened export from `28/36` to `27/36`, so no guard bypass was kept. The remaining prepared-base DifferenceAB evidence is now precise: copied source edge `92` for source edge `[3, 15]` has output origin `0` while the target candidate origin is `3`, and copied source edge `89` for source edge `[0, 15]` already has `left/right = 22/25` when the MeshLib open-left guard expects no previous left face. The next slice should inspect MeshLib's two-stage flipped-B prepared-part edge records and target/source ring identity around copied-record finalization, not broad candidate fallback.
- Prepared mapped-source replay diagnostics: after rechecking MeshLib's `addPartByMask` ordering around stitched-record updates, copied-record translation, face-record selection, and final `prevNextEdges` application, Rust now counts mapped-source record replays separately and reports how many replay onto edges already captured as near-stitch target candidates. Union and intersection prepared-base probes stay export-ready, with eight mapped-source replays and all eight landing on near-stitch targets. DifferenceAB remains `28/36` with two guard failures and eight left-ring export failures, but the new counter pins five mapped-source replays and all five landing on near-stitch targets. This keeps the next slice focused on replay target identity and flipped-B contour parity instead of loosening near-stitch guards.
- Copied-source raw prepared-record diagnostics: after another MeshLib pass over `preparePart` and `addPartByMask`, Rust/PyO3 now exposes the raw prepared-source half-edge record (`source_origin`, `source_left`, `source_right`, `source_next_halfedge`, `source_prev_halfedge`) next to the translated output record for copied source lookups. The first DifferenceAB diagnostic pass proved two source-to-output vertex-map drifts without temporary logging: source `[3, 15]` had prepared `source_origin=3` but translated `output_origin=0`, while source `[0, 15]` had prepared `source_origin=0` but translated `output_origin=4` and a closed output left side. A two-stage preflipped-source experiment that built the source topology as already flipped and then copied with no final flip was tested and rejected because it left DifferenceAB at two near-stitch failures and eight export failures while shifting the failed face set. The next fix should target the source-to-output vertex/contour map used for flipped-B copied records, not the raw prepared source walk.
- Copied-source contour vertex-map source identity: after rechecking MeshLib `preparePart` path remapping and `addPartByMask` `setVmap`, Rust copied-edge translation now orients contour vertex-map source edges through the MeshLib-style prepared source half-edge selected by cut-edge index before assigning copied vertices. Focused coverage pins duplicated cut-edge indices so repeated `[u, v]` source edges do not collapse to the same half-edge side. DifferenceAB prepared-base counters remain `28/36` with two guarded near-stitch failures, but the first copied source record is now source/output-origin aligned (`[3, 15]` maps `3 -> 3`); the remaining evidence shifts to the target/source ring identity for the `[0, 15]` endpoint, which still fails MeshLib's guarded near-stitch update.
- Prepared near-stitch contour-map source identity: after rechecking the same MeshLib `preparePart` remapped path and `addPartByMask` `setVmap` ordering, the Rust prepared near-stitch topology now uses the same source-indexed contour orientation as copied-edge translation before building its fresh incoming `vmap`. This removes the stale command/source-map drift where DifferenceAB endpoint commands were generated from un-oriented fallback edges while copied-source lookup used MeshLib-oriented edges. Counters intentionally remain `28/36` with two guarded near-stitch failures, but the failure payload now pins aligned command edges (`start [3,15] -> [3,17]`, `end [0,17] -> [4,13]`), narrowing the next slice to actual MeshLib open-left/open-right ring state after copied-record finalization.
- Mapped-source replay target-state diagnostics: after rechecking MeshLib `addPartByMask` ordering for stitched records, copied records, face records, and final `prevNextEdges` application, Rust/PyO3 now exposes every prepared mapped-source record replay attempt with target before/after half-edge state and skipped reason. Union shows `32` attempts with `8` applied replays, intersection now shows `32` attempts with `0` applied after indexed target registration closes those records earlier, and DifferenceAB shows `30` attempts with `5` applied and `25` skipped. The DifferenceAB blockers prove target edge `63` and target edge `44` were already closed before mapped-source replay attempted to write them, so the next implementation slice should inspect copied-record/finalization ring construction rather than loosening replay skips or near-stitch guards.
- Record-rewrite target-state diagnostics: after rechecking MeshLib `addPartByMask` stitched-record updates before copied-record translation, Rust/PyO3 now exposes each applied record-rewrite target with before/after origin, left/right, next/prev, and the translated source record. DifferenceAB now shows the start blocker directly: target edge `63` is captured as an open near-stitch candidate, then the stitched-record rewrite for stitch pair `1` sets `left=24` and `next=77`, exactly matching the later skipped replay. The end blocker is also clearer: edge `45` (the symmetric side of near-stitch target `44`) is rewritten closed on the left, making target `44` closed on the right before the final near-stitch guard. This confirms the next fix should focus on MeshLib parity for which target-side half-edge is registered for open-contour `prevNextEdges`, not on bypassing the final guard.
- Prepared-base indexed target registration: after rechecking MeshLib `prevNextEdges` construction, Rust now seeds prepared-base contour targets by `this_source_edge_index` so record rewrites can start from the same target-side cut-edge occurrence as MeshLib's `thisContour[j]`. The indexed target path still applies the open-target guard before use; if the indexed half-edge is already closed, it falls back to the existing guarded candidate selection instead of forcing a non-MeshLib rewrite. Focused Rust coverage pins that prepared-base cut-edge indices resolve to open boundary targets, while cube prepared-base export counters remain `union 40/40`, `intersection 32/32`, and `DifferenceAB 28/36` with the same two guarded near-stitch failures. Intersection mapped-source replay diagnostics now show `32` attempts and `0` applied replays because the indexed target record rewrite has already closed those target records; union stays at `32` attempts with `8` applied. The next slice should continue on the remaining DifferenceAB copied-record/finalization ring identity rather than broad target fallback.
- MeshLib contour-order vertex seeding: after rechecking MeshLib `addPartByMask` `setVmap` inside the `thisContours` / `fromContours` loop, Rust now orders copied-edge and near-stitch contour vertex-map seeds by `stitched_edge_paths` pair order before falling back to raw record-command order. This preserves MeshLib's first-assignment behavior when multiple contour edges share a cut vertex and protects future flipped-copy fixes from depending on assembly source-index order. Focused Rust coverage pins the path-order behavior. The current DifferenceAB prepared-base probe remains at `28/36` with the same two guarded near-stitch failures, so the active blocker is still the copied-record/finalization ring state for `[3,15]` / `[0,15]`, not contour seed traversal order.
- Captured-target retry diagnostics: after rechecking MeshLib's final guarded `prevNextEdges` application, Rust now records whether a failed near-stitch candidate temporarily reopened a target side from its captured pre-rewrite snapshot and, if that retry still failed, which guard stopped it next. This keeps the diagnostics MeshLib-first without relaxing `org`, `!left(ePr)`, or `!right(eNx)`. DifferenceAB now proves the start-side blocker is two-stage: target edge `63` can be reopened from its captured `left=None` state, but the copied source candidate `92` then fails `next near stitch edge must not have a right face`; the end-side blocker remains a true source/target origin mismatch. Focused Rust and PyO3 parity assertions pin this evidence for the next copied-record/finalization fix.
- Near-stitch target closure counters: after rechecking MeshLib's stitched-record rewrite order, Rust/PyO3 now count how many previously captured near-stitch target edges are closed on the left or right by record rewrites before the final guarded `prevNextEdges` pass. This is a retained diagnostic hardening step, not a guard bypass: a narrow experiment that reopened copied-source guard sides was rejected because it applied one DifferenceAB near-stitch update but worsened prepared-base export from `28/36` to `27/36`. Current counters pin union at `8` left closures, intersection at `16`, and DifferenceAB at `11` with zero right-closure counters, keeping the next behavior slice focused on copied-record/finalization ring identity.
- Copied-source face-map diagnostics: after rechecking MeshLib `addPartByMask` `fmap`, `translate_`, and final `prevNextEdges` guards, Rust/PyO3 now expose each copied source lookup's direct `source_left`/`source_right` mapped face. The payload keeps copied-record closure visible without temporary logging: source edge `[0,15]` currently maps source faces `1 -> 22` and `4 -> 25`, and the copied output edge remains closed on the left/right as `22/25`. This keeps the next behavior slice pointed at MeshLib prepared-region/source-side face selection and flipped copied-record finalization rather than target discovery or near-stitch guard bypasses.
- Prepared incoming two-stage DifferenceAB update: after rechecking MeshLib `preparePart` and `connectPreparedParts`, Rust now preflips the prepared incoming source topology for the prepared-base connect simulation and then runs the simulated connect with `flipOrientation=false`, matching MeshLib's two-stage DifferenceAB path more closely than the previous raw-source-plus-late-flip shortcut. The same prepared-source mode is used for near-stitch source walks and copied-edge record translation. The resolved start-side failure showed the active blocker had narrowed to the end-side copied source ring for `[0,15]`.
- Prepared-source open-side guard update: after rechecking MeshLib `addPartByMask` `prevNextEdges` generation and final debug guards, Rust now rejects prepared-source previous/next candidates before emitting a near-stitch command when their translated copied-record side would already violate `!left(ePr)` or `!right(eNx)`. DifferenceAB prepared-base near-stitch failures now drop to zero, with `near_stitch_skipped_previous_left_source_edges=1` and `near_stitch_skipped_next_right_source_edges=0` preserving the reason the `[0,15]` endpoint is not emitted. Export remains unchanged at `28/36`, so the remaining exact-parity blocker is copied-face left-ring closure, not target discovery, source lookup, or a near-stitch guard bypass.
- Copied-face export ring diagnostics: after rechecking MeshLib `translate_` face-record assignment through `leftRing(from, f)`, Rust/PyO3 export failure details now include the traced `left_ring_next_edge_ids`, the first repeated edge, and whether the traced ring returned to the face-record start edge. Current DifferenceAB evidence still exports `28/36` faces with eight `left-ring-not-closed` failures, and every failed copied-face ring repeats a non-start edge after leaking into another face id. The next behavior slice should repair copied-face ring closure and face-record side selection rather than changing near-stitch guards, which are now clean for this prepared-base probe.
- Copied-record `mapEdge` parity and edge-state tracing: after rechecking MeshLib `translateNoFlip_`, Rust copied-record next/prev walks now use the same undirected `mapEdge` parity restoration used elsewhere in the port instead of exact directed-map lookups. The DifferenceAB counters remain `28/36`, proving this case already had directed mappings, but the retained export payload now also carries record `next`, record `prev`, and right-face state for every traced failed-ring edge. The current failure evidence points to stitched-record updates assigning incoming copied faces onto open prepared-base contour targets such as edge `26 -> face 35`, `34 -> face 33`, `58 -> face 34`, and `65 -> face 33`; the next repair should focus on MeshLib-equivalent target-face lifecycle for those seam contour records rather than exporter fallback.
- Strict MeshLib target-open lifecycle: after rechecking MeshLib's `assert( !left( e1 ) )` before stitched-record updates, Rust prepared-base contour registration now refuses closed indexed contour targets instead of registering them and later clearing their left face. Record rewrite target selection now also requires an already-open target. This moves DifferenceAB prepared-base export from `28/36` to `32/36`, removes the first four base-face leak failures, and keeps near-stitch failures at zero. The remaining four failures are second-operand copied faces (`22`, `27`, `30`, `31`) whose rings still leak into prepared-base faces, so the next repair should inspect flipped incoming copied-face record translation/finalization rather than base target clearing.
- Exact cut-topology update: boundary-edge contour pieces mixed with real chords or interior spokes are now preserved as path/face topology instead of being treated as failed polygon chords, following MeshLib's `cutEdgesIntoPieces` direction for edge pieces. Boundary polygon triangulation now picks best fan anchors to preserve collinear boundary pieces.
- Exact stitch/topology-splice update: stitch planning now keeps MeshLib `resultCut`-indexed matched pairs and complete path groups even when a later indexed cut path remains incompatible, retains safe indexed pair evidence across mismatched contour lengths, and falls back to wider edge matching when mismatched indexed contours would hide additional seam pairs. Assembly keeps those conflict-free partial seam pairs as splice candidates and reuses conflict-free contour endpoint mappings before full stitch compatibility. Surviving mapped stitch path segments are remapped into the filtered emitted-source index space and split around unmapped or non-contiguous pairs before topology-splice apply, remaining incompatible edges fall back to individual boundary-stitch verification/materialization, duplicate output face groups are reported as a hard parity blocker, and stitch pairing uses a widened endpoint tolerance for MeshLib-style quantized cut vertices.
- Rust/Python SDK architecture hardening: Rust core `lib.rs` is now a small public facade, with implementation split into `analysis`, `types`, `math`, `mesh`, `grid`, `spatial`, `voxel`, `deform`, `deform_smooth`, `deform_target`, `distance`, `hollow`, `jewelry`, `manufacturability`, `materials`, `repair`, `resize`, `topology`, and test modules so future SDK kernels do not continue accumulating in a monolithic file. The spatial module is also decomposed into private AABB query, BVH, closest-point, ray traversal, float triangle-predicate, exact integer-predicate, exact-intersection, exact-contour, exact one-mesh contour, exact cut-preplan, exact cut-mutation, exact component-classification, exact boolean-assembly, exact boolean-diagnostics, exact stitch-plan, and winding submodules. The MeshLib-style splice-apply output topology is now split further so stitched-record rewrite logic lives in a private `output_topology/rewrite_records.rs` child module instead of growing the topology storage module. The PyO3 binding crate now mirrors that domain split with a small `lib.rs` facade and `analysis`, `mesh`, `aabb_tree`, `spatial`, `voxel`, `deform`, `deform_smooth`, `deform_target`, `distance`, `hollow`, `jewelry`, `manufacturability`, `repair`, `resize`, `boolean`, `topology`, and conversion modules. The new `boolean` binding is intentionally test-facing: it exposes `exact_boolean_mesh` for MeshLib oracle parity without adding a public Python SDK exact-boolean wrapper. The Python Rust accelerator boundary keeps `accelerators.rust` as an import-stable facade while private `_rust_*` modules own loader policy, analysis, manufacturability, AABB tree, mesh, spatial, voxel, deformation, distance, hollow, resize, and topology wrappers.
- Rust performance hardening: mesh stats, self-intersection, closest-point, winding-number, signed-distance, raycast, ray-thickness, and SDF-grid workloads now use parity/performance gates; uploaded-fragment gates cover portable processed ring/pendant components for mesh stats, closest-point, batched raycast, and ray-thickness behavior; the heavier distance-field workloads use Rayon-backed parallel iteration, and the PyO3 binding releases the Python GIL during long-running kernel calls.
- Core mesh stats: Rust-owned bbox, area, volume, boundary edges, and connected components behind a thin Python compatibility wrapper; lower-level mesh utilities still provide vertex normals and adjacency for modules not yet ported.
- Health V0: Rust-owned closure, boundary-loop hole count, boundary edge count, non-manifold edge count, and optional self-intersection counting, with Python now acting as a required-Rust compatibility wrapper rather than the primary implementation path.
- Repair V0: close/duplicate vertex merge, degenerate face removal, unreferenced vertex cleanup, face-orientation repair for negative signed-volume meshes, ordered boundary loops, and simple planar hole filling with structured reports.
- Repair V0: Rust-owned low-risk cleanup, outward orientation, ordered boundary loops, planar centroid-fan hole filling, MeshLib-style service hole triangulation over existing boundary vertices, and SDF/voxel rebuild pass for damaged or non-manifold inputs with explicit safety-offset support and structured before/after topology reports behind wrapper-only Python compatibility modules.
- Spatial/compare V0: Rust-owned cached AABB tree handles and broad-phase candidate traversal, Rust-owned flat-BVH closest point/point-distance with face IDs, Rust-owned all-hit/one-shot/batched raycast, ray-thickness traversal, Rust-owned self-intersection candidate-pair traversal and triangle predicate wrappers, Rust-owned winding-number/ray inside-outside classification, topology-aware signed-distance defaults for open/non-manifold/self-intersecting meshes, Rust-owned nearest-vertex/nearest-surface compare fields, Rust-owned signed-distance compare fields/summaries, Rust-owned service-clamped version compare fields, and Rust-owned production-style version compare summaries behind required-Rust compatibility wrappers.
- Voxel/SDF V0: signed-distance grid sampling with explicit raw-winding opt-in for repair workflows, coarse cell-volume estimation, aligned-grid union/intersection/difference with optional Rust value composition, offsets, shell fields, conservative occupied-cell extraction, marching-tetrahedra isosurface extraction with optional Rust raw extraction and forced-Rust face-orientation parity, resident Rust SDF offset/shell/boolean marching, phase-shifted boolean sampling for grid-aligned cutters, and Rust-owned mesh-in/mesh-out voxel offset/shell/boolean/global-thicken operations behind Python compatibility wrappers as the foundation for future hollowing/booleans.
- Voxel refinement V0: SDF interpolation, SDF gradients, optional Rust-accelerated Laplacian smoothing, optional Rust-accelerated projection back to the sampled iso-surface, a resident Rust smoothing-plus-projection refinement path, and topology-aware fallback when refinement would introduce non-manifold edges or self-intersections.
- Thickness V0: Rust-owned bidirectional vertex-normal ray thickness, shrinking-sphere inspired in-sphere thickness, service-style combined thickness that mirrors the current MeshLib service composition, and threshold summaries behind required-Rust Python compatibility wrappers. This is now the in-house contract to harden against MeshLib goldens.
- Artifact contract V0: thickness and compare NPZ helpers that preserve the current overlay key shapes and vertex-count alignment checks.
- Jewelry V0: Rust-owned ring measurement and Rust-owned semantic region detection, with Python wrappers limited to converting native payloads into SDK dataclasses.
- Jewelry hollow planning V0: Rust-owned fixed service hollowing through the SDK SDF shell path, Rust-owned protected-region hollow scale fields, Rust-owned weighted inner-offset previews, Rust-owned protected hollow mesh generation through the SDK voxel difference path, Rust-owned adaptive/protected-adaptive target-weight hollow search through the SDK voxel shell path, Rust-owned drain-hole plans, Rust-owned closed cutter meshes, and voxel drain subtraction composed through the SDK voxel boolean path.
- Manufacturability V0: Rust-owned material weight table, recommendation generation, health score, semantic-region checks, thickness/health aggregation, and export-readiness report behind a wrapper-only Python compatibility module.
- Deformation V0: Rust-owned radial resize/ring resize with protected-vertex falloff, Rust-owned vertex-target nearest-distance helper for protected falloffs, Rust-owned local thicken/local scoop/local thicken-to-minimum/local smooth wrappers, service-parity global thickening for the current MeshLib `generalOffsetMesh` operation contract, service-parity global Taubin smoothing for the current smooth operation contract, typed brush strokes with selected-vertex masks, protected vertices, Rust-owned region-derived masks from semantic ring regions, required-Rust brush composition wrappers, and forced-Rust weighted local-smooth parity for future precomputed-weight pipelines.
- Reference adapters for trimesh IO and MeshLib health, per-vertex thickness, signed compare, offset, and boolean benchmarking.
- Stored golden JSON reference metrics for deterministic fixtures, including thin-wall, hollowed-ring, and pendant/non-ring families, large app sample meshes for IO/stats scalability coverage, checked-in compact uploaded processed-sample fragments plus optional local full-upload manifests for app-pipeline artifact drift coverage, checked-in MeshLib-derived scalar NPZ artifacts for thickness and signed compare contracts, checked-in MeshLib operation metric goldens for cube offset/boolean, ring offset, real ring-cutter difference/intersection booleans, direct/generated-shell/thicker-shell drain-hole cutter subtraction, ring-with-head local cutter booleans, prong-like multi-cutter difference/union/intersection booleans, real pendant cutter booleans, uploaded-fragment offset envelopes, uploaded-fragment SDF-rebuild envelopes, stable post-rebuild uploaded-fragment boolean envelopes, and uploaded-fragment bbox-cutter difference envelopes for ring and pendant fragments, plus programmatic parity tests against trimesh and live MeshLib health, thickness, signed-compare, offset, and boolean references.
- Rust accelerator plan and implementation seed: PyO3/maturin + NumPy boundary, pure Rust core crate, thin Python binding crate with GIL release, benchmark crate, feature-flagged rollout, initial stats/manufacturability-report/AABB-tree/self-intersection/closest-point/point-distance/signed-distance/winding/one-shot-and-batched-raycast/ray-and-in-sphere-thickness/SDF-grid/SDF-boolean/resident-SDF-boolean-offset-shell-marching/product-facing-voxel-mesh-ops/service-hollow/SDF-rebuild-repair/SDF-projection/resident-SDF-refinement/nearest-distance/hollow-planning/protected-hollow-mesh/adaptive-hollow/protected-adaptive-hollow/falloff/local-offset/local-thicken-to-minimum/brush-composition/brush-region-mask-planning/global-Taubin-smoothing/weighted-and-resident-seeded-Laplacian-smoothing/marching-extraction/face-orientation kernels, Rayon-backed hot loops, and performance gates are now part of the migration plan.
- Rust-vs-Python performance budget tests now cover mesh stats, self-intersection, closest-point, point-distance, winding-number, signed-distance, raycast, ray-thickness, SDF-grid, default voxel offset/shell/boolean mesh extraction, SDF-projection, resident SDF refinement, vertex-target nearest distances, falloff weights, local thicken/scoop offsets, local brush composition, masked/protected brush composition, global Taubin smoothing, resident seeded smooth, and marching-extraction accelerator paths on deterministic generated jewelry fixtures, plus portable uploaded-fragment budgets for processed ring/pendant mesh stats, pendant closest-point queries, ring batched raycasts, and ring ray-thickness. The pure Rust SDF-boolean, local thicken-to-minimum, precomputed weighted local-smoothing, and face-orientation kernels are bench-smoked separately and kept out of auto mode until grid/topology/field state can remain resident in Rust without Python boundary copies.

Run from `meshinspector-backend`:

```bash
uv run --extra dev pytest tests/test_geometry_sdk_core.py tests/test_geometry_sdk_architecture.py tests/test_geometry_sdk_jewelry.py tests/test_geometry_sdk_brushes.py tests/test_geometry_sdk_fixture_families.py tests/test_geometry_sdk_real_samples.py tests/test_geometry_sdk_uploaded_samples.py tests/test_geometry_sdk_accelerators.py tests/test_geometry_sdk_artifacts.py tests/test_geometry_sdk_parity.py tests/test_geometry_sdk_engine.py tests/test_geometry_sdk_repair.py tests/test_geometry_sdk_spatial.py tests/test_geometry_sdk_thickness.py tests/test_geometry_sdk_manufacturability.py tests/test_geometry_sdk_goldens.py tests/test_geometry_sdk_voxel.py tests/test_geometry_sdk_performance.py -q
```

## Immediate Next Steps

1. Continue exact boolean work from the Rust exact-kernel, contour, cut, fill-plan, cut-boundary fill application, classification, assembly/provenance, stitch-plan, topology-splice readiness planner, internal half-edge splice/contour-stitch primitives, topology-splice apply planner, and pipeline diagnostics foundation. The current Rust path now includes MeshLib-style result-cut paths, face-lone retry/fallback, orientation-normalized cut-side solving, candidate-only same-oriented coplanar union/intersection selection, and cube-overlap paired candidates that promote only when closed, manifold, non-duplicated, stitch-compatible, preparePart-dividable, and volume/surface-envelope preserving.
2. Resolve coplanar exact-difference parity next. The current fallback matches MeshLib's cube-overlap difference volume/surface/bbox envelope but is open/nonmanifold; the paired candidate is closed/manifold but intentionally not promoted because its mathematical slab surface area does not match MeshLib's self-intersecting coplanar envelope. The next slice should continue from MeshLib `prevNextEdges` target/source ring identity and copied-face left-ring closure; do not bypass the MeshLib open-left/open-right guards because that regression was tested and rejected. Keep the MeshLib-backed production service path untouched until this parity gate is green.
3. Continue broadening real jewelry operation goldens beyond the current ring, drain-hole, generated-shell, thicker-shell, ring-with-head, prong-like, pendant, and uploaded-fragment cutter envelopes, with priority on thin-shell hardening and more uploaded sample components.
4. Expand benchmark thresholds for Rust self-intersection, raycast, and larger app samples beyond the current generated-fixture and uploaded-fragment stats/closest/thickness gates.
5. Promote the next voxel and deformation pipeline stages into Rust once the current dense-grid envelopes are stable, next targeting resident refined voxel extraction, richer brush mask falloffs, and larger jewelry sample benchmarks.
6. Continue hardening the flat Rust BVH around larger real meshes, near-surface winding ambiguity, and future boolean/repair source-map requirements.
7. Harden extracted voxel meshes for production hollowing/boolean outputs: sharper feature preservation, marching-output robustness for drain subtraction, tighter MeshLib envelopes, and real jewelry fixtures.
8. Only after operation parity is stable, introduce a feature-flagged service adapter for one low-risk operation such as ring measurement or regions.

## Success Criteria

The migration is successful when:

- active service code has no direct MeshLib/trimesh/scipy geometry imports after the migration phase, not during the parallel SDK buildout
- every current operation still produces the same API/job/artifact contract
- in-house modules can be enabled operation-by-operation
- MeshLib can be retained as a fallback during development but is no longer the design center
- new jewelry-specific behavior can be customized inside `zennah_geometry` without changing product service code
