use std::error::Error;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Decimal, Uint256, Uint128, StdError, Coin, CosmosMsg};
use cw_utils::must_pay;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{OWNER, CONFIG, ALL_BIDS_PER_BIDDER, Config, HIGHEST_CURRENT_BID};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:certification-project";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

const FEE_SCALE_FACTOR: Uint128 = Uint128::new(10_000);
const MAX_FEE_PERCENT: &str = "1";
const FEE_DECIMAL_PRECISION: Uint128 = Uint128::new(10u128.pow(20));

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    let owner = msg
        .owner
        .map(|str_addr| deps.api.addr_validate(&str_addr))
        .transpose()?
        .unwrap_or_else(|| env.contract.address.clone());
    OWNER.save(deps.storage, &owner)?;

    let config = Config {
        required_native_denom: msg.required_native_denom,
        fee:  msg.fee,
        open_sale: true
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
    .add_attribute("action", "instantiate")
    .add_attribute("sender", info.sender)
    .add_attribute("owner", owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    match msg {
        ExecuteMsg::Bid {} => do_bid(deps, info),
        ExecuteMsg::Close {  } => close_bid_event(deps, info)
    }
}

pub fn close_bid_event(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {

    // validate that it's the owner trying to close the bid event
    let owner = OWNER.load(deps.storage)?;
    if owner != info.sender {
        return Err(ContractError::Unauthorized {  })
    }

    // validate that bid event is still open
    let config = CONFIG.load(deps.storage)?;
    if !config.open_sale {
        return Err(ContractError::Unauthorized {  })
    }

    CONFIG.update(deps.storage, |mut con| -> Result<Config, ContractError> {
        con.open_sale = false;
        Ok(con)
    })?;

    let (_, am) = HIGHEST_CURRENT_BID.load(deps.storage)?;

    let fee_amount = get_owner_fee_amount(am.clone(), config.fee)?;

    let amount_wo_fee = am - fee_amount;

    let winner_bid_to_owner_msg: CosmosMsg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: owner.into_string(),
        amount: vec![Coin {
            denom: config.required_native_denom,
            amount: amount_wo_fee,
        }],
    });
           
    Ok(Response::new()
        .add_message(winner_bid_to_owner_msg))
}

pub fn do_bid(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;

    if !config.open_sale {
        return Err(ContractError::Unauthorized {  })
    }

    let paid = must_pay(&info, config.required_native_denom.as_str()
        ).map_err(|_| ContractError::Unauthorized {  })?;

    
    let highest_bid: Uint128 = get_highest_bid(&deps)?;

    let total_user_bid = match ALL_BIDS_PER_BIDDER.may_load(deps.storage, info.sender.clone()) {
        Ok(amount) => {
            match amount {
                Some(a) => a,
                None => Uint128::zero()
            }
        },
        Err(_) => return Err(ContractError::Unauthorized {  })
    };

    if highest_bid >= total_user_bid + paid   {
        return Err(ContractError::Unauthorized {  })
    }

    ALL_BIDS_PER_BIDDER.update(deps.storage, info.sender, |ve| -> Result<_, ContractError> {
        match ve {
            Some(mut v) => {             
                v = v + paid;
                Ok(v)
            }
            None => {
                Ok(paid)
            }
        }
    })?;

    let fee_amount = get_owner_fee_amount(paid.clone(), config.fee)?;

    let owner = OWNER.load(deps.storage)?;

    let fee_to_owner_msg: CosmosMsg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: owner.into_string(),
        amount: vec![Coin {
            denom: config.required_native_denom,
            amount: fee_amount,
        }],
    });
           
    Ok(Response::new()
        .add_message(fee_to_owner_msg))
}

fn get_highest_bid(deps: &DepsMut) -> Result<Uint128,ContractError> {

    let hb = HIGHEST_CURRENT_BID.load(deps.storage);

    match hb {
        Ok(highest_bid) => Ok(highest_bid.1),
        Err(_) => Ok(Uint128::zero())
    }
}

fn get_owner_fee_amount(input_amount: Uint128, fee_percent: Decimal) -> StdResult<Uint128> {
    if fee_percent.is_zero() {
        return Ok(Uint128::zero());
    }

    let fee_percent = fee_decimal_to_uint128(fee_percent)?;
    Ok(input_amount
        .full_mul(fee_percent)
        .checked_div(Uint256::from(FEE_SCALE_FACTOR))
        .map_err(StdError::divide_by_zero)?
        .try_into()?)
}

fn fee_decimal_to_uint128(decimal: Decimal) -> StdResult<Uint128> {
    let result: Uint128 = decimal
        .atomics()
        .checked_mul(FEE_SCALE_FACTOR)
        .map_err(StdError::overflow)?;

    Ok(result / FEE_DECIMAL_PRECISION)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
