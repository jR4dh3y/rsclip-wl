#!/usr/bin/env bash

set -euo pipefail

pkgdir="${1:-packaging/aur/rsclip-bin}"
pkgbuild="${pkgdir}/PKGBUILD"
srcinfo="${pkgdir}/.SRCINFO"

if [[ ! -f "${pkgbuild}" ]]; then
  printf 'Missing PKGBUILD: %s\n' "${pkgbuild}" >&2
  exit 1
fi

# shellcheck source=/dev/null
source "${pkgbuild}"

write_repeated() {
  local key="$1"
  shift

  local value
  for value in "$@"; do
    printf '\t%s = %s\n' "${key}" "${value}"
  done
}

{
  printf 'pkgbase = %s\n' "${pkgname}"
  printf '\tpkgdesc = %s\n' "${pkgdesc}"
  printf '\tpkgver = %s\n' "${pkgver}"
  printf '\tpkgrel = %s\n' "${pkgrel}"
  printf '\turl = %s\n' "${url}"
  write_repeated 'arch' "${arch[@]}"
  write_repeated 'license' "${license[@]}"
  write_repeated 'depends' "${depends[@]}"
  write_repeated 'optdepends' "${optdepends[@]}"
  write_repeated 'provides' "${provides[@]}"
  write_repeated 'conflicts' "${conflicts[@]}"
  write_repeated 'source' "${source[@]}"
  write_repeated 'sha256sums' "${sha256sums[@]}"
  printf '\n'
  printf 'pkgname = %s\n' "${pkgname}"
} > "${srcinfo}"
