# MeshInspector Feature Ground Truth

## Purpose

This document is the current ground-truth description of what `meshinspector` actually implements today.

It is intentionally different from architecture proposals or future plans:

- it describes shipped behavior in the current repository
- it separates implemented features from planned or partial features
- it acts as the product-level guide for the system

Use this together with [MeshInspector Technical Reference.md](./MeshInspector%20Technical%20Reference.md), which documents the API, storage model, runtime design, and major technical contracts.

## Product Scope

`meshinspector` is a manufacturability-focused mesh workspace for jewelry, optimized first for rings generated from mesh-first pipelines such as image-to-3D tools.

The current system is built around these goals:

- take an uploaded mesh and normalize it into a stable processing pipeline
- inspect whether the geometry is ready for manufacturing
- measure ring-specific properties such as size, inner diameter, and thickness
- apply versioned manufacturing edits such as repair, resize, hollowing, thickening, scoop, and smoothing
- compare derived versions and preserve a reversible editing history
- support both guided manufacturing workflows and more advanced local edits

## What Exists Today

## 1. Versioned Model Pipeline

The project no longer treats uploads as one-off processed files. It now uses a versioned model pipeline.

Current capabilities:

- upload creates a `model`
- every processing step creates a new immutable `model version`
- each version stores its own artifacts, analysis snapshots, and job history
- older versions can be reopened, compared, and restored as a new branch

Practical result:

- edits are reversible
- derived states are auditable
- manufacturing review can happen against a stable version history instead of a mutable working file

## 2. Upload, Ingest, and Normalization

The ingest system preserves the source upload and produces normalized working assets.

Implemented behavior:

- accepts `glb`, `gltf`, `obj`, `stl`, and `ply`
- preserves the original upload as an artifact
- produces a normalized processing mesh in `PLY`
- produces low- and high-detail `GLB` preview artifacts
- produces a manufacturing `STL` export path
- triggers baseline manufacturability analysis automatically

Important product assumption:

- the normalized mesh is the working geometry for processing
- STL is no longer treated as the only canonical internal format

## 3. Manufacturability Analysis

Each ready version can expose a manufacturability snapshot.

Current snapshot categories:

- mesh health
- ring dimensions
- thickness summary
- material weight table
- semantic regions
- recommendations
- export readiness

### Mesh health

Current outputs include:

- whether the mesh is closed
- hole count
- self-intersection count
- disconnected shell count
- an overall health score

### Ring dimensions

Current outputs include:

- detected ring axis
- ring-axis confidence
- estimated US ring size
- inner diameter in millimeters
- minimum and maximum band width
- head height
- bounding box
- a `needs_axis_confirmation` flag when confidence is low

### Thickness analysis

Current outputs include:

- minimum, average, and maximum thickness
- thickness violation count
- threshold used for evaluation
- scalar field artifact for heatmap rendering

### Material weight estimation

The system calculates predicted weight from mesh volume for:

- `gold_24k`
- `gold_22k`
- `gold_18k`
- `gold_14k`
- `gold_10k`
- `silver_925`
- `platinum`

### Semantic regions

The current ring-oriented region classifier can label:

- `inner_band`
- `outer_band`
- `head`
- `ornament_relief`
- `unknown`

Each region can expose:

- coverage percentage
- centroid
- thickness summary
- violation count
- whether it is protected by default
- which operations are allowed on it

## 4. Guided Manufacturing Workflow

The default workspace is built around guided manufacturing actions rather than only low-level tools.

Implemented guided actions:

- `Auto Repair`
- `Fit To Size`
- `Reduce Weight`
- `Prepare For Casting`
- `Make Manufacturable`

### Auto Repair

Purpose:

- heal obvious manufacturability blockers before later edits

Current behavior:

- runs as a versioned background job
- creates a new derived version when successful

### Fit To Size

Purpose:

- resize a ring to a target production size

Current behavior:

- uses measured ring properties rather than assuming source size
- supports manual axis override
- includes preserve-head weighting so decorative regions are less aggressively deformed

### Reduce Weight

Purpose:

- lower mass while trying to keep detail in protected regions

Current behavior:

- runs target-weight hollowing
- prioritizes safe interior hollowing
- preserves protected regions by default

### Prepare For Casting

Purpose:

- create a castable hollow output

Current behavior:

- performs protected hollowing
- adds conservative drain holes through the inner band

### Make Manufacturable

Purpose:

- run a single guided pipeline

Current behavior:

- orchestrates repair, sizing, optimization, and validation into one versioned job flow

## 5. Advanced Editing Tools

The system also includes a more explicit local-edit toolset for expert workflows.

Implemented advanced actions:

- global or selective resize
- protected hollowing
- protected hollowing with drain holes
- thicken violations
- region-targeted thickening
- batch multi-region thickening
- scoop
- region-targeted smoothing
- batch multi-region smoothing
- conservative global smoothing

### Protected hollowing

Current behavior:

- not just validation
- builds a weighted hollow shell
- hollows safe regions more aggressively
- preserves protected regions more conservatively
- supports both fixed-thickness and target-weight modes

### Drain-hole planning

Current behavior:

- available through hollow requests
- places two conservative radial drain cylinders using ring semantics
- subtracts them from the hollow shell

Current limitation:

- drain-hole placement is backend-driven and conservative
- there is no interactive drain-hole authoring UI yet

### Thickening

Current modes:

- `global`
- `violations_only`
- `selected_region`
- `selected_regions`

