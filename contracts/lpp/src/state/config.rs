use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::{currency::Currency, price::Price};
use sdk::{
    cosmwasm_std::{StdResult, Storage, Uint64},
    cw_storage_plus::Item,
};

use crate::{borrow::InterestRate, msg::InstantiateMsg, nlpn::NLpn};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Config {
    lpn_ticker: String,
    lease_code_id: Uint64,
    borrow_rate: InterestRate,
}

impl Config {
    const STORAGE: Item<'static, Self> = Item::new("config");

    #[cfg(test)]
    pub const fn new(lpn_ticker: String, lease_code_id: Uint64, borrow_rate: InterestRate) -> Self {
        Self {
            lpn_ticker,
            lease_code_id,
            borrow_rate,
        }
    }

    pub fn lpn_ticker(&self) -> &str {
        &self.lpn_ticker
    }

    pub const fn lease_code_id(&self) -> Uint64 {
        self.lease_code_id
    }

    pub const fn borrow_rate(&self) -> &InterestRate {
        &self.borrow_rate
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::STORAGE.save(storage, self)
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORAGE.load(storage)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Uint64) -> StdResult<()> {
        Self::STORAGE
            .update(storage, |mut config| {
                config.lease_code_id = lease_code;

                Ok(config)
            })
            .map(|_| ())
    }

    pub fn update_borrow_rate(
        storage: &mut dyn Storage,
        borrow_rate: InterestRate,
    ) -> StdResult<()> {
        Self::STORAGE
            .update(storage, |mut config| {
                config.borrow_rate = borrow_rate;

                Ok(config)
            })
            .map(|_| ())
    }

    pub fn initial_derivative_price<LPN>() -> Price<NLpn, LPN>
    where
        LPN: Currency + Serialize + DeserializeOwned,
    {
        Price::identity()
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        // 0 is a non-existing code id
        Self {
            lpn_ticker: msg.lpn_ticker,
            lease_code_id: Uint64::zero(),
            borrow_rate: msg.borrow_rate,
        }
    }
}
