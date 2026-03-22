#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKBENCH_DIR="${ROOT_DIR}/meshlib-workbench"
BUILD_DIR="${WORKBENCH_DIR}/build-wasm"
PUBLIC_RUNTIME_DIR="${ROOT_DIR}/meshinspector-frontend/public/meshlib-workbench/runtime"

if [[ -z "${EMSDK:-}" ]]; then
  echo "EMSDK is not set. Example:"
  echo "  source /path/to/emsdk/emsdk_env.sh"
  exit 1
fi

cmake -E make_directory "${BUILD_DIR}"
pushd "${BUILD_DIR}" >/dev/null
emcmake cmake "${WORKBENCH_DIR}" -GNinja "$@"
ninja
popd >/dev/null

rm -rf "${PUBLIC_RUNTIME_DIR}"
mkdir -p "${PUBLIC_RUNTIME_DIR}"
cp -R "${BUILD_DIR}/html/." "${PUBLIC_RUNTIME_DIR}/"

cat > "${PUBLIC_RUNTIME_DIR}/manifest.json" <<'JSON'
{
  "status": "ready",
  "message": "MeshLib workbench runtime bundle installed.",
  "entry_html_url": "/meshlib-workbench/runtime/index.html",
  "assets_base_url": "/meshlib-workbench/runtime"
}
JSON

echo "Installed MeshLib workbench runtime into ${PUBLIC_RUNTIME_DIR}"
