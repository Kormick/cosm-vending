use cosm_vending::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    cosmwasm_schema::write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }
}
