#!/usr/bin/env bash
###############################################################################
# Build + migrate + deploy MeshInspector to Cloud Run.
# Images are built on Cloud Build (native linux/amd64) — no local Docker needed.
#
#   bash deploy/deploy.sh [TAG]        # TAG defaults to the short git SHA
#
# Ordering matters: the frontend bakes in the API URL at build time, and the API
# must allow the frontend origin via CORS — so we deploy API -> build/deploy
# frontend with its URL -> update API CORS.
###############################################################################
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${HERE}/.." && pwd)"
[[ -f "${HERE}/config.sh" ]] || { echo "ERROR: create deploy/config.sh from config.example.sh first"; exit 1; }
# shellcheck disable=SC1091
source "${HERE}/config.sh"

TAG="${1:-$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || date +%Y%m%d%H%M%S)}"
BACKEND_IMAGE="${IMAGE_BASE}/backend:${TAG}"
FRONTEND_IMAGE="${IMAGE_BASE}/frontend:${TAG}"
gcloud config set project "$PROJECT_ID" >/dev/null

BACKEND_ENV="QUEUE_BACKEND=database,OBJECT_STORE_DRIVER=s3,OBJECT_STORE_BUCKET=${GCS_BUCKET},OBJECT_STORE_PREFIX=${OBJECT_STORE_PREFIX},S3_ENDPOINT_URL=https://storage.googleapis.com,S3_REGION=auto,AUTO_CREATE_SCHEMA=false"
BACKEND_SECRETS="DATABASE_URL=${SECRET_DB_URL}:latest,DIRECT_URL=${SECRET_DB_URL}:latest,S3_ACCESS_KEY_ID=${SECRET_S3_KEY}:latest,S3_SECRET_ACCESS_KEY=${SECRET_S3_SECRET}:latest"

echo "==> [1/6] Build + push backend image: ${BACKEND_IMAGE}"
gcloud builds submit "${ROOT}/meshinspector-backend" \
  --config "${HERE}/cloudbuild.backend.yaml" --substitutions "_IMAGE=${BACKEND_IMAGE}"

echo "==> [2/6] Run DB migrations (alembic upgrade head)"
if gcloud run jobs describe "$MIGRATE_JOB" --region "$REGION" >/dev/null 2>&1; then JOP=update; else JOP=create; fi
gcloud run jobs "$JOP" "$MIGRATE_JOB" \
  --image "$BACKEND_IMAGE" --region "$REGION" \
  --service-account "$RUNTIME_SA_EMAIL" \
  --set-cloudsql-instances "$SQL_CONN" \
  --set-secrets "DATABASE_URL=${SECRET_DB_URL}:latest,DIRECT_URL=${SECRET_DB_URL}:latest" \
  --set-env-vars "ROLE=migrate,AUTO_CREATE_SCHEMA=false" \
  --max-retries 1 --task-timeout 600s
gcloud run jobs execute "$MIGRATE_JOB" --region "$REGION" --wait

echo "==> [3/6] Deploy API service: ${API_SERVICE}"
gcloud run deploy "$API_SERVICE" \
  --image "$BACKEND_IMAGE" --region "$REGION" \
  --service-account "$RUNTIME_SA_EMAIL" \
  --add-cloudsql-instances "$SQL_CONN" \
  --set-secrets "$BACKEND_SECRETS" \
  --set-env-vars "ROLE=web,${BACKEND_ENV}" \
  --cpu "$API_CPU" --memory "$API_MEMORY" --max-instances "$API_MAX_INSTANCES" \
  --port 8080 --allow-unauthenticated
API_URL="$(gcloud run services describe "$API_SERVICE" --region "$REGION" --format='value(status.url)')"
echo "    API_URL=${API_URL}"

echo "==> [4/6] Deploy worker service: ${WORKER_SERVICE} (always-on, internal)"
gcloud run deploy "$WORKER_SERVICE" \
  --image "$BACKEND_IMAGE" --region "$REGION" \
  --service-account "$RUNTIME_SA_EMAIL" \
  --add-cloudsql-instances "$SQL_CONN" \
  --set-secrets "$BACKEND_SECRETS" \
  --set-env-vars "ROLE=worker,${BACKEND_ENV}" \
  --cpu "$WORKER_CPU" --memory "$WORKER_MEMORY" \
  --min-instances 1 --max-instances 1 --no-cpu-throttling \
  --port 8080 --no-allow-unauthenticated --ingress internal

echo "==> [5/6] Build + push frontend image (API URL baked in): ${FRONTEND_IMAGE}"
gcloud builds submit "${ROOT}/meshinspector-frontend" \
  --config "${HERE}/cloudbuild.frontend.yaml" \
  --substitutions "_IMAGE=${FRONTEND_IMAGE},_API_URL=${API_URL}"

echo "==> [6/6] Deploy frontend: ${FRONTEND_SERVICE}, then wire API CORS"
gcloud run deploy "$FRONTEND_SERVICE" \
  --image "$FRONTEND_IMAGE" --region "$REGION" \
  --cpu "$FRONTEND_CPU" --memory "$FRONTEND_MEMORY" --max-instances "$FRONTEND_MAX_INSTANCES" \
  --port 8080 --allow-unauthenticated
FRONTEND_URL="$(gcloud run services describe "$FRONTEND_SERVICE" --region "$REGION" --format='value(status.url)')"
gcloud run services update "$API_SERVICE" --region "$REGION" \
  --update-env-vars "CORS_ORIGINS=${FRONTEND_URL}" >/dev/null

echo ""
echo "============================================================"
echo " Deployed (tag ${TAG})"
echo "   Frontend : ${FRONTEND_URL}"
echo "   API      : ${API_URL}"
echo "   Worker   : ${WORKER_SERVICE} (internal, min-instances=1)"
echo "============================================================"
echo "NOTE: NEXT_PUBLIC_API_URL is baked into the frontend image. If the API URL"
echo "changes (e.g. a custom domain), re-run deploy.sh so the frontend rebuilds."
