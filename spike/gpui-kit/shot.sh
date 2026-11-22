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

# providers (terang)
shot ui-light

# klik "+ Add provider" supaya dialog aksi tidak menutupi; lalu tema gelap
xdotool mousemove --window "$W" 1134 26 click 1
sleep 1.2
shot ui-dark
xdotool mousemove --window "$W" 1134 26 click 1   # balik ke terang
sleep 0.8

# Riwayat
xdotool mousemove --window "$W" 60 470 click 1
sleep 1.2
shot ui-history

kill "$PID" 2>/dev/null || true
wait "$PID" 2>/dev/null || true
echo "selesai"
