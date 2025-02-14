use std::collections::HashSet;

use thiserror::Error;

use sdk::{
    cosmwasm_std::{Addr, DepsMut, StdError, StdResult, Storage},
    cw_storage_plus::Item,
};

/// Errors returned from Feeders
#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedersError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("Unauthorized")]
    Unauthorized {},
}

// state/logic
pub struct PriceFeeders<'f>(Item<'f, HashSet<Addr>>);

// this is the core business logic we expose
impl<'f> PriceFeeders<'f> {
    pub const fn new(namespace: &'f str) -> Self {
        Self(Item::new(namespace))
    }

    pub fn get(&self, storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        if self.0.may_load(storage)?.is_none() {
            return Ok(HashSet::new());
        }

        self.0.load(storage)
    }

    pub fn is_registered(&self, storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        if self.0.may_load(storage)?.is_none() {
            return Ok(false);
        }

        let addrs = self.0.load(storage)?;

        Ok(addrs.contains(address))
    }

    pub fn register(&self, deps: DepsMut<'_>, address: Addr) -> Result<(), PriceFeedersError> {
        let mut db = self.0.may_load(deps.storage)?.unwrap_or_default();

        if db.contains(&address) {
            return Err(PriceFeedersError::FeederAlreadyRegistered {});
        }

        db.insert(address);

        self.0.save(deps.storage, &db)?;

        Ok(())
    }

    pub fn remove(&self, deps: DepsMut<'_>, addr: Addr) -> Result<(), PriceFeedersError> {
        let remove_address = |mut addrs: HashSet<Addr>| -> StdResult<HashSet<Addr>> {
            addrs.remove(&addr);
            Ok(addrs)
        };

        if self.0.may_load(deps.storage)?.is_some() {
            self.0.update(deps.storage, remove_address)?;
        }

        Ok(())
    }
}
