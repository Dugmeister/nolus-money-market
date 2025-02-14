use serde::{de::DeserializeOwned, Serialize};

use currency::native::Nls;
use finance::{coin::Coin, currency::Currency};
use platform::{
    bank::{self, BankAccount},
    batch::Batch,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Storage},
};

use crate::{
    error::ContractError,
    lpp::LiquidityPool,
    msg::{LppBalanceResponse, RewardsResponse},
    state::Deposit,
};

pub fn try_distribute_rewards(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let amount: Coin<Nls> = bank::received_one(info.funds)?;
    Deposit::distribute_rewards(deps, amount)?;

    Ok(Response::new().add_attribute("method", "try_distribute_rewards"))
}

pub fn try_claim_rewards(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    other_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let recipient = other_recipient
        .map(|recipient| deps.api.addr_validate(recipient.as_str()))
        .transpose()?
        .unwrap_or_else(|| info.sender.clone());

    let mut deposit =
        Deposit::may_load(deps.storage, info.sender)?.ok_or(ContractError::NoDeposit {})?;

    let reward = deposit.claim_rewards(deps.storage)?;

    if reward.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    let mut bank = bank::account(&env.contract.address, &deps.querier);
    bank.send(reward, &recipient);

    let batch: Batch = bank.into();

    let mut batch: Response = batch.into();
    batch = batch.add_attribute("method", "try_claim_rewards");
    Ok(batch)
}

pub fn query_lpp_balance<LPN>(
    deps: Deps<'_>,
    env: Env,
) -> Result<LppBalanceResponse<LPN>, ContractError>
where
    LPN: 'static + Currency + DeserializeOwned + Serialize,
{
    let lpp = LiquidityPool::<LPN>::load(deps.storage)?;
    lpp.query_lpp_balance(&deps, &env)
}

pub fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse, ContractError> {
    let rewards = Deposit::may_load(storage, addr)?
        .ok_or(ContractError::NoDeposit {})?
        .query_rewards(storage)?;

    Ok(RewardsResponse { rewards })
}

#[cfg(test)]
mod test {
    use access_control::SingleUserAccess;
    use finance::{percent::Percent, test::currency::Usdc};
    use platform::coin_legacy;
    use sdk::cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR},
        Coin as CwCoin,
    };

    use crate::{borrow::InterestRate, contract::lender, state::Config};

    use super::*;

    type TheCurrency = Usdc;

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);

    #[test]
    fn test_claim_zero_rewards() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut lpp_balance = 0;
        let deposit = 20_000;

        SingleUserAccess::new_contract_owner(Addr::unchecked("admin"))
            .store(deps.as_mut().storage)
            .unwrap();

        LiquidityPool::<TheCurrency>::store(
            deps.as_mut().storage,
            Config::new(
                TheCurrency::TICKER.into(),
                1000u64.into(),
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
            ),
        )
        .unwrap();

        // no deposit
        let info = mock_info("lender", &[]);
        let response = try_claim_rewards(deps.as_mut(), env.clone(), info, None);
        assert_eq!(response, Err(ContractError::NoDeposit {}));

        lpp_balance += deposit;
        let info = mock_info("lender", &cwcoins(deposit));
        deps.querier
            .update_balance(MOCK_CONTRACT_ADDR, cwcoins(lpp_balance));
        lender::try_deposit::<TheCurrency>(deps.as_mut(), env.clone(), info).unwrap();

        // pending rewards == 0
        let info = mock_info("lender", &[]);
        let response = try_claim_rewards(deps.as_mut(), env, info, None);
        assert_eq!(response, Err(ContractError::NoRewards {}));
    }

    fn cwcoins<A>(amount: A) -> Vec<CwCoin>
    where
        A: Into<Coin<TheCurrency>>,
    {
        vec![coin_legacy::to_cosmwasm::<TheCurrency>(amount.into())]
    }
}
