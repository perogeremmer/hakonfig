#!/usr/bin/env bash
# Tangkap ulang screenshot dokumentasi.
#
#   ./shot.sh                 # config asli di $HOME, tema terang
#   ./shot.sh --dark          # config asli, tema gelap (Providers + Pengaturan)
#   ./shot.sh --fresh         # HOME kosong: tampilan instalasi baru, semua layar
#   ./shot.sh --fresh --dark  # instalasi baru, tema gelap
#
# Navigasi: ikon gear kanan atas (1160 26) membuka Pengaturan, kotak AKSI di
# layar Providers membuka Tambah/Diff/Riwayat, tombol "‹ Providers" di (300 68)
# untuk kembali.
set -euo pipefail
cd "$(dirname "$0")"

FRESH=0; DARK=0; HOME_DIR=""
for arg in "$@"; do
  case "$arg" in
    --fresh) FRESH=1; HOME_DIR=$(mktemp -d) ;;
    --dark)  DARK=1 ;;
    *) echo "argumen tidak dikenal: $arg" >&2; exit 2 ;;
  esac
done

cargo build -j 6 >/dev/null
pkill -x hakonfig-spike 2>/dev/null || true
sleep 0.4

DARK_ENV=(); [ "$DARK" = 1 ] && DARK_ENV=(SPIKE_DARK=1)
if [ -n "$HOME_DIR" ]; then
  env HOME="$HOME_DIR" "${DARK_ENV[@]}" ./target/debug/hakonfig-spike >/tmp/shot-app.log 2>&1 &
else
  env "${DARK_ENV[@]}" ./target/debug/hakonfig-spike >/tmp/shot-app.log 2>&1 &
fi
sleep 5

WIN=$(xdotool search --name "spike gpui-kit" | tail -1)
OUT="docs"; PREFIX=""
if [ "$FRESH" = 1 ]; then OUT="docs/fresh-install"; PREFIX="fresh-"; fi
SUFFIX=""; [ "$DARK" = 1 ] && SUFFIX="-dark"
mkdir -p "$OUT"

click() { xdotool mousemove --window "$WIN" "$1" "$2"; sleep 0.3; xdotool click 1; sleep 1.5; }
grab()  { import -window "$WIN" "$OUT/$PREFIX$1$SUFFIX.png"; echo "$OUT/$PREFIX$1$SUFFIX.png"; }

xdotool windowactivate "$WIN"; sleep 0.6
grab providers

if [ "$FRESH" = 0 ] && [ "$DARK" = 1 ]; then
  click 1160 26; grab settings     # tema gelap cukup dua layar ini
  exit 0
fi

click 400 360; grab add-1          # kotak AKSI → Tambah provider
click 400 300; grab add-2          # pilih preset
click 1130 784; grab add-3         # lanjut ke model
click 300 68                       # ‹ Providers
click 400 412; grab diff
click 300 68
click 400 465; grab history
click 300 68
click 1160 26; grab settings
