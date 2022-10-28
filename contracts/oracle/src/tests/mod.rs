use currency::{
    lease::{Atom, Cro, Osmo, Wbtc, Weth},
    lpn::Usdc,
    native::Nls,
};
use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    percent::Percent,
    price::{self, dto::PriceDTO},
};
use sdk::cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier},
    MemoryStorage, MessageInfo, OwnedDeps,
};
use trees::tr;

use crate::{
    contract::{execute, instantiate},
    msg::{ExecuteMsg, InstantiateMsg},
    state::supported_pairs::TreeStore,
};

#[cfg(test)]
mod oracle_tests;

pub(crate) const CREATOR: &str = "creator";

pub(crate) fn dummy_instantiate_msg(
    base_asset: SymbolOwned,
    price_feed_period_secs: u32,
    expected_feeders: Percent,
    swap_tree: TreeStore,
    alarms_addr: String,
) -> InstantiateMsg {
    InstantiateMsg {
        base_asset,
        price_feed_period_secs,
        expected_feeders,
        swap_tree,
        timealarms_addr: alarms_addr,
    }
}

pub(crate) fn dummy_default_instantiate_msg() -> InstantiateMsg {
    dummy_instantiate_msg(
        Usdc::TICKER.to_string(),
        60,
        Percent::from_percent(50),
        TreeStore(
            tr((0, Usdc::TICKER.to_string()))
                / (tr((3, Weth::TICKER.to_string()))
                    / (tr((2, Atom::TICKER.to_string())) / tr((1, Osmo::TICKER.to_string()))))
                / (tr((4, Wbtc::TICKER.to_string())) / (tr((5, Cro::TICKER.to_string())))),
        ),
        "timealarms".to_string(),
    )
}

pub(crate) fn dummy_feed_prices_msg() -> ExecuteMsg {
    ExecuteMsg::FeedPrices {
        prices: vec![
            PriceDTO::try_from(price::total_of(Coin::<Osmo>::new(10)).is(Coin::<Atom>::new(12)))
                .unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<Atom>::new(10)).is(Coin::<Weth>::new(32)))
                .unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<Weth>::new(10)).is(Coin::<Usdc>::new(12)))
                .unwrap(),
            PriceDTO::try_from(price::total_of(Coin::<Wbtc>::new(10)).is(Coin::<Usdc>::new(120)))
                .unwrap(),
        ],
    }
}

pub(crate) fn setup_test(
    msg: InstantiateMsg,
) -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, MessageInfo) {
    let mut deps = mock_dependencies();
    let info = mock_info(CREATOR, &coins(1000, Nls::TICKER));
    let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(0, res.messages.len());

    // register single feeder address
    let msg = ExecuteMsg::RegisterFeeder {
        feeder_address: CREATOR.to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    (deps, info)
}
