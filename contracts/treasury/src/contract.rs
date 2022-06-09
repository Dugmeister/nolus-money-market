#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{self, ADMIN, REWARDS_DISPATCHER};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let admin = info.sender;
    ADMIN.save(deps.storage, &admin)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender;
    match msg {
        ExecuteMsg::ConfigureRewardTransfer { rewards_dispatcher } => {
            try_configure_reward_transfer(deps, sender, rewards_dispatcher)
        }
        ExecuteMsg::SendRewards { lpp_addr, amount } => {
            try_send_rewards(deps, sender, lpp_addr, amount)
        }
    }
}

fn try_configure_reward_transfer(
    deps: DepsMut,
    sender: Addr,
    rewards_dispatcher: Addr,
) -> Result<Response, ContractError> {
    state::assert_admin(deps.storage, sender)?;
    deps.api.addr_validate(rewards_dispatcher.as_str())?;
    REWARDS_DISPATCHER.save(deps.storage, &rewards_dispatcher)?;
    Ok(Response::new().add_attribute("method", "try_configure_reward_transfer"))
}

fn try_send_rewards(
    deps: DepsMut,
    sender: Addr,
    lpp_addr: Addr,
    amount: Coin,
) -> Result<Response, ContractError> {
    state::assert_rewards_dispatcher(deps.storage, sender)?;

    let pay_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        funds: vec![amount],
        contract_addr: lpp_addr.to_string(),
        msg: to_binary(&lpp::msg::ExecuteMsg::DistributeRewards {})?,
    });

    let response = Response::new()
        .add_attribute("method", "try_send_rewards")
        .add_message(pay_msg);

    Ok(response)
}
