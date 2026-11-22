#!/usr/bin/env bash
# Gabungkan hasil kerja satu agent (worktree /tmp/gpui-<nama>, branch agent/<nama>)
# ke branch spike/gpui-kit tanpa merge seluruh branch.
#
# Agent hanya boleh menyentuh `src/views/<nama>.rs` + `progress/<nama>.md`, jadi
# yang diambil pun hanya dua file itu. Kalau file lain ikut berubah, skrip ini
# berhenti — itu tanda agent melanggar batasan dan perlu diperiksa manusia.
#
# Pakai: ./merge-agent.sh add|diff|settings|history
set -euo pipefail

NAME="${1:?pakai: ./merge-agent.sh add|diff|settings|history}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"     # root repo (rekonfig)
SPIKE="$ROOT/spike/gpui-kit"
WT="/tmp/gpui-$NAME"
BRANCH="agent/$NAME"

VIEW="spike/gpui-kit/src/views/$NAME.rs"
PROG="spike/gpui-kit/progress/$NAME.md"

[[ -d "$WT" ]] || { echo "worktree tidak ada: $WT"; exit 1; }

CHANGED=$(git -C "$WT" diff --name-only HEAD~0 2>/dev/null || true)
UNCOMMITTED=$(git -C "$WT" status --porcelain | awk '{print $2}')

echo "== periksa cakupan file =="
for f in $UNCOMMITTED; do
  case "$f" in
    "$VIEW"|"$PROG") ;;
    *) echo "  TOLAK: agent mengubah file di luar cakupan: $f"; exit 1 ;;
  esac
done

echo "== ambil $VIEW + $PROG dari $BRANCH =="
for f in "$VIEW" "$PROG"; do
  if [[ -f "$WT/$f" ]]; then
    mkdir -p "$(dirname "$SPIKE/${f#spike/gpui-kit/}")"
    cp "$WT/$f" "$SPIKE/${f#spike/gpui-kit/}"
    echo "  ok  ${f#spike/gpui-kit/}"
  fi
done

echo "== build =="
cd "$SPIKE"
cargo build -j 4 2>&1 | tail -3

echo
echo "Kalau build hijau: tambahkan entri di PROGRESS.md, lalu"
echo "  git add -A spike && git commit -m \"feat(gpui): <ringkas> (agent/$NAME)\""
