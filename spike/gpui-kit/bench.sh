#!/usr/bin/env bash
# Bandingkan angka, bukan rasa: startup, ukuran binary, dependency sistem, RAM.
#
# Pakai:
#   ./bench.sh            # build release lalu ukur
#   ./bench.sh --no-build # ulangi pengukuran tanpa build
set -euo pipefail
cd "$(dirname "$0")"

RUN_SECONDS="${RUN_SECONDS:-6}"

if [[ "${1:-}" != "--no-build" ]]; then
  echo "== build release =="
  cargo build --release 2>&1 | tail -3
fi

BIN=target/release/hakonfig-spike
[[ -x $BIN ]] || { echo "binary tidak ada: $BIN"; exit 1; }

echo
echo "== ukuran binary =="
ls -lh "$BIN" | awk '{print "  file      : " $5}'
strip -o /tmp/spike-stripped "$BIN" 2>/dev/null && ls -lh /tmp/spike-stripped | awk '{print "  di-strip  : " $5}'

echo
echo "== dependency sistem (yang harus ada di mesin user) =="
if command -v ldd >/dev/null; then
  ldd "$BIN" | grep -vE "linux-vdso|ld-linux" | awk '{print "  " $1}'
fi

echo
echo "== startup + RAM (jendela dibuka, $RUN_SECONDS detik) =="
if [[ -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
  echo "  dilewati: tidak ada display"
  exit 0
fi

OUT=$(mktemp)
"$BIN" >"$OUT" 2>&1 &
PID=$!
PEAK=0
for _ in $(seq 1 $((RUN_SECONDS * 10))); do
  sleep 0.1
  RSS=$(ps -o rss= -p "$PID" 2>/dev/null | tr -d ' ')
  [[ -n "${RSS:-}" && "$RSS" -gt "$PEAK" ]] && PEAK=$RSS
done
kill "$PID" 2>/dev/null || true
wait "$PID" 2>/dev/null || true

grep -E "^\[spike\]" "$OUT" | sed 's/^/  /'
echo "  RSS puncak: $((PEAK / 1024)) MB"
rm -f "$OUT"
