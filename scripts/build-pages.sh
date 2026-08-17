#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESTINATION="${1:-${ROOT_DIR}/site}"

rm -rf "${DESTINATION}"
mkdir -p "${DESTINATION}/pkg"

wasm-pack build "${ROOT_DIR}/web" \
  --target web \
  --release \
  --out-dir "${DESTINATION}/pkg" \
  --out-name rex_web

cp "${ROOT_DIR}/demo/index.html" "${DESTINATION}/index.html"
cp "${ROOT_DIR}/demo/styles.css" "${DESTINATION}/styles.css"
cp "${ROOT_DIR}/demo/app.js" "${DESTINATION}/app.js"
cp "${ROOT_DIR}/demo/controller.js" "${DESTINATION}/controller.js"
cp "${ROOT_DIR}/demo/renderer.js" "${DESTINATION}/renderer.js"
cp "${ROOT_DIR}/demo/view.js" "${DESTINATION}/view.js"
cp "${ROOT_DIR}/rex-xits.woff2" "${DESTINATION}/rex-xits.woff2"
touch "${DESTINATION}/.nojekyll"
