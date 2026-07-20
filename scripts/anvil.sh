#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="${ENV_FILE:-"$ROOT_DIR/.env"}"

if [[ ! -f "$ENV_FILE" ]]; then
    echo "Missing environment file: $ENV_FILE" >&2
    exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

: "${MAINNET_RPC_URL:?MAINNET_RPC_URL is required}"

args=(
    --fork-url "$MAINNET_RPC_URL"
    --host "${ANVIL_HOST:-127.0.0.1}"
    --port "${ANVIL_PORT:-8545}"
    --chain-id "${ANVIL_CHAIN_ID:-1}"
)

if [[ -n "${FORK_BLOCK_NUMBER:-}" ]]; then
    args+=(--fork-block-number "$FORK_BLOCK_NUMBER")
fi

echo "Starting Ethereum mainnet fork at ${ANVIL_HOST:-127.0.0.1}:${ANVIL_PORT:-8545}"
exec anvil "${args[@]}"
