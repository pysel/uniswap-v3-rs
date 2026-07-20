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

: "${LOCAL_RPC_URL:?LOCAL_RPC_URL is required}"
: "${TEST_PRIVATE_KEY:?TEST_PRIVATE_KEY is required}"
: "${WETH_ADDRESS:?WETH_ADDRESS is required}"
: "${USDC_ADDRESS:?USDC_ADDRESS is required}"
: "${USDT_ADDRESS:?USDT_ADDRESS is required}"
: "${WBTC_ADDRESS:?WBTC_ADDRESS is required}"
: "${USDC_HOLDER:?USDC_HOLDER is required}"
: "${USDT_HOLDER:?USDT_HOLDER is required}"
: "${WBTC_HOLDER:?WBTC_HOLDER is required}"

client_version="$(cast rpc --rpc-url "$LOCAL_RPC_URL" web3_clientVersion | tr -d '"')"
if [[ "${client_version,,}" != *anvil* ]]; then
    echo "Refusing to fund a non-Anvil RPC endpoint: $LOCAL_RPC_URL ($client_version)" >&2
    exit 1
fi

recipient="${TEST_ADDRESS:-$(cast wallet address --private-key "$TEST_PRIVATE_KEY")}"

set_eth_balance() {
    local address="$1"
    local amount_wei="$2"
    cast rpc \
        --rpc-url "$LOCAL_RPC_URL" \
        anvil_setBalance \
        "$address" \
        "$(cast to-hex "$amount_wei")" >/dev/null
}

fund_erc20() {
    local symbol="$1"
    local token="$2"
    local holder="$3"
    local amount="$4"
    local holder_balance

    holder_balance="$(
        cast call \
            --rpc-url "$LOCAL_RPC_URL" \
            "$token" \
            "balanceOf(address)(uint256)" \
            "$holder" | awk '{ print $1 }'
    )"

    if (( holder_balance < amount )); then
        echo "$symbol holder has $holder_balance raw units; $amount required" >&2
        exit 1
    fi

    cast rpc --rpc-url "$LOCAL_RPC_URL" anvil_impersonateAccount "$holder" >/dev/null
    set_eth_balance "$holder" "$(cast to-wei 100 ether)"

    set +e
    cast send \
        --rpc-url "$LOCAL_RPC_URL" \
        --unlocked \
        --from "$holder" \
        "$token" \
        "transfer(address,uint256)(bool)" \
        "$recipient" \
        "$amount" >/dev/null
    local status=$?
    set -e

    cast rpc --rpc-url "$LOCAL_RPC_URL" anvil_stopImpersonatingAccount "$holder" >/dev/null
    if (( status != 0 )); then
        echo "Failed to fund $symbol" >&2
        exit "$status"
    fi

    echo "Funded $symbol"
}

set_eth_balance "$recipient" "$(cast to-wei 10000 ether)"

cast send \
    --rpc-url "$LOCAL_RPC_URL" \
    --private-key "$TEST_PRIVATE_KEY" \
    --value "${WETH_AMOUNT_ETH:-1000}ether" \
    "$WETH_ADDRESS" \
    "deposit()" >/dev/null
echo "Funded WETH"

fund_erc20 "USDC" "$USDC_ADDRESS" "$USDC_HOLDER" "${USDC_AMOUNT_RAW:-1000000000000}"
fund_erc20 "USDT" "$USDT_ADDRESS" "$USDT_HOLDER" "${USDT_AMOUNT_RAW:-1000000000000}"
fund_erc20 "WBTC" "$WBTC_ADDRESS" "$WBTC_HOLDER" "${WBTC_AMOUNT_RAW:-1000000000}"

echo "Funded $recipient on $LOCAL_RPC_URL"
