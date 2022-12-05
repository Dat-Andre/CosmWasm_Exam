use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Uint64, Decimal};


#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub required_native_denom: String,
    pub fee: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    Bid { },
    Close { },

}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
