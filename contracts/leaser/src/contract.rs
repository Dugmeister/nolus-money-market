use std::ops::Sub;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use lease::msg::InstantiateMsg as LeaseInstantiateMsg;

use crate::config::Config;
use crate::error::ContractError;
use crate::helpers::assert_sent_sufficient_coin;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, QuoteResponse};
use crate::state::{CONFIG, INSTANTIATE_REPLY_IDS, LEASES, PENDING_INSTANCE_CREATIONS};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config::new(info.sender, msg)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Borrow {} => try_borrow(deps, info.funds, info.sender),
    }
}

pub fn try_borrow(
    deps: DepsMut,
    amount: Vec<Coin>,
    sender: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    assert_sent_sufficient_coin(&amount, config.lease_minimal_downpayment)?;

    let instance_reply_id = INSTANTIATE_REPLY_IDS.next(deps.storage)?;
    PENDING_INSTANCE_CREATIONS.save(deps.storage, instance_reply_id, &sender)?;
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: config.lease_code_id,
                funds: amount,
                label: "lease".to_string(),
                msg: to_binary(&LeaseInstantiateMsg {
                    owner: sender.to_string(),
                })?,
            }),
            instance_reply_id,
        )]),
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Quote { downpayment } => to_binary(&query_quote(env, deps, downpayment)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_quote(_env: Env, deps: Deps, downpayment: Coin) -> StdResult<QuoteResponse> {
    // borrowUST = LeaseInitialLiability% * downpaymentUST / (1 - LeaseInitialLiability%)
    if downpayment.amount.is_zero() {
        return Err(StdError::generic_err(
            "cannot open lease with zero downpayment",
        ));
    }
    let config = CONFIG.load(deps.storage)?;
    let numerator = config.lease_initial_liability.numerator() * downpayment.amount;
    let denominator = Decimal::one()
        .sub(config.lease_initial_liability)
        .numerator();

    let borrow_amount = numerator / denominator;
    let total_amount = borrow_amount + downpayment.amount;

    Ok(QuoteResponse {
        total: Coin::new(total_amount.u128(), downpayment.denom.clone()),
        borrow: Coin::new(borrow_amount.u128(), downpayment.denom.clone()),
        annual_interest_rate: get_annual_interest_rate(deps, downpayment)?,
    })
}

#[cfg(not(test))]
fn get_annual_interest_rate(deps: Deps, downpayment: Coin) -> StdResult<Decimal> {
    use cosmwasm_std::{QueryRequest, WasmQuery};

    use crate::msg::{LPPQueryMsg, QueryQuoteResponse};

    let config = CONFIG.load(deps.storage)?;
    let query_msg: LPPQueryMsg = LPPQueryMsg::Quote {
        amount: downpayment,
    };
    let query_response: QueryQuoteResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.lpp_ust_addr.to_string(),
            msg: to_binary(&query_msg)?,
        }))?;
    match query_response {
        QueryQuoteResponse::QuoteInterestRate(rate) => Ok(rate),
        QueryQuoteResponse::NoLiquidity => Err(StdError::generic_err("NoLiquidity")),
    }
}

#[cfg(test)]
fn get_annual_interest_rate(_deps: Deps, _downpayment: Coin) -> StdResult<Decimal> {
    Ok(Decimal::one())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr_raw = parse_reply_instantiate_data(msg.clone())
        .map(|r| r.contract_address)
        .map_err(|_| ContractError::ParseError {})?;

    let contract_addr = deps.api.addr_validate(&contract_addr_raw)?;
    register_lease(deps, msg.id, contract_addr)
}

fn register_lease(deps: DepsMut, msg_id: u64, lease_addr: Addr) -> Result<Response, ContractError> {
    // TODO: Remove pending id if the creation was not successful
    let owner_addr = PENDING_INSTANCE_CREATIONS.load(deps.storage, msg_id)?;
    LEASES.save(deps.storage, &owner_addr, &lease_addr)?;
    PENDING_INSTANCE_CREATIONS.remove(deps.storage, msg_id);
    Ok(Response::new().add_attribute("lease_address", lease_addr))
}
