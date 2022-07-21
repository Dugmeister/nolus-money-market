use std::time::SystemTime;

use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{Api, DepsMut, Timestamp};

use crate::feeders::PriceFeeders;
use crate::market_price::{PriceFeeds, PriceFeedsError, PriceQuery};
use crate::storage::PriceStorage;
use finance::duration::Duration;

const MINUTE: Duration = Duration::from_secs(60);

#[test]
fn register_feeder() {
    let mut deps = mock_dependencies();

    let control = PriceFeeders::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();
    let resp = control.is_registered(&deps.storage, &f_address).unwrap();
    assert!(!resp);

    control.register(deps.as_mut(), f_address.clone()).unwrap();

    let resp = control.is_registered(&deps.storage, &f_address).unwrap();
    assert!(resp);

    let feeders = control.get(&deps.storage).unwrap();
    assert_eq!(1, feeders.len());

    // should return error that address is already added
    let res = control.register(deps.as_mut(), f_address);
    assert!(res.is_ok());

    let f_address = deps.api.addr_validate("address2").unwrap();
    control.register(deps.as_mut(), f_address).unwrap();

    let f_address = deps.api.addr_validate("address3").unwrap();
    control.register(deps.as_mut(), f_address).unwrap();

    let feeders = control.get(&deps.storage).unwrap();
    assert_eq!(3, feeders.len());
}

#[test]
fn marketprice_add_feed_expect_err() {
    let deps = mock_dependencies();
    let market = PriceFeeds::new("foo");

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());
    let query = PriceQuery::new(("DEN1".to_string(), "DEN2".to_string()), MINUTE, 50);
    let expected_err = market.get(&deps.storage, ts, query).unwrap_err();
    assert_eq!(expected_err, PriceFeedsError::NoPrice {});
}

#[test]
fn marketprice_add_feed_empty_vec() {
    let mut deps = mock_dependencies();

    let market = PriceFeeds::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let prices: Vec<PriceStorage> = Vec::new();
    market
        .feed(&mut deps.storage, ts, &f_address, prices, MINUTE)
        .unwrap();
}

#[test]
fn marketprice_add_feed() {
    let mut deps = mock_dependencies();

    let market = PriceFeeds::new("foo");
    let f_address = deps.api.addr_validate("address1").unwrap();

    let prices: Vec<PriceStorage> = vec![
        PriceStorage::new("DEN1".to_string(), 10, "DEN2".to_string(), 5),
        PriceStorage::new(
            "DEN1".to_string(),
            10000000000,
            "DEN3".to_string(),
            1000000009,
        ),
        PriceStorage::new(
            "DEN1".to_string(),
            10000000000000,
            "DEN4".to_string(),
            100000000000002,
        ),
    ];

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    market
        .feed(&mut deps.storage, ts, &f_address, prices, MINUTE)
        .unwrap();
    let query = PriceQuery::new(("DEN1".to_string(), "DEN2".to_string()), MINUTE, 50);
    let err = market.get(&deps.storage, ts, query).unwrap_err();
    assert_eq!(err, PriceFeedsError::NoPrice {});

    let query = PriceQuery::new(("DEN1".to_string(), "DEN2".to_string()), MINUTE, 1);
    let price_resp = market.get(&deps.storage, ts, query).unwrap();
    let expected_price = PriceStorage::new("DEN1".to_string(), 10, "DEN2".to_string(), 5);
    assert_eq!(expected_price, price_resp);
}

#[test]
fn marketprice_follow_the_path() {
    let mut deps = mock_dependencies();
    let market = PriceFeeds::new("foo");
    let _ = feed_price(deps.as_mut(), &market, "DEN1".into(), 1, "DEN0".into(), 1).unwrap();

    let _ = feed_price(deps.as_mut(), &market, "DEN3".into(), 1, "DEN4".into(), 3).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DEN3".into(), 1, "DENX".into(), 3).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DEN1".into(), 1, "DEN2".into(), 1).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DEN3".into(), 1, "DEN4".into(), 3).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DEN2".into(), 1, "DEN3".into(), 2).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DEN3".into(), 1, "DEN2".into(), 3).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DENZ".into(), 1, "DENX".into(), 3).unwrap();
    let _ = feed_price(deps.as_mut(), &market, "DEN4".into(), 1, "DEN1".into(), 3).unwrap();
    let ts = feed_price(deps.as_mut(), &market, "DENC".into(), 1, "DEN4".into(), 3).unwrap();

    // valid search denom pair
    let query = PriceQuery::new(("DEN1".to_string(), "DEN4".to_string()), MINUTE, 1);
    let price_resp = market.get(&deps.storage, ts, query).unwrap();
    let expected = PriceStorage::new("DEN1".into(), 1, "DEN4".into(), 6);
    assert_eq!(expected, price_resp);

    // first and second part of denom pair are the same
    let query = PriceQuery::new(("DEN1".to_string(), "DEN1".to_string()), MINUTE, 1);
    let price_resp = market.get(&deps.storage, ts, query).unwrap();
    let expected = PriceStorage::new("DEN1".into(), 1, "DEN1".into(), 1);
    assert_eq!(expected, price_resp);

    // second part of denome pair doesn't exists in the storage
    let query = PriceQuery::new(("DEN1".to_string(), "DEN5".to_string()), MINUTE, 1);
    assert_eq!(
        market.get(&deps.storage, ts, query).unwrap_err(),
        PriceFeedsError::NoPrice {}
    );

    // first part of denome pair doesn't exists in the storage
    let query = PriceQuery::new(("DEN6".to_string(), "DEN1".to_string()), MINUTE, 1);
    assert_eq!(
        market.get(&deps.storage, ts, query).unwrap_err(),
        PriceFeedsError::NoPrice {}
    );
}

fn feed_price(
    deps: DepsMut,
    market: &PriceFeeds,
    sym_base: String,
    amount_base: u128,
    sym_quote: String,
    amount_quote: u128,
) -> Result<Timestamp, PriceFeedsError> {
    let f_address = deps.api.addr_validate("address1").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let ts = Timestamp::from_seconds(now.as_secs());

    let price = PriceStorage::new(sym_base, amount_base, sym_quote, amount_quote);
    market.feed(deps.storage, ts, &f_address, vec![price], MINUTE)?;
    Ok(ts)
}
