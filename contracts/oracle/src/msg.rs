use cosmwasm_std::Addr;
use marketprice::storage::{Denom, DenomPair, PriceStorage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub base_asset: String,
    pub price_feed_period_secs: u32,
    pub feeders_percentage_needed: u8,
    pub supported_denom_pairs: Vec<DenomPair>,
    pub timealarms_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterFeeder {
        feeder_address: String,
    },
    FeedPrices {
        prices: Vec<PriceStorage>,
    },
    Config {
        price_feed_period_secs: u32,
        feeders_percentage_needed: u8,
    },
    SupportedDenomPairs {
        pairs: Vec<DenomPair>,
    },
    AddPriceAlarm {
        target: PriceStorage,
    },
    RemovePriceAlarm {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // returns the contract configuration
    Config {},
    // returns all registered feeders
    Feeders {},
    // check if an address belongs to a registered feeder
    IsFeeder { address: Addr },
    // returns the price of the denom against the base asset
    PriceFor { denoms: Vec<Denom> },
    // returns a list of supported denom pairs
    SupportedDenomPairs {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub base_asset: String,
    pub price_feed_period_secs: u32,
    pub feeders_percentage_needed: u8,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub prices: Vec<PriceStorage>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    Alarm(PriceStorage),
}
