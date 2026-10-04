use alloy::{
    primitives::{Address, Bytes, U256},
    sol_types::SolValue,
};

use crate::{calltypes::Path, errors::UniswapV3Error};

use super::UniversalRouterData;

/// Universal Router `V3_SWAP_EXACT_IN` command.
const V3_SWAP_EXACT_IN: u8 = 0x00;

/// Encodes `swapData` for Revert's `Swapper._routerSwap`:
/// `abi.encode(universalRouter, abi.encode(UniversalRouterData))`.
///
/// The transformer transfers `amount_in` to the router before `execute`, so the swap pays from
/// the router's balance (`payerIsUser = false`) and sends output back to `recipient`.
pub(crate) fn universal_router_swap_data(
    universal_router: Address,
    path: &Path,
    amount_in: U256,
    amount_out_min: U256,
    recipient: Address,
    deadline: U256,
) -> Result<Bytes, UniswapV3Error> {
    let input = (
        recipient,
        amount_in,
        amount_out_min,
        path.bytes(false)?,
        false,
    )
        .abi_encode_params();
    let router_data = UniversalRouterData {
        commands: vec![V3_SWAP_EXACT_IN].into(),
        inputs: vec![input.into()],
        deadline,
    }
    .abi_encode();

    Ok((universal_router, Bytes::from(router_data))
        .abi_encode_params()
        .into())
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;
    use uniswap_sdk_core::prelude::Token;

    use super::*;

    #[test]
    fn encodes_revert_swapper_layout() {
        let token_in = Token::new(
            1,
            address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
            6,
            None,
            None,
            0,
            0,
        );
        let token_out = Token::new(
            1,
            address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
            18,
            None,
            None,
            0,
            0,
        );
        let mut path = Path::new(token_in);
        path.add_hop(token_out, 500).unwrap();

        let router = address!("1111111111111111111111111111111111111111");
        let recipient = address!("2222222222222222222222222222222222222222");
        let data = universal_router_swap_data(
            router,
            &path,
            U256::from(1_000),
            U256::from(990),
            recipient,
            U256::from(42),
        )
        .unwrap();

        let (decoded_router, router_data) = <(Address, Bytes)>::abi_decode_params(&data).unwrap();
        assert_eq!(decoded_router, router);

        let router_data = UniversalRouterData::abi_decode(&router_data).unwrap();
        assert_eq!(router_data.commands.as_ref(), &[V3_SWAP_EXACT_IN]);
        assert_eq!(router_data.deadline, U256::from(42));

        let (to, amount_in, amount_out_min, encoded_path, payer_is_user) =
            <(Address, U256, U256, Bytes, bool)>::abi_decode_params(&router_data.inputs[0])
                .unwrap();
        assert_eq!(to, recipient);
        assert_eq!(amount_in, U256::from(1_000));
        assert_eq!(amount_out_min, U256::from(990));
        assert_eq!(encoded_path, path.bytes(false).unwrap());
        assert!(!payer_is_user);
    }
}
