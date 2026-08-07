#!/usr/bin/env bash
# Solana base58 excludes 0, O, I, l — so "Ruslan" becomes "Rus1an" (l -> 1).
# --ignore-case matches ruS1an, rUs1An, RUS1AN, etc.
set -euo pipefail
PREFIX="${1:-Rus1an}"
COUNT="${2:-1}"
THREADS="${3:-$(sysctl -n hw.ncpu 2>/dev/null || echo 8)}"
echo "Searching for Solana pubkey starting with '${PREFIX}' (case-insensitive)"
echo "Threads: ${THREADS}"
echo "Note: expected attempts ~58^${#PREFIX} / case-variants — 6 chars can take hours/days on CPU"
echo "Writing keypair file(s) into: $(pwd)"
exec solana-keygen grind \
  --ignore-case \
  --starts-with "${PREFIX}:${COUNT}" \
  --num-threads "${THREADS}"
