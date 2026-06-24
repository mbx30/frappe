#!/usr/bin/env bash
# scripts/perf.sh — run the local perf-suite and post results to the PR.
#
# Usage:
#   ./scripts/perf.sh                # local run, prints summary
#   ./scripts/perf.sh <pr-number>    # local run, posts comment to PR
#
# The script runs the Criterion benches under src-tauri/, then
# assembles a markdown table from the saved `estimates.json` files.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PR_NUMBER="${1:-}"

cd "$ROOT/src-tauri"

echo "==> Building benches (warm cache)..."
cargo bench --no-run

echo "==> Running compression bench..."
cargo bench --bench compression -- --output-format bencher

echo "==> Running preflight bench..."
cargo bench --bench preflight -- --output-format bencher

BENCH_DIR="target/criterion"
OUT_DIR="$ROOT/bench-output"
mkdir -p "$OUT_DIR"

echo "==> Collecting estimates..."
for bench in compression preflight; do
  for variant_dir in "$BENCH_DIR/$bench"/*/; do
    [ -d "$variant_dir" ] || continue
    variant="${variant_dir%/}"
    variant="${variant##*/}"
    if [ -f "$variant_dir/new/estimates.json" ]; then
      cp "$variant_dir/new/estimates.json" "$OUT_DIR/${bench}_${variant}.json"
    fi
  done
done

echo
echo "==> Summary:"
echo

SUMMARY="| Bench | Median (ns/iter) | Throughput |"
SUMMARY+=$'\n'"| --- | --- | --- |"
for f in "$OUT_DIR"/*.json; do
  [ -f "$f" ] || continue
  name="$(basename "$f" .json)"
  median=$(node -e "console.log(JSON.parse(require('fs').readFileSync('$f','utf8')).median.point_estimate)" 2>/dev/null || echo "-")
  tput=$(node -e "console.log(JSON.parse(require('fs').readFileSync('$f','utf8')).throughput?.per_iteration ?? '-')" 2>/dev/null || echo "-")
  SUMMARY+=$'\n'"| \`$name\` | $median | $tput |"
done

echo "$SUMMARY"

if [ -n "$PR_NUMBER" ]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh CLI not found; skipping PR comment."
    exit 0
  fi
  echo
  echo "==> Posting comment to PR #$PR_NUMBER..."
  gh pr comment "$PR_NUMBER" --body "$SUMMARY"
fi
