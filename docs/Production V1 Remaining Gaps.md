# Production V1 Remaining Gaps

This document tracks the main items still required to call MeshInspector a true production-grade v1 after the current platform rewrite.

## What Is Already In Place

- versioned model, artifact, and job records
- manufacturability snapshots
- async ingest and operation submission
- Supabase-compatible `DATABASE_URL` and `DIRECT_URL` support
- Alembic scaffolding plus initial schema migration
- local or S3-compatible object storage abstraction
- dashboard-first frontend flow
- version-aware viewer pipeline

## Remaining High-Priority Gaps

## 1. Real Database Migration Workflow

Current state:

- initial migration exists
- runtime still supports sqlite auto-create for local convenience

Remaining work:

- run the initial migration against Supabase
- add a repeatable deploy command for `alembic upgrade head`
- stop relying on runtime schema creation outside sqlite/dev

## 2. Real Worker Runtime

Current state:

- Celery integration exists
- eager mode can be disabled

Remaining work:

- run a dedicated worker process in deployment
- add Redis availability checks to deploy docs
- add worker concurrency and retry tuning for geometry tasks

## 3. Artifact Lifecycle And Cleanup

Current state:

- artifacts are persisted to object storage

Remaining work:

- cleanup temp scratch directories after success/failure
- add retention policy for obsolete derived artifacts
- add safe delete/archive flow for models and versions

## 4. Better Job Streaming

Current state:

- job polling works
- `/api/jobs/{id}/events` returns an SSE-compatible payload stream from stored events

Remaining work:

- support live event streaming instead of replay-only responses
- push worker progress events during long-running tasks
- expose operation-step progress in the frontend

## 5. Ring Semantics And Protected Geometry

Current state:

- ring measurement exists
- hollowing works

Remaining work:

- implement explicit region classification: `inner_band`, `outer_band`, `head`, `ornament_relief`
- make protected-detail hollowing actually region-aware instead of using the current placeholder protection contract
- add axis confirmation UX when confidence is low

## 6. Local Editing Tools

Current state:

- `thicken` is implemented
- `scoop` and `smooth` are scaffolded only

Remaining work:

- implement region-scoped scoop/carve
- implement safe regional smoothing
- expose selected-region operations in the viewer

## 7. Compare Visualization

Current state:

- compare job exists in the backend

Remaining work:

- persist compare scalar-field artifacts
- render compare heatmaps in the viewer
- add version-to-version compare UI

## 8. Frontend Production Hardening

Current state:

- the new viewer and dashboard build successfully

Remaining work:

- replace placeholder action defaults with real forms
- wire viewer overlay toggles to actual thickness and compare artifacts
- add user-facing failure states for jobs and missing artifacts
- add route restoration and refresh-safe version/job hydration

## 9. Test Coverage

Current state:

- backend smoke tests were run manually in the implementation session

Remaining work:

- add automated pytest coverage for ingest, repair, resize, hollow, and compare
- add golden mesh fixtures
- add frontend integration coverage for upload, version view, and job polling

## 10. Deployment Readiness

Current state:

- local infra scaffold exists in `docker-compose.yml`

Remaining work:

- add backend deployment instructions for Supabase + Redis + object storage
- add frontend deployment env documentation
- define production env defaults
- add readiness/liveness checks to deployment guide

## Recommended Order

1. Run Alembic on Supabase and turn off runtime schema creation in production.
2. Run a real Celery worker and verify Redis-backed async behavior.
3. Add automated tests for ingest plus one derived operation.
4. Implement real protected-region semantics for hollowing.
5. Finish scoop/smooth and compare visualization.
6. Harden the frontend job/overlay UX.
