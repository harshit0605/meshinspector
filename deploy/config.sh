#!/usr/bin/env bash
# MeshInspector GCP deployment config (gitignored). Generated for meshinspector-prod.

# ----- GCP project / location -------------------------------------------------
export PROJECT_ID="meshinspector-prod"
export REGION="us-central1"

# ----- Artifact Registry ------------------------------------------------------
export AR_REPO="meshinspector"

# ----- Cloud SQL (PostgreSQL) -------------------------------------------------
export SQL_INSTANCE="meshinspector-db"
export SQL_TIER="db-custom-1-3840"
export SQL_DB="meshinspector"
export SQL_USER="meshinspector"

# ----- Cloud Storage ----------------------------------------------------------
export GCS_BUCKET="meshinspector-prod-artifacts"
export OBJECT_STORE_PREFIX="artifacts"

# ----- Runtime service account ------------------------------------------------
export RUNTIME_SA="meshinspector-run"

# ----- Cloud Run service / job names ------------------------------------------
export API_SERVICE="meshinspector-api"
export WORKER_SERVICE="meshinspector-worker"
export FRONTEND_SERVICE="meshinspector-frontend"
export MIGRATE_JOB="meshinspector-migrate"

# ----- Secret Manager secret names --------------------------------------------
export SECRET_DB_URL="meshinspector-database-url"
export SECRET_S3_KEY="meshinspector-s3-access-key"
export SECRET_S3_SECRET="meshinspector-s3-secret-key"

# ----- Service sizing ---------------------------------------------------------
export API_CPU="2";      export API_MEMORY="2Gi";   export API_MAX_INSTANCES="10"
export WORKER_CPU="2";   export WORKER_MEMORY="2Gi"
export FRONTEND_CPU="1"; export FRONTEND_MEMORY="512Mi"; export FRONTEND_MAX_INSTANCES="5"

# ----- Derived ----------------------------------------------------------------
export AR_HOST="${REGION}-docker.pkg.dev"
export IMAGE_BASE="${AR_HOST}/${PROJECT_ID}/${AR_REPO}"
export SQL_CONN="${PROJECT_ID}:${REGION}:${SQL_INSTANCE}"
export RUNTIME_SA_EMAIL="${RUNTIME_SA}@${PROJECT_ID}.iam.gserviceaccount.com"
