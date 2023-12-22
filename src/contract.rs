pub use self::{execute::execute, instantiate::instantiate, query::query};

mod instantiate {
    use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};

    use crate::{
        errors::Error,
        msg::InstantiateMsg,
        state::{OWNER, SNACKS},
    };

    /// Sets provided address as contract owner and stores initial amounts of items
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, Error> {
        OWNER.save(deps.storage, &deps.api.addr_validate(msg.owner.as_str())?)?;
        msg.initial_amount
            .into_iter()
            .try_for_each(|(snack, amount)| SNACKS.save(deps.storage, snack as u64, &amount))?;
        Ok(Response::new()
            .add_attribute("action", "instantiate")
            .add_attribute("owner", msg.owner.as_str()))
    }
}

mod query {
    use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, Env, StdResult};
    use strum::IntoEnumIterator;

    use crate::{
        msg::{ItemsCountResp, QueryMsg, Snack},
        state::SNACKS,
    };

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::ItemsCount => to_json_binary(&items_count(deps)?),
        }
    }

    /// Returns list of current amounts of items. If item is not in the storage, returns 0 amount
    fn items_count(deps: Deps) -> StdResult<ItemsCountResp> {
        Ok(ItemsCountResp {
            items: Snack::iter()
                .map(|snack| {
                    if SNACKS.has(deps.storage, snack as u64) {
                        SNACKS.load(deps.storage, snack as u64).map(|r| (snack, r))
                    } else {
                        Ok((snack, 0))
                    }
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

mod execute {
    use cosmwasm_std::{entry_point, DepsMut, Env, Event, MessageInfo, Response};

    use crate::{
        errors::Error,
        msg::{ExecuteMsg, Snack},
        state::{OWNER, SNACKS},
    };

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, Error> {
        match msg {
            ExecuteMsg::GetItem(item) => get_item(deps, item),
            ExecuteMsg::Refill { item, amount } => refill(deps, info, item, amount),
        }
    }

    /// Reduces the amount of specified item by 1.
    /// Return `OutOfStock` error if current item amount is 0
    fn get_item(deps: DepsMut, item: Snack) -> Result<Response, Error> {
        let total_amount = SNACKS.update(deps.storage, item as u64, |count| -> Result<_, _> {
            count
                .unwrap_or_default()
                .checked_sub(1)
                .ok_or(Error::OutOfStock(item))
        })?;
        Ok(Response::new()
            .add_event(
                Event::new("item_retrieved")
                    .add_attribute("item", item.to_string())
                    .add_attribute("total_amount", total_amount.to_string()),
            )
            .add_attribute("action", "get_item"))
    }

    /// Increases the amount of specified item by given amount
    /// Returns `Unauthorized` error if request sent not by contract owner
    /// Returns `ItemOverflow` error if total amounts of items overflows `u64`
    fn refill(
        deps: DepsMut,
        info: MessageInfo,
        item: Snack,
        amount: u64,
    ) -> Result<Response, Error> {
        if OWNER.load(deps.storage)? != info.sender {
            return Err(Error::Unauthorized {
                sender: info.sender,
            });
        }

        let total = SNACKS.update(deps.storage, item as u64, |count| {
            count
                .unwrap_or_default()
                .checked_add(amount)
                .ok_or(Error::ItemOverflow { item, amount })
        })?;
        Ok(Response::new().add_attribute("action", "refill").add_event(
            Event::new("item_refilled")
                .add_attribute("item", item.to_string())
                .add_attribute("amount", amount.to_string())
                .add_attribute("total_amount", total.to_string()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        from_json,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };
    use cw_multi_test::{App, ContractWrapper, Executor};

    use crate::{
        errors::Error,
        msg::{ExecuteMsg, InstantiateMsg, ItemsCountResp, QueryMsg, Snack},
    };

    use super::*;

    fn default_app(init: InstantiateMsg) -> (App, Addr) {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &init,
                &[],
                "Contract",
                None,
            )
            .unwrap();
        (app, addr)
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let init = InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)],
        };
        let resp = instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("owner", &[]),
            init.clone(),
        )
        .unwrap();
        assert_eq!(
            resp.attributes
                .iter()
                .find(|a| a.key == "action")
                .unwrap()
                .value,
            "instantiate"
        );
        assert_eq!(
            resp.attributes
                .iter()
                .find(|a| a.key == "owner")
                .unwrap()
                .value,
            init.owner
        );

        let resp = query(deps.as_ref(), env, QueryMsg::ItemsCount).unwrap();
        let resp: ItemsCountResp = from_json(resp).unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 1)));
        assert!(resp.items.contains(&(Snack::Chips, 2)));
        assert!(resp.items.contains(&(Snack::Water, 3)));
    }

    #[test]
    fn test_get_items_count() {
        let init = InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)],
        };
        let (app, addr) = default_app(init.clone());

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();

        assert!(resp.items.contains(&(Snack::Chocolate, 1)));
        assert!(resp.items.contains(&(Snack::Chips, 2)));
        assert!(resp.items.contains(&(Snack::Water, 3)));
    }

    #[test]
    fn test_get_items_with_empty_init() {
        let init = InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![],
        };
        let (app, addr) = default_app(init.clone());

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();

        assert!(resp.items.contains(&(Snack::Chocolate, 0)));
        assert!(resp.items.contains(&(Snack::Chips, 0)));
        assert!(resp.items.contains(&(Snack::Water, 0)));
    }

    #[test]
    fn test_get_items() {
        let init = InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)],
        };
        let (mut app, addr) = default_app(init.clone());

        for (item, total) in [(Snack::Chocolate, 0), (Snack::Chips, 1), (Snack::Water, 2)] {
            let resp = app
                .execute_contract(
                    Addr::unchecked("user"),
                    addr.clone(),
                    &ExecuteMsg::GetItem(item),
                    &[],
                )
                .unwrap();
            let wasm = resp.events.iter().find(|e| e.ty == "wasm").unwrap();
            assert_eq!(
                wasm.attributes
                    .iter()
                    .find(|a| a.key == "action")
                    .unwrap()
                    .value,
                "get_item"
            );
            let event = resp
                .events
                .iter()
                .find(|e| e.ty == "wasm-item_retrieved")
                .unwrap();
            assert_eq!(
                event
                    .attributes
                    .iter()
                    .find(|a| a.key == "item")
                    .unwrap()
                    .value,
                item.to_string()
            );
            assert_eq!(
                event
                    .attributes
                    .iter()
                    .find(|a| a.key == "total_amount")
                    .unwrap()
                    .value,
                total.to_string(),
            );
        }

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 0)));
        assert!(resp.items.contains(&(Snack::Chips, 1)));
        assert!(resp.items.contains(&(Snack::Water, 2)));
    }

    #[test]
    fn test_get_item_with_underflow() {
        let (mut app, addr) = default_app(InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 0), (Snack::Chips, 1), (Snack::Water, 1)],
        });

        let err = app
            .execute_contract(
                Addr::unchecked("user"),
                addr.clone(),
                &ExecuteMsg::GetItem(Snack::Chocolate),
                &[],
            )
            .unwrap_err();
        assert_eq!(Error::OutOfStock(Snack::Chocolate), err.downcast().unwrap());

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 0)));
        assert!(resp.items.contains(&(Snack::Chips, 1)));
        assert!(resp.items.contains(&(Snack::Water, 1)));
    }

    #[test]
    fn test_refill() {
        let init = InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)],
        };
        let (mut app, addr) = default_app(init);

        for (snack, amount, total) in [
            (Snack::Chocolate, 1, 2),
            (Snack::Chips, 2, 4),
            (Snack::Water, 3, 6),
        ] {
            let resp = app
                .execute_contract(
                    Addr::unchecked("owner"),
                    addr.clone(),
                    &ExecuteMsg::Refill {
                        item: snack,
                        amount,
                    },
                    &[],
                )
                .unwrap();
            let wasm = resp.events.iter().find(|e| e.ty == "wasm").unwrap();
            assert_eq!(
                wasm.attributes
                    .iter()
                    .find(|a| a.key == "action")
                    .unwrap()
                    .value,
                "refill"
            );
            let events = resp
                .events
                .iter()
                .filter(|e| e.ty == "wasm-item_refilled")
                .collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(
                events[0]
                    .attributes
                    .iter()
                    .find(|a| a.key == "item")
                    .unwrap()
                    .value,
                snack.to_string()
            );
            assert_eq!(
                events[0]
                    .attributes
                    .iter()
                    .find(|a| a.key == "amount")
                    .unwrap()
                    .value,
                amount.to_string()
            );
            assert_eq!(
                events[0]
                    .attributes
                    .iter()
                    .find(|a| a.key == "total_amount")
                    .unwrap()
                    .value,
                total.to_string()
            );
        }

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 2)));
        assert!(resp.items.contains(&(Snack::Chips, 4)));
        assert!(resp.items.contains(&(Snack::Water, 6)));
    }

    #[test]
    fn test_refill_unauthorized() {
        let (mut app, addr) = default_app(InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)],
        });

        let err = app
            .execute_contract(
                Addr::unchecked("user"),
                addr.clone(),
                &ExecuteMsg::Refill {
                    item: Snack::Chocolate,
                    amount: 1,
                },
                &[],
            )
            .unwrap_err();
        assert_eq!(
            Error::Unauthorized {
                sender: Addr::unchecked("user")
            },
            err.downcast().unwrap()
        );

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 1)));
        assert!(resp.items.contains(&(Snack::Chips, 2)));
        assert!(resp.items.contains(&(Snack::Water, 3)));
    }

    #[test]
    fn test_refill_with_overflow() {
        let (mut app, addr) = default_app(InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)],
        });

        let err = app
            .execute_contract(
                Addr::unchecked("owner"),
                addr.clone(),
                &ExecuteMsg::Refill {
                    item: Snack::Chocolate,
                    amount: u64::MAX,
                },
                &[],
            )
            .unwrap_err();
        assert_eq!(
            Error::ItemOverflow {
                item: Snack::Chocolate,
                amount: u64::MAX,
            },
            err.downcast().unwrap()
        );
        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 1)));
        assert!(resp.items.contains(&(Snack::Chips, 2)));
        assert!(resp.items.contains(&(Snack::Water, 3)));
    }

    #[test]
    fn test_refill_with_empty_init() {
        let (mut app, addr) = default_app(InstantiateMsg {
            owner: Addr::unchecked("owner").into_string(),
            initial_amount: vec![],
        });

        for (snack, amount) in [(Snack::Chocolate, 1), (Snack::Chips, 2), (Snack::Water, 3)] {
            let resp = app
                .execute_contract(
                    Addr::unchecked("owner"),
                    addr.clone(),
                    &ExecuteMsg::Refill {
                        item: snack,
                        amount,
                    },
                    &[],
                )
                .unwrap();
            let wasm = resp.events.iter().find(|e| e.ty == "wasm").unwrap();
            assert_eq!(
                wasm.attributes
                    .iter()
                    .find(|a| a.key == "action")
                    .unwrap()
                    .value,
                "refill"
            );
            let events = resp
                .events
                .iter()
                .filter(|e| e.ty == "wasm-item_refilled")
                .collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(
                events[0]
                    .attributes
                    .iter()
                    .find(|a| a.key == "item")
                    .unwrap()
                    .value,
                snack.to_string()
            );
            assert_eq!(
                events[0]
                    .attributes
                    .iter()
                    .find(|a| a.key == "amount")
                    .unwrap()
                    .value,
                amount.to_string()
            );
            assert_eq!(
                events[0]
                    .attributes
                    .iter()
                    .find(|a| a.key == "total_amount")
                    .unwrap()
                    .value,
                amount.to_string()
            );
        }

        let resp: ItemsCountResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::ItemsCount)
            .unwrap();
        assert!(resp.items.contains(&(Snack::Chocolate, 1)));
        assert!(resp.items.contains(&(Snack::Chips, 2)));
        assert!(resp.items.contains(&(Snack::Water, 3)));
    }
}
