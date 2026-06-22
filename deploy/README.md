# MeshInspector — GCP deployment runbook

Deploys the frontend (Next.js) and backend (FastAPI + Rust/PyO3 geometry kernel)
to **Cloud Run**, backed by **Cloud SQL for PostgreSQL** and **Cloud Storage**.

## Architecture

```
                       ┌──────────────────────────┐
   browser ──HTTPS──▶  │ meshinspector-frontend   │  Cloud Run (public)
        │              │  Next.js standalone       │
        │  CORS + SSE  └──────────────────────────┘
        └────────────▶ ┌──────────────────────────┐   ┌──────────────────┐
                       │ meshinspector-api         │──▶│ Cloud SQL (PG16) │
                       │  uvicorn (ROLE=web)       │   └──────────────────┘
                       └──────────────────────────┘            ▲
                       ┌──────────────────────────┐            │ jobs (DB queue)
                       │ meshinspector-worker      │────────────┘
                       │  uvicorn + queue runner   │   ┌──────────────────┐
                       │  (ROLE=worker, always-on) │──▶│ GCS bucket       │
                       └──────────────────────────┘   │ (S3 interop)     │
                                                       └──────────────────┘
```

- **Three Cloud Run services** from **two images**:
  - `meshinspector-api` — public API. `ROLE=web`. Autoscales (to zero when idle).
  - `meshinspector-worker` — background job processor. Same backend image, `ROLE=worker`. Pinned `--min-instances 1 --no-cpu-throttling` (the queue runner must always be alive). Internal ingress, no public access.
  - `meshinspector-frontend` — Next.js standalone server.
- **Migrations** run as a Cloud Run **Job** (`meshinspector-migrate`, `ROLE=migrate` → `alembic upgrade head`) before each release.
- **Job queue** is the built-in **database queue** — no Redis required.
- **Object storage** is GCS, reached through the existing S3 driver via GCS's S3 interoperability endpoint (HMAC keys).

> The backend Rust kernel is compiled **inside the image** for linux/amd64 (the committed `.so` is gitignored and arm64). Images build on **Cloud Build** (native amd64) — you don't need Docker locally to deploy.

## Prerequisites

- `gcloud` CLI, authenticated: `gcloud auth login` and a billing-enabled project.
- `openssl` (for the generated DB password). `git` (for the image tag).
- A globally-unique GCS bucket name (default `${PROJECT_ID}-meshinspector-artifacts`).

## 1. Configure

```bash
cp deploy/config.example.sh deploy/config.sh
$EDITOR deploy/config.sh        # set PROJECT_ID, REGION, sizing, etc.
```
`deploy/config.sh` is gitignored.

## 2. Provision (one-time, idempotent)

```bash
bash deploy/setup-gcp.sh
```
Creates / configures: required APIs, Artifact Registry repo, Cloud SQL instance + database + user (random password → Secret Manager), GCS bucket, a runtime service account with least-privilege IAM (`cloudsql.client`, `secretmanager.secretAccessor`, bucket `objectAdmin`), a GCS **HMAC key** (→ Secret Manager), and the Cloud Build SA deploy permissions.

Secrets created: `meshinspector-database-url`, `meshinspector-s3-access-key`, `meshinspector-s3-secret-key`.

## 3. Deploy

```bash
bash deploy/deploy.sh            # tag = git short SHA, or: deploy/deploy.sh v1
```
Order (handled for you): build backend → migrate → deploy API (capture URL) → deploy worker → build frontend with the API URL baked in → deploy frontend → set the API's `CORS_ORIGINS` to the frontend URL. Prints the public URLs at the end.

## Configuration reference

Backend env (set by `deploy.sh`; secrets via Secret Manager):

| Var | Value | Notes |
|---|---|---|
| `ROLE` | `web` / `worker` / `migrate` | selects the process |
| `DATABASE_URL`, `DIRECT_URL` | *(secret)* | Cloud SQL unix-socket DSN |
| `QUEUE_BACKEND` | `database` | DB-backed queue (no Redis) |
| `DEV_DB_QUEUE_RUNNER_ENABLED` | `false` (web) / `true` (worker) | defaulted by the entrypoint |
| `OBJECT_STORE_DRIVER` | `s3` | GCS via S3 interop |
| `OBJECT_STORE_BUCKET` | `${GCS_BUCKET}` | |
| `S3_ENDPOINT_URL` | `https://storage.googleapis.com` | |
| `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY` | *(secret)* | GCS HMAC key |
| `S3_REGION` | `auto` | |
| `AUTO_CREATE_SCHEMA` | `false` | Postgres uses Alembic, not auto-create |
| `CORS_ORIGINS` | frontend URL | set automatically post-deploy |
| `DATA_DIR`/`TEMP_DIR`/`STORAGE_DIR`/… | `/tmp/...` | entrypoint default (Cloud Run FS is read-only except `/tmp`) |

Frontend:

| Var | When | Notes |
|---|---|---|
| `NEXT_PUBLIC_API_URL` | **build time** | baked into the bundle; changing it requires a rebuild (`deploy.sh` handles this) |

