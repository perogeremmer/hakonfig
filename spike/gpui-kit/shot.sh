#!/usr/bin/env bash
# Ambil ulang semua screenshot dokumentasi (butuh X11, ImageMagick, xdotool).
#
# Pakai: ./shot.sh            # dari binary debug
#        BIN=./target/release/hakonfig-spike ./shot.sh
set -euo pipefail
cd "$(dirname "$0")"

BIN="${BIN:-./target/debug/hakonfig-spike}"
[[ -x "$BIN" ]] || { echo "binary tidak ada: $BIN (jalankan cargo build dulu)"; exit 1; }

pkill -x hakonfig-spike 2>/dev/null || true
sleep 0.4
"$BIN" >/tmp/shot-run.log 2>&1 &
PID=$!
sleep 4

W=$(xdotool search --pid "$PID" | tail -1)
[[ -n "$W" ]] || { echo "jendela tidak muncul (lihat /tmp/shot-run.log)"; kill "$PID"; exit 1; }
xdotool windowactivate "$W"
sleep 0.4

shot() { import -window "$W" "docs/$1.png"; echo "  docs/$1.png"; }
# Tab layar ada di baris atas (y=74): Providers 51 · Tambah 151 · Diff 236
# · Riwayat 293 · Pengaturan 374. Tombol aksi utama di kanan bawah (1130,784).
click() { xdotool mousemove --window "$W" "$1" "$2"; sleep 0.25; xdotool click 1; sleep 1.3; }

shot ui-light
click 1140 26; shot ui-dark          # tombol tema di top bar
click 1140 26

click 151 74; shot ui-add-1          # langkah 1: preset
click 372 296; shot ui-add-2         # langkah 2: key
click 1130 784; shot ui-add-3        # langkah 3: model
click 1130 784                       # Pratinjau diff
sleep 1.5; shot ui-add-4

click 236 74; sleep 1.5; shot ui-diff
click 293 74; shot ui-history
click 374 74; shot ui-settings
click 1140 26; shot ui-settings-dark

kill "$PID" 2>/dev/null || true
wait "$PID" 2>/dev/null || true
echo "selesai"
