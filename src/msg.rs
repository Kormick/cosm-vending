use std::str::FromStr;

use cosmwasm_schema::{cw_serde, QueryResponses};
use strum_macros::EnumIter;

use crate::errors::Error;

/// Kinds of snacks
#[derive(Copy, Eq, Hash, EnumIter)]
#[cw_serde]
pub enum Snack {
    Chocolate,
    Water,
    Chips,
}

impl std::fmt::Display for Snack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for Snack {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "chocolate" => Snack::Chocolate,
            "chips" => Snack::Chips,
            "water" => Snack::Water,
            _ => return Err(Error::UnknownItem(s.to_owned())),
        })
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the owner of the current contract instance
    pub owner: String,
    /// List with initial amounts of items
    pub initial_amount: Vec<(Snack, u64)>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the current amount of items available
    #[returns(ItemsCountResp)]
    ItemsCount,
}

#[cw_serde]
pub struct ItemsCountResp {
    /// List of items and their available amounts
    pub items: Vec<(Snack, u64)>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Decreases amount of specified item by 1
    GetItem(Snack),
    /// Increases amount of specified item by given amount
    Refill { item: Snack, amount: u64 },
}
