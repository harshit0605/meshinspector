# MeshInspector Technical Reference

## Purpose

This document describes the current technical shape of the `meshinspector` project.

It focuses on:

- backend API surface
- data model
- artifact model
- job model
- viewer and frontend workspace contracts
- runtime and storage behavior
- current compatibility boundaries

Use this together with [MeshInspector Feature Ground Truth.md](./MeshInspector%20Feature%20Ground%20Truth.md).

## 1. Current System Shape

`meshinspector` is split into two main applications:

- `meshinspector-backend`
- `meshinspector-frontend`

The backend is the geometry authority. Mesh editing and manufacturability logic live there. The frontend is a viewer and workflow workspace that drives versioned operations.

## 2. Backend Architecture

The current backend stack is:

- `FastAPI` for the API layer
- `SQLAlchemy 2` for persistence
- `Alembic` for migrations
- `Postgres` as the intended metadata database
- `psycopg3` URL normalization for hosted Postgres providers such as Supabase
- object storage abstraction with local and S3-compatible drivers
- asynchronous jobs through either:
  - Celery-compatible dispatch
  - database-backed development queue

### Important runtime behavior

- `DATABASE_URL` is normalized to `postgresql+psycopg://...` when needed
- `DIRECT_URL` is used for migration-safe direct database access
- `OBJECT_STORE_DRIVER=local` stores artifacts in local storage
- `OBJECT_STORE_DRIVER=s3` uses the configured S3-compatible bucket
- `QUEUE_BACKEND=database` avoids Redis in development
- `QUEUE_BACKEND=celery` remains the production-oriented queue path

Primary configuration is in:

- `meshinspector-backend/core/config.py`
- `meshinspector-backend/storage/object_store.py`

## 3. Persistent Domain Model

Current persistent tables:

- `models`
- `model_versions`
- `model_artifacts`
- `analysis_snapshots`
- `jobs`
- `job_events`
- `operation_requests`
- `dev_task_queue`

### `models`

Purpose:

- logical container for one uploaded source asset

Key fields:

- `id`
- `source_filename`
- `source_type`
- `created_at`

### `model_versions`

Purpose:

- immutable derived states of a model

Key fields:

- `id`
- `model_id`
- `parent_version_id`
- `operation_type`
- `operation_label`
- `status`
- `created_at`

### `model_artifacts`

Purpose:

- all files attached to a version

Key fields:

- `id`
- `version_id`
- `artifact_type`
- `mime_type`
- `storage_key`
- `size_bytes`
- `metadata_json`
- `created_at`

### `analysis_snapshots`

Purpose:

- manufacturability and inspection snapshot storage

Key fields:

- `id`
- `version_id`
- `snapshot_type`
- `payload_json`
- `created_at`

### `jobs`

Purpose:

- operation execution record

Key fields:

- `id`
- `version_id`
- `operation_type`
- `status`
- `progress_pct`
- `error_code`
- `error_message`
- `started_at`
- `finished_at`
- `created_at`

### `job_events`

Purpose:

- streamed progress and step messages for a job

### `operation_requests`

Purpose:

- audit record of submitted operation payloads

### `dev_task_queue`

Purpose:

- database-backed development queue for environments that do not want Redis during local work

The model definitions are in:

- `meshinspector-backend/domain/models.py`

## 4. Artifact Model

Current artifact types used by the system include:

- `original_upload`
- `normalized_mesh_ply`
- `preview_glb_low`
- `preview_glb_high`
- `manufacturing_stl`
- `analysis_thickness_npz`
- `analysis_compare_npz_<other_version_id>`
- `analysis_regions_json`
- inspection snapshots in `analysis_snapshots`
- additional operation-generated files as needed

### Artifact strategy

The current design separates concerns:

- original upload is preserved
- normalized `PLY` is used for processing and semantic overlays
- `GLB` is used for interactive viewing
- `STL` is used as the manufacturing export surface
- `NPZ` and `JSON` artifacts back overlays and analysis

## 5. API Surface

## 5.1 Current primary API

### Models

- `POST /api/models`
  - upload a new source mesh
  - creates model, initial version, and ingest job