### Scoop

Current behavior:

- localized carve/deformation
- only allowed on scoop-safe semantic regions
- enforces minimum thickness constraints

### Smoothing

Current behavior:

- local smoothing on a primary region
- batch smoothing across multiple selected regions
- optional conservative global smoothing pass

## 6. Viewer and Inspection Workspace

The main viewer is now an edit-oriented manufacturing workspace.

Implemented viewer capabilities:

- low-detail and high-detail preview assets
- normalized mesh overlay path
- wireframe mode
- region overlay
- thickness heatmap
- compare heatmap
- section clipping
- region picking
- additive multi-region selection
- contour extraction from the active section plane
- dimension guide rendering for active section

### Region picking

Current behavior:

- click selects the nearest semantic region
- Shift/Ctrl/Cmd click supports additive multi-selection

### Section inspection

Current behavior:

- section plane can be enabled and moved
- contour geometry is extracted from triangle-plane intersections
- section width and depth are measured
- contour segments can be exported to SVG with dimension annotations

### Section presets

Current presets:

- centerline
- region-derived presets such as `inner_band`, `head`, `ornament_relief`, and `outer_band` when present

## 7. Inspection Panels

The workspace currently includes these major panels:

- Manufacturability panel
- Guided workflow panel
- Advanced edit panel
- Overlay legend and inspection panel
- Job activity panel
- Compare panel
- Version history panel

### Manufacturability panel

Current responsibilities:

- show readiness and manufacturing metrics
- expose material-aware weight data
- surface recommendations
- expose axis confirmation controls

### Overlay legend and inspection panel

Current responsibilities:

- overlay legend for thickness or compare views
- section-plane controls
- section presets
- selected-region readouts
- saved inspection snapshots
- section SVG export

### Compare panel

Current responsibilities:

- choose compare target version
- toggle compare overlay
- show compare summary
- show compare cache history

### Version history panel

Current responsibilities:

- list versions for the active model
- open a prior version
- compare a prior version
- restore a prior version as a new branch

## 8. Jobs, Progress, and Background Processing

Heavy operations are asynchronous.

Implemented behavior:

- ingest runs as a job
- geometry operations run as jobs
- job state can be polled
- job events can be streamed
- frontend shows a live activity panel during execution

This is a key shift from the original synchronous demo-style flow.

## 9. Versioning, Branching, and History

The current version model supports manufacturing review and rollback.

Implemented behavior:

- each operation creates a new version
- prior versions remain immutable
- any prior version can be restored as a new branch
- compare works across versions
- inspection states can be saved against a specific version

Current limitation:

- history is presented as a list/timeline, not a full branch graph UI

## 10. Saved Inspection States

Inspection state can be saved and reloaded per version.

Saved state currently includes:

- axis mode
- manual axis
- section enabled state
- section position
- selected region
- selected regions
- heatmap enabled state
- compare enabled state
- compare target version

This supports repeatable inspection and manufacturing review.

## 11. Compare and Change Analysis

The system supports version-to-version geometric comparison.

Current capabilities:

- signed-distance compare against another version
- compare overlay rendering in the viewer
- compare summary values
- compare cache listing for previously generated comparisons
- reuse of cached compare artifacts instead of recomputing every time

## 12. Runtime and Storage Features

The current backend supports a more production-oriented runtime model.

Implemented platform features:

- Postgres-backed metadata model
- Alembic migrations
- Supabase-compatible Postgres configuration
- psycopg3 database URL normalization
- local object storage driver
- S3-compatible object storage driver
- Celery-compatible queue path
- database-backed development queue path

Important current development behavior:

- local object storage can be used with `OBJECT_STORE_DRIVER=local`
- development can use a database-backed queue instead of Redis

## 13. Legacy Compatibility Layer

The repository still contains older API routes from the earlier demo implementation.

These legacy surfaces still exist for compatibility or migration:

- upload endpoints
- legacy analysis endpoints
- legacy health endpoints
- legacy processing endpoints
- legacy preview/download routes

They should not be treated as the primary product API anymore. The versioned `/api/models`, `/api/versions`, `/api/jobs`, and operation routes are the current product surface.

## Current Gaps and Partial Features

The following items are important but are not yet fully implemented as production-grade features.

## 1. Semantic understanding is still heuristic

Current limitation:

- region classification is rule-based and ring-oriented
- it is not yet CAD-grade or learned from a richer jewelry model

## 2. Hollowing is improved but still not fully designer-driven

Current limitation:

- protected hollowing is real
- interactive user-directed hollow region painting or weighting is not implemented

## 3. Advanced deformation is still limited

Not implemented yet:

- comfort-fit shaping
- free-form deformation tools
- Laplacian editing tools
- sculpt-like local shape editing beyond scoop, thicken, and smooth

## 4. Drain-hole control is limited

Current limitation:

- drain holes are conservatively auto-planned
- there is no direct UI for placement, radius, count, or orientation authoring

## 5. History UX is not a full branch graph

Current limitation:

- versions and branching exist
- the UI is still a practical list/timeline instead of a visual branch tree

## 6. Production hardening is not complete

Still needed:

- golden-mesh regression tests
- artifact cleanup and retention policies
- deeper deployment hardening
- fuller operational documentation

## Ground-Truth Rule

When this document and older planning docs disagree, this document should be treated as the current product truth unless the code has changed after the document date.

If the product changes, this file should be updated in the same change set as the feature work.
