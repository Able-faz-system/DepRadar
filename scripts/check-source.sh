#!/usr/bin/env bash
# check-source.sh — fail if any tracked .rs / .ts / .tsx file contains
# Git conflict markers (<<<<<<< / ======= / >>>>>>>) or Markdown code-fence
# delimiters (```).  Run as the first step of the lint job so garbled files
# are caught before the compiler even starts.
set -euo pipefail

PATTERNS=(
  '<<<<<<<'
  '>>>>>>>'
  '======='
  '```'
)

# Only check tracked source files; untracked / ignored files are not our problem.
SOURCES=$(git ls-files -- '*.rs' '*.ts' '*.tsx')

if [ -z "$SOURCES" ]; then
  echo "check-source: no .rs/.ts/.tsx files tracked — nothing to check"
  exit 0
fi

FOUND=0

for pattern in "${PATTERNS[@]}"; do
  # grep -l prints the filename once per match; redirect stderr in case a
  # file disappears between ls-files and the grep pass.
  MATCHES=$(echo "$SOURCES" | xargs grep -rl --fixed-strings "$pattern" 2>/dev/null || true)
  if [ -n "$MATCHES" ]; then
    echo "ERROR: found forbidden pattern '$pattern' in:"
    echo "$MATCHES" | sed 's/^/  /'
    FOUND=1
  fi
done

if [ "$FOUND" -eq 1 ]; then
  echo ""
  echo "check-source: FAILED — remove conflict markers / code fences before merging."
  exit 1
fi

echo "check-source: OK"
