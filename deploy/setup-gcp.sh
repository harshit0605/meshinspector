#!/usr/bin/env bash
###############################################################################
# One-time GCP provisioning for MeshInspector (Cloud Run + Cloud SQL + GCS).
# Idempotent: safe to re-run. Requires: gcloud (authenticated), openssl.
#
#   cp deploy/config.example.sh deploy/config.sh && edit deploy/config.sh
#   bash deploy/setup-gcp.sh
###############################################################################
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ -f "${HERE}/config.sh" ]] || { echo "ERROR: create deploy/config.sh from config.example.sh first"; exit 1; }
# shellcheck disable=SC1091
source "${HERE}/config.sh"
[[ "$PROJECT_ID" != "CHANGE_ME" ]] || { echo "ERROR: set PROJECT_ID in deploy/config.sh"; exit 1; }

gcloud config set project "$PROJECT_ID" >/dev/null

echo "==> Enabling required APIs"
gcloud services enable \
  run.googleapis.com artifactregistry.googleapis.com cloudbuild.googleapis.com \
  sqladmin.googleapis.com secretmanager.googleapis.com storage.googleapis.com

echo "==> Artifact Registry repo: ${AR_REPO}"
gcloud artifacts repositories describe "$AR_REPO" --location "$REGION" >/dev/null 2>&1 || \
  gcloud artifacts repositories create "$AR_REPO" \
    --repository-format docker --location "$REGION" \
    --description "MeshInspector container images"

echo "==> Cloud SQL instance: ${SQL_INSTANCE} (Postgres 16)"
gcloud sql instances describe "$SQL_INSTANCE" >/dev/null 2>&1 || \
  gcloud sql instances create "$SQL_INSTANCE" \
    --database-version POSTGRES_16 --tier "$SQL_TIER" --region "$REGION" \
    --storage-auto-increase --edition ENTERPRISE
gcloud sql databases describe "$SQL_DB" --instance "$SQL_INSTANCE" >/dev/null 2>&1 || \
  gcloud sql databases create "$SQL_DB" --instance "$SQL_INSTANCE"

echo "==> Cloud SQL user: ${SQL_USER}"
DB_PASSWORD="$(openssl rand -base64 30 | tr -dc 'A-Za-z0-9' | head -c 32)"
if gcloud sql users list --instance "$SQL_INSTANCE" --format='value(name)' | grep -qx "$SQL_USER"; then
  gcloud sql users set-password "$SQL_USER" --instance "$SQL_INSTANCE" --password "$DB_PASSWORD"
else
  gcloud sql users create "$SQL_USER" --instance "$SQL_INSTANCE" --password "$DB_PASSWORD"
fi
# Unix-socket DSN for Cloud Run (psycopg v3). Same value used for DIRECT_URL (no pooler).
DATABASE_URL="postgresql+psycopg://${SQL_USER}:${DB_PASSWORD}@/${SQL_DB}?host=/cloudsql/${SQL_CONN}"

echo "==> GCS bucket: gs://${GCS_BUCKET}"
gcloud storage buckets describe "gs://${GCS_BUCKET}" >/dev/null 2>&1 || \
  gcloud storage buckets create "gs://${GCS_BUCKET}" \
    --location "$REGION" --uniform-bucket-level-access

echo "==> Runtime service account: ${RUNTIME_SA_EMAIL}"
if ! gcloud iam service-accounts describe "$RUNTIME_SA_EMAIL" >/dev/null 2>&1; then
  gcloud iam service-accounts create "$RUNTIME_SA" --display-name "MeshInspector Cloud Run runtime"
  echo "    waiting for service account to propagate to IAM..."
  sleep 15
fi

echo "==> IAM for runtime SA (Cloud SQL, Secret Manager, bucket objects)"
# IAM is eventually consistent — a freshly-created SA can 404 on the first binding.
_project_binding() {  # $1=role
  for _ in 1 2 3 4 5 6; do
    if gcloud projects add-iam-policy-binding "$PROJECT_ID" \
         --member "serviceAccount:${RUNTIME_SA_EMAIL}" --role "$1" --condition=None -q >/dev/null 2>&1; then
      return 0
    fi
    sleep 5
  done
  gcloud projects add-iam-policy-binding "$PROJECT_ID" \
    --member "serviceAccount:${RUNTIME_SA_EMAIL}" --role "$1" --condition=None -q >/dev/null  # surface error
}
_project_binding roles/cloudsql.client
_project_binding roles/secretmanager.secretAccessor
gcloud storage buckets add-iam-policy-binding "gs://${GCS_BUCKET}" \
  --member "serviceAccount:${RUNTIME_SA_EMAIL}" --role roles/storage.objectAdmin >/dev/null

echo "==> GCS HMAC key (S3 interoperability) for the runtime SA"
# Reuse an existing active key if present, else create one (secret only shown at create).
HMAC_KEY=""; HMAC_SECRET=""
CREATE_OUT="$(gcloud storage hmac create "$RUNTIME_SA_EMAIL" --format='value(metadata.accessId,secret)' 2>/dev/null || true)"
if [[ -n "$CREATE_OUT" ]]; then
  HMAC_KEY="$(echo "$CREATE_OUT" | awk '{print $1}')"
  HMAC_SECRET="$(echo "$CREATE_OUT" | awk '{print $2}')"
else
  echo "    (could not create a new HMAC key — you may already have the max; see runbook to rotate)"
fi

echo "==> Secret Manager"
put_secret() { # name value
  if gcloud secrets describe "$1" >/dev/null 2>&1; then
    printf '%s' "$2" | gcloud secrets versions add "$1" --data-file=- >/dev/null
  else
    printf '%s' "$2" | gcloud secrets create "$1" --data-file=- --replication-policy automatic >/dev/null
  fi
  echo "    secret ${1} updated"
}
put_secret "$SECRET_DB_URL" "$DATABASE_URL"
if [[ -n "$HMAC_KEY" ]]; then
  put_secret "$SECRET_S3_KEY" "$HMAC_KEY"
  put_secret "$SECRET_S3_SECRET" "$HMAC_SECRET"
else
  echo "    NOTE: HMAC secrets not written. Create them manually (see runbook) before deploy.sh."
fi

echo "==> Granting Cloud Build permission to deploy + act as the runtime SA"
PROJECT_NUMBER="$(gcloud projects describe "$PROJECT_ID" --format='value(projectNumber)')"
CB_SA="${PROJECT_NUMBER}@cloudbuild.gserviceaccount.com"
for role in roles/run.admin roles/iam.serviceAccountUser roles/cloudsql.client; do
  gcloud projects add-iam-policy-binding "$PROJECT_ID" \
    --member "serviceAccount:${CB_SA}" --role "$role" --condition=None -q >/dev/null
done

echo ""
echo "Provisioning complete. Next: bash deploy/deploy.sh"