- `GET /api/models/{model_id}`
  - fetch model metadata

- `GET /api/models/{model_id}/versions`
  - list immutable versions for a model

### Versions

- `GET /api/versions/{version_id}`
  - fetch version metadata, artifacts, and latest snapshot

- `GET /api/versions/{version_id}/manuf`
  - fetch manufacturability snapshot payload

- `GET /api/versions/{version_id}/viewer`
  - fetch viewer manifest for the frontend workspace

- `GET /api/versions/{version_id}/compare-cache`
  - list cached compare artifacts for this version

- `POST /api/versions/{version_id}/branch`
  - create a new version by restoring or branching from an existing one

- `GET /api/versions/{version_id}/inspection-snapshots`
  - list saved inspection views

- `POST /api/versions/{version_id}/inspection-snapshots`
  - persist a saved inspection view

- `GET /api/versions/{version_id}/overlays/thickness`
  - fetch thickness overlay payload

- `GET /api/versions/{version_id}/overlays/compare/{other_version_id}`
  - fetch compare overlay payload, using cache when present

### Artifacts

- `GET /api/artifacts/{artifact_id}`
  - download or stream the artifact through the backend

### Jobs

- `GET /api/jobs/{job_id}`
  - fetch job status

- `GET /api/jobs/{job_id}/events`
  - stream job events over server-sent events

### Operations

- `POST /api/versions/{version_id}/repair`
- `POST /api/versions/{version_id}/resize`
- `POST /api/versions/{version_id}/hollow`
- `POST /api/versions/{version_id}/thicken`
- `POST /api/versions/{version_id}/compare`
- `POST /api/versions/{version_id}/make-manufacturable`
- `POST /api/versions/{version_id}/scoop`
- `POST /api/versions/{version_id}/smooth`

All of these create jobs. They do not perform synchronous heavy geometry work in the request/response cycle.

Primary router files:

- `meshinspector-backend/api/routers/models.py`
- `meshinspector-backend/api/routers/versions.py`
- `meshinspector-backend/api/routers/operations.py`
- `meshinspector-backend/api/routers/jobs.py`

## 5.2 Compatibility and legacy API

Older routes still exist in `meshinspector-backend/api/routes`.

Current legacy endpoints still present:

- `POST /api/upload`
- `GET /api/analyze/{model_id}`
- `GET /api/health/{model_id}`
- `GET /api/thickness/{model_id}`
- `POST /api/repair/{model_id}`
- `POST /api/process`
- `GET /api/download/{model_id}/{format}`
- `GET /api/preview/{model_id}`

These should be treated as migration compatibility surfaces, not the primary technical contract.

## 6. Core Schema Contracts

The main current request/response contracts are defined in:

- `meshinspector-backend/domain/schemas.py`

### Materials

Supported material enum values:

- `gold_24k`
- `gold_22k`
- `gold_18k`
- `gold_14k`
- `gold_10k`
- `silver_925`
- `platinum`

### Operations

Current operation enum values:

- `ingest`
- `repair`
- `resize`
- `hollow`
- `thicken`
- `scoop`
- `smooth`
- `compare`
- `make_manufacturable`

### Manufacturability snapshot contract

The manufacturability snapshot currently includes:

- `version_id`
- `mesh_health`
- `dimensions`
- `material_weight`
- `thickness`
- `regions`
- `recommendations`
- `export_ready`

### Viewer manifest contract

The viewer manifest currently includes:

- `version_id`
- `preview_low_url`
- `preview_high_url`
- `normalized_mesh_url`
- `thickness_artifact_url`
- `region_artifact_url`
- `bounding_box`
- `default_material`
- `available_overlays`
- `region_manifest`
- `measurements_summary`
- `can_edit`
- `needs_axis_confirmation`

### Operation request contracts

#### Resize

Current request fields:

- `target_ring_size_us`
- `axis_mode`
- `manual_axis`
- `preserve_head`

#### Hollow

Current request fields:

- `mode`
- `material`
- `wall_thickness_mm`
- `target_weight_g`
- `min_allowed_thickness_mm`
- `protect_regions`
- `add_drain_holes`

#### Thicken

