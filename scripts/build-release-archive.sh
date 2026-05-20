#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
VERSION="${1:-}"
ARCHITECTURE="${2:-$(uname -m)}"
ARCHIVE_STEM="clipvault-${VERSION}-${ARCHITECTURE}"
STAGE_DIR="${DIST_DIR}/${ARCHIVE_STEM}"
ARCHIVE_PATH="${DIST_DIR}/${ARCHIVE_STEM}.tar.zst"

if [[ -z "${VERSION}" ]]; then
  printf 'Usage: %s <version> [arch]\n' "${BASH_SOURCE[0]##*/}" >&2
  exit 1
fi

if [[ ! -f "${ROOT_DIR}/LICENSE" ]]; then
  printf 'Missing LICENSE file in %s\n' "${ROOT_DIR}" >&2
  exit 1
fi

rm -rf "${STAGE_DIR}" "${ARCHIVE_PATH}" "${ARCHIVE_PATH}.sha256"
mkdir -p "${DIST_DIR}"

cargo build \
  --release \
  --locked \
  -p clipvault-ui \
  -p clipvault-daemon \
  --bins

install -Dm755 "${ROOT_DIR}/target/release/clipvault" \
  "${STAGE_DIR}/usr/bin/clipvault"
install -Dm755 "${ROOT_DIR}/target/release/clipvaultd" \
  "${STAGE_DIR}/usr/bin/clipvaultd"
install -Dm644 "${ROOT_DIR}/packaging/desktop/clipvault.desktop" \
  "${STAGE_DIR}/usr/share/applications/clipvault.desktop"
install -Dm644 "${ROOT_DIR}/packaging/systemd/clipvaultd.service" \
  "${STAGE_DIR}/usr/lib/systemd/user/clipvaultd.service"
install -Dm644 "${ROOT_DIR}/config.example.toml" \
  "${STAGE_DIR}/usr/share/doc/clipvault/config.example.toml"
install -Dm644 "${ROOT_DIR}/README.md" \
  "${STAGE_DIR}/usr/share/doc/clipvault/README.md"
install -Dm644 "${ROOT_DIR}/LICENSE" \
  "${STAGE_DIR}/usr/share/licenses/clipvault/LICENSE"

tar --zstd -cf "${ARCHIVE_PATH}" -C "${DIST_DIR}" "${ARCHIVE_STEM}"
sha256sum "${ARCHIVE_PATH}" > "${ARCHIVE_PATH}.sha256"

printf 'Built %s\n' "${ARCHIVE_PATH}"
printf 'Wrote %s.sha256\n' "${ARCHIVE_PATH}"
