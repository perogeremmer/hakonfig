#!/usr/bin/env bash
# Waktu dari exec sampai jendela benar-benar ter-map di X11.
# Metrik ini netral terhadap toolkit: dipakai sama untuk GUI Tauri dan spike GPUI.
# Pakai: ./window-startup.sh <binary> <judul-jendela> [ulangan]
set -euo pipefail

BIN="$1"
TITLE="$2"
REPEAT="${3:-3}"

for i in $(seq 1 "$REPEAT"); do
  START=$(date +%s%N)
  "$BIN" >/dev/null 2>&1 &
  PID=$!
  MS=""
  for _ in $(seq 1 2000); do   # maks 20 detik
    if xdotool search --name "$TITLE" >/dev/null 2>&1; then
      MS=$(( ($(date +%s%N) - START) / 1000000 ))
      break
    fi
    sleep 0.01
  done
  RSS=0
  for _ in $(seq 1 30); do
    sleep 0.1
    CUR=$(ps -o rss= -p "$PID" 2>/dev/null | tr -d ' ' || true)
    [[ -n "${CUR:-}" && "$CUR" -gt "$RSS" ]] && RSS=$CUR
  done
  kill "$PID" 2>/dev/null || true
  wait "$PID" 2>/dev/null || true
  sleep 0.4
  echo "  run $i: jendela siap ${MS:-timeout} ms · RSS $((RSS / 1024)) MB"
done