## Operations

- **Logs**: `gcloud run services logs read meshinspector-api --region $REGION`
- **Manual migration**: `gcloud run jobs execute meshinspector-migrate --region $REGION --wait`
- **Scale the API**: `gcloud run services update meshinspector-api --region $REGION --max-instances 20 --cpu 4 --memory 4Gi`
- **Redeploy one service**: re-run `deploy/deploy.sh` (rebuilds + redeploys all; safe and idempotent).
- **Rotate the DB password / HMAC key**: update the value, `gcloud secrets versions add <secret> --data-file=-`, then redeploy (services read `:latest`).
- **Custom domains**: map domains to the frontend and API services (`gcloud run domain-mappings create ...`). Because `NEXT_PUBLIC_API_URL` is build-time, set `_API_URL` to the API's custom domain and re-run `deploy.sh`, then the API's `CORS_ORIGINS` to the frontend's custom domain.

## Local testing of the production images

The backend image only builds on **linux/amd64** (the `meshlib` wheel is glibc/amd64-only).
On Apple Silicon use emulation (slower):

```bash
# Backend image (amd64; emulated on arm64 Macs)
docker buildx build --platform linux/amd64 -t mi-backend meshinspector-backend

# Run it against the local dev infra (postgres/minio) from the root compose file:
docker compose up -d postgres minio          # repo-root docker-compose.yml
docker run --rm --platform linux/amd64 -p 8080:8080 \
  -e ROLE=web -e DATABASE_URL='postgresql+psycopg://meshinspector:meshinspector@host.docker.internal:5432/meshinspector' \
  -e AUTO_CREATE_SCHEMA=true -e OBJECT_STORE_DRIVER=local mi-backend
# health: curl localhost:8080/health

# Frontend image (builds natively):
docker build --build-arg NEXT_PUBLIC_API_URL=http://localhost:8080 -t mi-frontend meshinspector-frontend
docker run --rm -p 3000:8080 mi-frontend     # open http://localhost:3000
```

## Push-to-deploy (CI/CD)

`deploy/cloudbuild.deploy.yaml` is the full pipeline (build both images → migrate → deploy all 3 services). To run it on every push to `master`:

1. **Connect GitHub to Cloud Build (one-time, interactive — must be done in the console):**
   Console → Cloud Build → Triggers → *Connect Repository* → GitHub (Cloud Build GitHub App) → authorize `harshit0605/meshinspector`. (This OAuth step can't be done from the CLI.)

2. **Create the trigger** (after the connection exists):
   ```bash
   gcloud builds triggers create github \
     --name meshinspector-deploy \
     --region us-central1 \
     --repo-name meshinspector --repo-owner harshit0605 \
     --branch-pattern '^master$' \
     --build-config deploy/cloudbuild.deploy.yaml
   ```
   (Or create it in the console pointing at `deploy/cloudbuild.deploy.yaml`.)

3. **Grant the build service account deploy permissions** (the trigger's SA needs to deploy Cloud Run + act as the runtime SA):
   ```bash
   PN=$(gcloud projects describe meshinspector-prod --format='value(projectNumber)')
   for r in run.admin iam.serviceAccountUser cloudsql.client; do
     gcloud projects add-iam-policy-binding meshinspector-prod \
       --member "serviceAccount:${PN}@cloudbuild.gserviceaccount.com" --role roles/$r --condition=None
   done
   ```

Now `git push origin master` builds and deploys automatically. The pipeline uses the stable service URLs (substitutions in the yaml); update them there if you add custom domains.

> Worker scale-to-zero: the API wakes the worker via `POST /internal/drain` (OIDC) after enqueuing a job; a Cloud Scheduler job (`meshinspector-drain`, every 15 min) is the backstop. Worker runs `min-instances=0` so it costs nothing when idle (first job after idle waits ~15-30s for cold start).

## Troubleshooting

- **`uv sync` / meshlib install fails** → ensure the build is linux/amd64 (no Alpine; `meshlib` has only glibc manylinux wheels).
- **Rust build: `feature edition2024 is required`** → the image uses Rust `stable`; don't pin an older toolchain.
- **API 500 `Rust geometry backend requires _zennah_geometry_rs`** → the extension didn't compile into the image; check the backend build logs for the `maturin develop` step.
- **DB connection errors** → confirm `--add-cloudsql-instances` is set and `DATABASE_URL` uses `...@/<db>?host=/cloudsql/<conn>`. Migrations must have run (`meshinspector-migrate`).
- **Bucket warning at startup (`could not verify/create bucket`)** → harmless if the bucket exists; grant the runtime SA `storage.buckets.get` to silence it. Real access errors surface on the first upload.
- **Jobs never finish** → confirm the worker service is up with `--min-instances 1 --no-cpu-throttling` and `ROLE=worker`.
- **CORS errors in the browser** → the API's `CORS_ORIGINS` must equal the frontend origin exactly (scheme + host, no trailing slash). `deploy.sh` sets this; for custom domains, update it.
