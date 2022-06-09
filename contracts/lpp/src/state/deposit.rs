use crate::error::ContractError;
use crate::lpp::NTokenPrice;
use cosmwasm_std::{
    coin, Addr, BankMsg, Coin, Decimal, Deps, DepsMut, Env, Fraction, StdResult, Storage, Uint128,
};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

type Balance = Uint128;
// TODO: move to some global config package
pub const NOLUS_DENOM: &str = "uNLS";

pub struct Deposit {
    addr: Addr,
    data: DepositData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositData {
    pub deposited_nlpn: Balance,

    // Rewards
    pub reward_per_token: Decimal,
    pub pending_rewards_nls: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositsGlobals {
    pub balance_nlpn: Balance,

    // Rewards
    pub balance_nls: Balance,
    pub reward_per_token: Decimal,
}

impl Deposit {
    const DEPOSITS: Map<'static, Addr, DepositData> = Map::new("deposits");
    const GLOBALS: Item<'static, DepositsGlobals> = Item::new("deposits_globals");

    pub fn load(storage: &dyn Storage, addr: Addr) -> StdResult<Self> {
        let data = Self::DEPOSITS
            .may_load(storage, addr.clone())?
            .unwrap_or_default();

        Ok(Self { addr, data })
    }

    // TODO: forbid zero amount_lnp deposit
    pub fn deposit(
        &mut self,
        storage: &mut dyn Storage,
        amount_lnp: Uint128,
        price: NTokenPrice,
    ) -> StdResult<()> {
        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        let inv_price = price.get().inv().expect("price should not be zero");
        let deposited_nlpn = inv_price * amount_lnp;
        self.data.deposited_nlpn += deposited_nlpn;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        globals.balance_nlpn += deposited_nlpn;

        Self::GLOBALS.save(storage, &globals)
    }

    /// return optional reward payment msg in case of deleting account
    pub fn withdraw(
        &mut self,
        storage: &mut dyn Storage,
        amount_nlpn: Uint128,
    ) -> Result<Option<BankMsg>, ContractError> {
        if self.data.deposited_nlpn < amount_nlpn {
            return Err(ContractError::InsufficientBalance);
        }

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        self.data.deposited_nlpn -= amount_nlpn;
        globals.balance_nlpn -= amount_nlpn;

        let maybe_reward = if self.data.deposited_nlpn.is_zero() {
            let reward: Uint128 = self.data.pending_rewards_nls * Uint128::new(1);
            globals.balance_nls -= reward;
            Self::DEPOSITS.remove(storage, self.addr.clone());
            Some(Self::pay_nls(self.addr.clone(), reward))
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
            None
        };

        Self::GLOBALS.save(storage, &globals)?;

        Ok(maybe_reward)
    }

    // NOTE: a chain of messages: BankMsg and SendRewards
    pub fn distribute_rewards(deps: DepsMut, env: Env, funds: Vec<Coin>) -> StdResult<()> {
        println!("Distributing: {:?}", funds);
        let current_balance_nls = Self::balance_nls(&deps.as_ref(), &env)?;
        let mut globals = Self::GLOBALS.may_load(deps.storage)?.unwrap_or_default();

        globals.reward_per_token += Decimal::from_ratio(
            current_balance_nls - globals.balance_nls,
            globals.balance_nlpn,
        );
        globals.balance_nls = current_balance_nls;
        Self::GLOBALS.save(deps.storage, &globals)
    }

    fn update_rewards(&mut self, globals: &DepositsGlobals) {
        self.data.pending_rewards_nls = self.calculate_reward(globals);
        self.data.reward_per_token = globals.reward_per_token;
    }

    fn calculate_reward(&self, globals: &DepositsGlobals) -> Decimal {
        let deposit = &self.data;
        deposit.pending_rewards_nls
            + (globals.reward_per_token - deposit.reward_per_token)
                * Decimal::from_ratio(deposit.deposited_nlpn.u128(), 1u128)
    }

    pub fn query_rewards(&self, storage: &dyn Storage) -> StdResult<Coin> {
        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();

        let reward = self.calculate_reward(&globals) * Uint128::new(1);
        Ok(coin(reward.u128(), NOLUS_DENOM))
    }

    pub fn claim_rewards(
        &mut self,
        storage: &mut dyn Storage,
        recipient: Option<Addr>,
    ) -> StdResult<BankMsg> {
        let recipient = recipient.unwrap_or_else(|| self.addr.clone());

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();

        let reward = self.calculate_reward(&globals) * Uint128::new(1);
        globals.balance_nls -= reward;
        self.data.pending_rewards_nls -= Decimal::from_ratio(reward.u128(), 1u128);

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
        Self::GLOBALS.save(storage, &globals)?;

        Ok(Self::pay_nls(recipient, reward))
    }

    pub fn pay_nls(addr: Addr, amount: Uint128) -> BankMsg {
        BankMsg::Send {
            to_address: addr.to_string(),
            amount: vec![coin(amount.u128(), NOLUS_DENOM)],
        }
    }

    pub fn balance_nlpn(storage: &dyn Storage) -> StdResult<Balance> {
        Ok(Self::GLOBALS
            .may_load(storage)?
            .unwrap_or_default()
            .balance_nlpn)
    }

    pub fn query_balance_nlpn(storage: &dyn Storage, addr: Addr) -> StdResult<Option<Balance>> {
        let maybe_balance = Self::DEPOSITS
            .may_load(storage, addr)?
            .map(|data| data.deposited_nlpn);
        Ok(maybe_balance)
    }

    pub fn balance_nls(deps: &Deps, env: &Env) -> StdResult<Uint128> {
        let querier = deps.querier;
        querier
            .query_balance(&env.contract.address, NOLUS_DENOM)
            .map(|coin| coin.amount)
    }
}
