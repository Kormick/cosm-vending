# Cosmwasm Vending Machine
This is simple smart contract for a vending machine.
Allows to get items from machine one by one, to refill machine with more items (only for owner), and to check total amount of items in the machine.

## Details
List of supported items
```rust
pub enum Snack {
    Chocolate,
    Water,
    Chips,
}
``` 

### Instantiation
To instantiate the contract, you should pass address of the owner and list with initial amounts of items.
```rust
pub struct InstantiateMsg {
    pub owner: String,
    pub initial_amount: Vec<(Snack, u64)>,
}
```

### Queries
 * `ItemsCount` - returns the list of amount of items available

### Execution

* `GetItem(item)`
Available to everyone. Reduces amount of the `item` by 1. Returns `OutOfStock` error if current amount of items is 0.

* `Refill { item, amount }`
Available only to contract owner. Increases the amount of `item` by `amount`. 
Returns `Unauthorized` error if requested not by contract owner.
Returns `ItemOverflow` error if total amount of `item` overflows `u64` type.

### Running the contract
This contract was built and tested with Rust 1.74.1 with `wasm32-unknown-unknown` target.

You can run unit tests with `cargo test`.

Also, was tested with [rust-optimizer](https://github.com/CosmWasm/rust-optimizer) and on Osmosis testnet.