#!/bin/sh
# Installer hakonfig-cli untuk Linux dan macOS.
#
# Pakai:
#   curl -fsSL https://raw.githubusercontent.com/perogeremmer/hakonfig/master/scripts/install.sh | sh
#
# Atau dengan versi dan lokasi sendiri:
#   install.sh v0.4.0
#   HAKONFIG_INSTALL_DIR=/usr/local/bin install.sh
#
# Binary diunduh dari GitHub releases dan diverifikasi terhadap SHA256SUMS.
set -eu

REPO="perogeremmer/hakonfig"
BIN="hakonfig-cli"

# ------------------------------------------------------------------ target
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin)
    # Release workflow menggabungkan kedua arsitektur macOS dengan lipo.
    asset="$BIN-universal-apple-darwin"
    ;;
  Linux)
    case "$arch" in
      x86_64) asset="$BIN-x86_64-unknown-linux-gnu" ;;
      *)
        echo "hakonfig: Linux $arch belum tersedia sebagai binary." >&2
        echo "  Bangun dari source: cargo install --git https://github.com/$REPO hakonfig-cli" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "hakonfig: OS '$os' tidak didukung oleh installer ini." >&2
    echo "  Untuk Windows pakai install.ps1. Lainnya: cargo install --git https://github.com/$REPO hakonfig-cli" >&2
    exit 1
    ;;
esac

# ------------------------------------------------------------------ url
version="${1:-}"
if [ -n "$version" ]; then
  base="https://github.com/$REPO/releases/download/$version"
else
  # Mengikuti rilis terakhir yang sudah di-publish (bukan draft).
  base="https://github.com/$REPO/releases/latest/download"
fi

# ------------------------------------------------------------------ unduh
dest="${HAKONFIG_INSTALL_DIR:-$HOME/.local/bin}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "mengunduh $asset ..."
curl -fsSL "$base/$asset" -o "$tmp/$BIN"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"

# ------------------------------------------------------------------ verifikasi
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmp" && grep " $asset\$" SHA256SUMS | sha256sum -c - >/dev/null)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$tmp" && grep " $asset\$" SHA256SUMS | shasum -a 256 -c - >/dev/null)
else
  echo "hakonfig: sha256sum/shasum tidak ditemukan; verifikasi dilewati." >&2
fi
echo "checksum ok"

# ------------------------------------------------------------------ pasang
mkdir -p "$dest"
mv "$tmp/$BIN" "$dest/$BIN"
chmod 0755 "$dest/$BIN"

# ------------------------------------------------------------------ laporan
case ":$PATH:" in
  *":$dest:"*) ;;
  *)
    echo "" >&2
    echo "CATATAN: $dest tidak ada di PATH. Tambahkan ke shell profile kamu:" >&2
    echo "  export PATH=\"\$PATH:$dest\"" >&2
    ;;
esac

echo "terpasang: $dest/$BIN ($("$dest/$BIN" --version 2>/dev/null || echo '?'))"
