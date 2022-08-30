use near_sdk::PromiseOrValue;

use crate::*;

const MIN_SLIPPAGE_ALLOWED: u128 = 1;

#[near_bindgen]
impl Contract {
    /// Check if farm still have rewards to distribute (status == Running)
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn stable_callback_list_farms_by_seed(
        &mut self,
        #[callback_result] farms_result: Result<Vec<FarmInfoBoost>, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<String> {
        assert!(farms_result.is_ok(), "ERR_LIST_FARMS_FAILED");

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        let farms = farms_result.unwrap();

        // Try to unclaim before change to Ended
        for farm in farms.iter() {
            if farm.farm_id == farm_id && farm.status != *"Running" {
                let stable_compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

                for strat_farm in stable_compounder.farms.iter_mut() {
                    if strat_farm.id == farm_id {
                        strat_farm.state = AutoCompounderState::Ended;
                    }
                }
            }
        }

        let farm_contract_id = self
            .get_strat(&seed_id)
            .get_stable_compounder_ref()
            .farm_contract_id
            .clone();

        PromiseOrValue::Promise(
            ext_ref_farming::get_unclaimed_rewards(
                env::current_account_id(),
                seed_id,
                farm_contract_id,
                0,
                Gas(3_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_post_get_unclaimed_reward(
                    farm_id_str,
                    env::current_account_id(),
                    0,
                    Gas(70_000_000_000_000),
                ),
            ),
        )
    }

    #[private]
    pub fn stable_callback_post_get_unclaimed_reward(
        &mut self,
        #[callback_result] reward_amount_result: Result<HashMap<String, U128>, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128> {
        assert!(reward_amount_result.is_ok(), "ERR_GET_REWARD_FAILED");

        let mut rewards_map = reward_amount_result.unwrap();

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

        let farm_info = compounder.get_mut_farm_info(&farm_id);

        for (token, amount) in rewards_map.iter() {
            log!("token: {} amount: {}", token, amount.0);
        }

        // this should never panics from .unwrap(), given that the reward_token was previously known
        let reward_amount: U128 = if rewards_map.contains_key(&farm_info.reward_token.to_string()) {
            rewards_map
                .remove(&farm_info.reward_token.to_string())
                .unwrap()
        } else {
            U128(0)
        };

        if reward_amount.0 == 0u128 {
            // if farm is ended, there is no more actions to do
            if farm_info.state == AutoCompounderState::Ended {
                farm_info.state = AutoCompounderState::Cleared;
                return PromiseOrValue::Value(0u128);
            } else {
                panic!("ERR: zero rewards earned")
            }
        }

        PromiseOrValue::Promise(
            ext_ref_farming::claim_reward_by_seed(
                seed_id,
                compounder.farm_contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_post_claim_reward(
                    farm_id_str,
                    reward_amount,
                    rewards_map,
                    env::current_account_id(),
                    0,
                    Gas(10_000_000_000_000),
                ),
            ),
        )
    }

    #[private]
    pub fn stable_callback_post_claim_reward(
        &mut self,
        #[callback_result] claim_reward_result: Result<(), PromiseError>,
        farm_id_str: String,
        reward_amount: U128,
        rewards_map: HashMap<String, U128>,
    ) -> u128 {
        assert!(claim_reward_result.is_ok(), "ERR_WITHDRAW_FAILED");

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        // update strategies with the same seed
        let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();
        compounder.update_strats_by_seed(rewards_map);

        // store the amount of reward earned
        let farm_info = compounder.get_mut_farm_info(&farm_id);
        farm_info.last_reward_amount += reward_amount.0;

        farm_info.next_cycle();

        reward_amount.0
    }

    #[private]
    pub fn stable_callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<bool, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<U128> {
        assert!(withdraw_result.is_ok(), "ERR_WITHDRAW_FROM_FARM_FAILED");

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let data_mut = self.data_mut();

        let compounder = data_mut
            .strategies
            .get_mut(&seed_id)
            .expect(ERR21_TOKEN_NOT_REG)
            .get_stable_compounder_mut();

        let last_reward_amount = compounder.get_mut_farm_info(&farm_id).last_reward_amount;

        let (remaining_amount, protocol_amount, sentry_amount, strat_creator_amount) =
            compounder.compute_fees(last_reward_amount);

        // storing the amount earned by the strat creator
        compounder.admin_fees.strat_creator.current_amount += strat_creator_amount;

        // store sentry amount under contract account id to be used in the last step
        compounder
            .admin_fees
            .sentries
            .insert(env::current_account_id(), sentry_amount);

        // increase protocol amount to cover the case that the last transfer failed
        data_mut.treasury.current_amount += protocol_amount;

        // remaining amount to reinvest
        compounder.get_mut_farm_info(&farm_id).last_reward_amount = remaining_amount;

        // amount sent to ref, both remaining value and treasury
        let amount = remaining_amount + protocol_amount;

        PromiseOrValue::Promise(
            ext_reward_token::ft_transfer_call(
                compounder.exchange_contract_id.clone(),
                U128(amount), //Amount after withdraw the rewards
                "".to_string(),
                compounder.get_mut_farm_info(&farm_id).reward_token.clone(),
                1,
                Gas(40_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_post_ft_transfer(
                    farm_id_str,
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            ),
        )
    }

    #[private]
    pub fn stable_callback_post_ft_transfer(
        &mut self,
        #[callback_result] exchange_transfer_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) {
        if exchange_transfer_result.is_err() {
            log!("ERR_TRANSFER_TO_EXCHANGE");
            return;
        }

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();
        let farm_info_mut = compounder.get_mut_farm_info(&farm_id);

        farm_info_mut.next_cycle();
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn stable_callback_post_treasury_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
    ) {
        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!("Transfer to treasure failed");
            return;
        }

        let data_mut = self.data_mut();
        let amount: u128 = data_mut.treasury.current_amount;

        // reset treasury amount earned since tx was successful
        data_mut.treasury.current_amount = 0;

        log!("Stable Transfer {} to treasure succeeded", amount)
    }

    #[private]
    pub fn stable_callback_post_creator_ft_transfer(
        &mut self,
        #[callback_result] strat_creator_transfer_result: Result<(), PromiseError>,
        seed_id: String,
    ) {
        if strat_creator_transfer_result.is_err() {
            log!("ERR_TRANSFER_TO_CREATOR");
            return;
        }

        let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

        // what if a new value was added to this var during the completion of this execution?
        // tx0 (add strat_creator_fees) -> tx1 (send strat_creator fees) -> tx2 -> (add strat_creator_fees) -> tx3 (update current amount to 0, because value was already sent)
        // this means that the value from tx2 was never sent to the strat_creator, losing the earned tokens
        compounder.admin_fees.strat_creator.current_amount = 0;

        log!("Transfer fees to the creator of the strategy succeeded");
    }

    #[private]
    pub fn stable_callback_get_token_return(
        &mut self,
        #[callback_result] token_out: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128> {
        assert!(token_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_RETURN");

        let mut min_amount_out: U128 = token_out.unwrap();

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let stable_compounder = self.get_strat(&seed_id).get_stable_compounder();

        let farm_info = stable_compounder.get_farm_info(&farm_id);

        let percent = Percentage::from(farm_info.slippage);

        min_amount_out = U128(percent.apply_to(min_amount_out.0));

        let amount_in: U128 = U128(farm_info.last_reward_amount);

        log!(
            "min amount out: {} for {}",
            min_amount_out.0,
            farm_info.token_address,
        );

        if min_amount_out.0 == 0u128 {
            log!("ERR_COULD_NOT_GET_TOKEN_RETURN");
            let stable_compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();
            let farm_info_mut = stable_compounder.get_mut_farm_info(&farm_id);
            farm_info_mut.increase_slippage();

            return PromiseOrValue::Value(0u128);
        }

        PromiseOrValue::Promise(
            self.call_swap(
                stable_compounder.exchange_contract_id,
                farm_info.pool_id_token_reward,
                farm_info.reward_token,
                // TODO: what if I want the strategies to stake different token address from the tokens availables?
                farm_info.token_address,
                Some(amount_in),
                min_amount_out,
            )
            .then(callback_stable_ref_finance::stable_callback_post_swap(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(80_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn stable_callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);
        let compounder_mut = self.get_strat_mut(&seed_id).get_stable_compounder_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(&farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            farm_info_mut.increase_slippage();
            log!("ERR_SECOND_SWAP_FAILED");
            return;
        }

        // no more rewards to spend
        farm_info_mut.last_reward_amount = 0;

        let amount_earned = swap_result.unwrap();

        farm_info_mut.available_balance[farm_info_mut.token_position as usize] = amount_earned.0;

        // reset slippage
        farm_info_mut.slippage = 100 - MIN_SLIPPAGE_ALLOWED;

        // after both swaps succeeded, it's ready to stake
        farm_info_mut.next_cycle();
    }

    #[private]
    pub fn stable_callback_post_sentry(
        &mut self,
        #[callback_result] result: Result<Option<StorageBalance>, PromiseError>,
        farm_id_str: String,
        sentry_acc_id: AccountId,
        reward_token: AccountId,
    ) -> Promise {
        // TODO: propagate error
        match result {
            Ok(balance_op) => match balance_op {
                Some(balance) => assert!(
                    balance.total.0 > 1,
                    "ERR: account does not have enough funds to pay for storage"
                ),
                _ => {
                    let msg = format!(
                        "{}{:#?}",
                        "ERR: callback_post_sentry - not enough balance on storage",
                        balance_op
                            .unwrap_or(StorageBalance {
                                total: U128(0),
                                available: U128(0)
                            })
                            .total
                    );
                    env::panic_str(msg.as_str());
                }
            },
            Err(_) => env::panic_str(
                "ERR: callback post Sentry - caller not registered to Reward token contract",
            ),
        }

        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.clone());
        let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

        // reset default sentry address and get last earned amount
        let amount = compounder
            .admin_fees
            .sentries
            .remove(&env::current_account_id())
            .unwrap();

        log!("Sending {} to sentry account {}", amount, sentry_acc_id);

        ext_reward_token::ft_transfer(
            sentry_acc_id.clone(),
            U128(amount),
            Some("".to_string()),
            reward_token,
            1,
            Gas(20_000_000_000_000),
        )
        .then(
            callback_stable_ref_finance::stable_callback_post_sentry_mft_transfer(
                farm_id_str,
                sentry_acc_id,
                amount,
                env::current_account_id(),
                0,
                Gas(200_000_000_000_000),
            ),
        )
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn stable_callback_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        farm_id_str: String,
        sentry_id: AccountId,
        amount_earned: u128,
    ) -> PromiseOrValue<u64> {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!("Transfer to sentry failed".to_string());

            let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

            // store amount earned by sentry to be redeemed
            compounder
                .admin_fees
                .sentries
                .insert(sentry_id, amount_earned);
        } else {
            log!("Transfer to sentry succeeded".to_string());
        }

        let compounder = self.get_strat(&seed_id).get_stable_compounder();
        let farm_info = compounder.get_farm_info(&farm_id);

        // if farm is ended, there is no more actions to do
        if farm_info.state == AutoCompounderState::Ended {
            let compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();
            let farm_info = compounder.get_mut_farm_info(&farm_id);
            farm_info.state = AutoCompounderState::Cleared;

            log!("There farm {} ended. Strategy is now Cleared.", farm_id_str);
            return PromiseOrValue::Value(0u64);
        }

        let mut amounts_to_add: Vec<U128> = vec![];

        for balance in farm_info.available_balance {
            amounts_to_add.push(U128(balance))
        }

        PromiseOrValue::Promise(
            ext_ref_exchange::add_stable_liquidity(
                compounder.pool_id,
                amounts_to_add,
                U128(0u128),
                compounder.exchange_contract_id,
                970000000000000000000, // TODO: create const to do a meaningful name to this value
                Gas(30_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_post_add_stable_liquidity(
                    farm_id_str,
                    env::current_account_id(),
                    0,
                    Gas(150_000_000_000_000),
                ),
            ),
        )
    }

    #[private]
    pub fn stable_callback_post_add_stable_liquidity(
        &mut self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> Promise {
        assert!(
            shares_result.is_ok(),
            "ERR: failed to add liquidity to stable pool"
        );

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        let total_seed = self.seed_total_amount(&seed_id);

        // update owned shares for given seed
        let shares_received = shares_result.unwrap().0;

        log!("shares received {}. total {}", shares_received, total_seed);

        let data = self.data_mut();

        data.seed_id_amount
            .insert(&seed_id, &(total_seed + shares_received));

        let compounder_mut = self.get_strat_mut(&seed_id).get_stable_compounder_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(&farm_id);

        // ensure that in the next run we won't have a balance unless previous steps succeeds
        farm_info_mut.available_balance[farm_info_mut.token_position as usize] = 0u128;

        ext_ref_exchange::get_pool_shares(
            compounder_mut.pool_id,
            env::current_account_id(),
            compounder_mut.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        // Update user balance and stake
        .then(
            callback_stable_ref_finance::stable_callback_post_get_pool_shares(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(120_000_000_000_000),
            ),
        )
    }

    /// Receives shares from auto-compound and stake it
    /// Change the user_balance and the auto_compounder balance of lps/shares
    #[private]
    pub fn stable_callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128> {
        assert!(
            total_shares_result.is_ok(),
            "ERR: failed to get shares from exchange"
        );

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);
        let compounder_mut = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

        let exchange_contract_id: AccountId = compounder_mut.exchange_contract_id.clone();
        let farm_contract_id: AccountId = compounder_mut.farm_contract_id.clone();

        compounder_mut.harvest_timestamp = env::block_timestamp_ms();

        let farm_info_mut = compounder_mut.get_mut_farm_info(&farm_id);

        farm_info_mut.next_cycle();

        let accumulated_shares = total_shares_result.unwrap().0;

        // Prevents failing on stake if below minimum deposit
        let min_deposit = compounder_mut.seed_min_deposit;
        log!(
            "min_deposit {} and shares {}",
            min_deposit.0,
            accumulated_shares
        );
        if accumulated_shares < min_deposit.0 {
            log!(
                "The current number of shares {} is below minimum deposit",
                accumulated_shares
            );
            return PromiseOrValue::Value(0u128);
        }

        PromiseOrValue::Promise(self.call_stake(
            exchange_contract_id,
            farm_contract_id,
            token_id,
            U128(accumulated_shares),
            "\"Free\"".to_string(),
        ))
    }
}