Current request fields:

- `mode`
- `min_target_thickness_mm`
- `region_id`
- `region_ids`
- `smoothing_pass`

#### Scoop

Current request fields:

- `region_id`
- `depth_mm`
- `falloff_mm`
- `keep_min_thickness_mm`

#### Smooth

Current request fields:

- `region_id`
- `region_ids`
- `iterations`
- `strength`
- `global_mode`

#### Make manufacturable

Current request fields:

- `material`
- `target_ring_size_us`
- `target_weight_g`
- `min_allowed_thickness_mm`

## 7. Backend Geometry Feature Map

This section describes what the backend is responsible for today.

### Ingest and normalization

Current responsibilities:

- validate file extension and size
- create model/version/job records
- preserve upload source
- normalize geometry into processing artifacts
- generate preview assets
- trigger baseline analysis

### Manufacturability analysis

Current responsibilities:

- mesh health summary
- ring measurement
- material weight estimation
- thickness field generation
- semantic region generation
- recommendation generation

### Repair

Current responsibilities:

- run geometry cleanup and healing as a versioned job

### Resize

Current responsibilities:

- use measured ring dimensions
- honor manual axis overrides
- bias deformation away from protected head/detail regions when `preserve_head` is enabled

### Hollow

Current responsibilities:

- fixed-thickness hollowing
- target-weight hollowing
- protected-detail weighted hollowing
- optional drain-hole subtraction

### Thicken

Current responsibilities:

- thicken all geometry
- thicken violating regions
- thicken one selected region
- thicken multiple selected regions

### Scoop

Current responsibilities:

- apply a constrained localized carve to allowed regions
- reject or constrain unsafe geometry edits through thickness rules

### Smooth

Current responsibilities:

- region-local smoothing
- multi-region smoothing
- conservative global smoothing

### Compare

Current responsibilities:

- signed-distance compare against another version
- return scalar overlay values
- persist cache artifacts on first compute

### Branch/restore

Current responsibilities:

- duplicate a version into a new branch-safe working version

## 8. Frontend Workspace

The current frontend viewer page lives at:

- `meshinspector-frontend/src/app/viewer/page.tsx`

The page is a dashboard-first manufacturing workspace composed from panel modules and a viewer engine.

### Current major frontend modules

- `ManufacturabilityPanel`
- `GuidedWorkflowPanel`
- `AdvancedEditPanel`
- `OverlayLegendPanel`
- `JobActivityPanel`
- `ComparePanel`
- `VersionHistoryPanel`
- `ViewerEngine`

### Current frontend data hooks

The viewer page uses hooks from:

- `meshinspector-frontend/src/hooks/useModelProcessing.ts`

Current data responsibilities include:

- load versions
- load manufacturability snapshot
- load viewer manifest
- load inspection snapshots
- load compare cache
- load thickness overlay
- load compare overlay
- submit jobs for repair, resize, hollow, thicken, scoop, smooth, and make-manufacturable
- download STL exports
- poll job status
- stream job events

## 9. Viewer Engine

The viewer engine is implemented in:

- `meshinspector-frontend/src/features/editor/viewer/ViewerEngine.tsx`

Current responsibilities include:

- load low-detail preview mesh
- optionally load high-detail preview mesh
- render normalized mesh-derived overlays
- section clipping
- region overlay rendering
- scalar overlay rendering for thickness and compare
- region picking
- additive multi-selection
- section contour extraction
- section contour rendering
- dimension guide rendering on the section plane

### Overlay sources

Current overlay inputs:

- `GLB` preview assets for the visible model
- normalized `PLY` plus region JSON for semantic overlays and picking
- JSON overlay payloads from backend endpoints for thickness and compare scalar data

## 10. Frontend Editor State

Current viewer/editor state is managed in:

- `meshinspector-frontend/src/features/editor/store.ts`

Current state categories include:

- wireframe toggle
- section enabled state
- section plane value
- heatmap enabled state
- region overlay enabled state
- selected material
- selected primary region
- selected batch regions
- compare overlay enabled state
- compare target version

### URL-persisted state

The viewer page persists or restores these values from the URL:

