//! Odyssey chainspec parsing logic.
use std::sync::LazyLock;

use alloy_primitives::{address, b256, U256};
use reth_chainspec::{
    BaseFeeParams, BaseFeeParamsKind, Chain, ChainHardforks, ChainSpec, DepositContract,
    EthereumHardfork, ForkCondition,
};
use reth_cli::chainspec::{parse_genesis, ChainSpecParser};

use reth_primitives::constants::ETHEREUM_BLOCK_GAS_LIMIT;
use std::sync::Arc;
use reth::chainspec::{chain_value_parser, SUPPORTED_CHAINS};

/// WVM forks. Let's consider DEV at first.
pub static WVM_DEV_FORKS: LazyLock<ChainHardforks> = LazyLock::new(|| {
    ChainHardforks::new(vec![
        (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Dao.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
        (
            EthereumHardfork::Paris.boxed(),
            ForkCondition::TTD { fork_block: None, total_difficulty: U256::ZERO },
        ),
        (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(1730192354)),
        (EthereumHardfork::Cancun.boxed(), ForkCondition::Timestamp(1730192418)),
    ])
});

pub(crate) const WVM_DEPOSIT_CONTRACT: DepositContract = DepositContract::new(
    address!("4242424242424242424242424242424242424242"),
    0,
    b256!("649bbc62d0e31342afea4e5cd82d4049e7e1ee912fc0889aa790803be39038c5"),
);

/// WVM dev testnet specification.
pub static WVM_DEV: LazyLock<Arc<ChainSpec>> = LazyLock::new(|| {
    ChainSpec {
        chain: Chain::from_id(999777),
        genesis: serde_json::from_str(include_str!("../../../../etc/dev-genesis.json"))
            .expect("Can't deserialize odyssey genesis json"),
        paris_block_and_final_difficulty: Some((0, U256::ZERO)),
        hardforks: WVM_DEV_FORKS.clone(),
        base_fee_params: BaseFeeParamsKind::Constant(BaseFeeParams::ethereum()),
        max_gas_limit: *ETHEREUM_BLOCK_GAS_LIMIT,
        deposit_contract: Some(WVM_DEPOSIT_CONTRACT),
        ..Default::default()
    }
    .into()
});

/// WVM chain specification parser.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WvmChainSpecParser;

impl ChainSpecParser for WvmChainSpecParser {
    type ChainSpec = ChainSpec;

    const SUPPORTED_CHAINS: &'static [&'static str] = &["dev"];

    fn parse(s: &str) -> eyre::Result<Arc<Self::ChainSpec>> {
        Ok(match s {
            "dev" => WVM_DEV.clone(),
            s => {
                let chainspec = ChainSpec::from(parse_genesis(s)?);
                Arc::new(chainspec)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::WvmChainSpecParser;
    use reth_cli::chainspec::ChainSpecParser;

    #[test]
    fn chainspec_parser_adds_prague() {
        let mut chainspec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        chainspec_path.push("../../../../etc/dev-genesis.json");

        WvmChainSpecParser::parse(&chainspec_path.to_string_lossy())
            .expect("could not parse chainspec");
    }
}
