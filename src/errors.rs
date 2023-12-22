use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

use crate::msg::Snack;

#[derive(Debug, PartialEq, Error)]
pub enum Error {
    #[error("{sender} is not contract admin")]
    Unauthorized { sender: Addr },
    #[error("Item {0:?} is out of stock")]
    OutOfStock(Snack),
    #[error("Overflow while refilling item {item:?} with {amount} amount")]
    ItemOverflow { item: Snack, amount: u64 },
    #[error("Unknown item {0}")]
    UnknownItem(String),
    #[error("{0}")]
    StdError(#[from] StdError),
}