- `model`
- `version`
- `job`
- `wire`
- `section`
- `plane`
- `heatmap`
- `regions`
- `region`
- `regions_selected`
- `axis_mode`
- `axis`
- `compare`
- `compare_target`

This makes inspection and review state shareable and reload-safe.

## 11. Inspection Snapshot Contract

Inspection snapshots are stored under `analysis_snapshots`.

The current saved payload includes:

- `name`
- `axis_mode`
- `manual_axis`
- `section_enabled`
- `section_constant`
- `selected_region_id`
- `selected_region_ids`
- `heatmap_enabled`
- `compare_enabled`
- `compare_target_version_id`

This allows reproducible review states per version.

## 12. Version History Workflow

The current version history UI allows:

- opening any prior version
- comparing any prior version to the current one
- restoring any prior version as a new branch

This is not a mutable undo stack. It is an immutable version model with explicit branching.

## 13. Object Storage Behavior

Object storage is abstracted through:

- `meshinspector-backend/storage/object_store.py`

### Local mode

Behavior:

- files are copied into the configured storage directory
- artifacts can be served directly from local paths through the API

### S3 mode

Behavior:

- files are uploaded to the configured bucket and key prefix
- artifacts are downloaded to temp files when the API needs to serve them

## 14. Job Execution Model

Operation submission creates:

- a `job`
- an `operation_request`
- queued execution through the selected backend

The frontend then:

- polls job status
- streams job events
- switches to the resulting new version when the job completes successfully

This behavior is orchestrated through:

- `meshinspector-backend/workers/*`
- `meshinspector-frontend/src/features/editor/hooks/useJobEventStream.ts`
- `meshinspector-frontend/src/hooks/useJobPolling.ts`

## 15. Current Missing or Partial Technical Areas

These are important technical gaps relative to the broader manufacturing vision.

### Deeper deformation toolset

Not implemented yet:

- comfort-fit shaping
- free-form deformation
- Laplacian editing

### Richer semantic intelligence

Current limitation:

- region detection is still heuristic and ring-specific

### Drain-hole authoring UI

Current limitation:

- drain holes are backend-planned rather than interactively authored

### Full production hardening

Still needed:

- geometry regression fixtures and golden-mesh tests
- artifact retention and cleanup policies
- deeper deployment hardening

## 16. Source Files That Define Current Behavior

When updating this system, these files are the most important ground-truth technical entry points.

Backend:

- `meshinspector-backend/api/routers/models.py`
- `meshinspector-backend/api/routers/versions.py`
- `meshinspector-backend/api/routers/operations.py`
- `meshinspector-backend/api/routers/jobs.py`
- `meshinspector-backend/domain/models.py`
- `meshinspector-backend/domain/schemas.py`
- `meshinspector-backend/services/operations.py`
- `meshinspector-backend/services/hollow.py`
- `meshinspector-backend/services/measure_ring.py`
- `meshinspector-backend/services/manufacturability.py`
- `meshinspector-backend/services/regions.py`
- `meshinspector-backend/services/versioning.py`
- `meshinspector-backend/storage/object_store.py`

Frontend:

- `meshinspector-frontend/src/app/viewer/page.tsx`
- `meshinspector-frontend/src/features/editor/viewer/ViewerEngine.tsx`
- `meshinspector-frontend/src/features/editor/store.ts`
- `meshinspector-frontend/src/features/editor/panels/ManufacturabilityPanel.tsx`
- `meshinspector-frontend/src/features/editor/panels/GuidedWorkflowPanel.tsx`
- `meshinspector-frontend/src/features/editor/panels/AdvancedEditPanel.tsx`
- `meshinspector-frontend/src/features/editor/panels/OverlayLegendPanel.tsx`
- `meshinspector-frontend/src/features/editor/panels/ComparePanel.tsx`
- `meshinspector-frontend/src/features/editor/panels/JobActivityPanel.tsx`
- `meshinspector-frontend/src/features/editor/panels/VersionHistoryPanel.tsx`
- `meshinspector-frontend/src/hooks/useModelProcessing.ts`

## Maintenance Rule

If a feature is changed in code, this technical reference should be updated in the same pull request or change set.
