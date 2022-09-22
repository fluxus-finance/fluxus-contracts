use near_sdk::PromiseOrValue;

use crate::*;

const MIN_SLIPPAGE_ALLOWED: u128 = 1;

#[near_bindgen]
impl Contract {
    /// Check if farm still have rewards to distribute (status == Running)
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn callback_list_farms_by_seed(
        &mut self,
        #[callback_result] farms_result: Result<Vec<FarmInfoBoost>, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<String> {
        assert!(farms_result.is_ok(), "{}", ERR01_LIST_FARMS_FAILED);

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        let farms = farms_result.unwrap();

        // Try to unclaim before change to Ended
        for farm in farms.iter() {
            if farm.farm_id == farm_id && farm.status != *"Running" {
                let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();

                for strat_farm in compounder.farms.iter_mut() {
                    if strat_farm.id == farm_id {
                        strat_farm.state = AutoCompounderState::Ended;
                    }
                }
            }
        }

        let compounder = self.get_strat(&seed_id).get_compounder();

        PromiseOrValue::Promise(
            ext_ref_farming::get_unclaimed_rewards(
                env::current_account_id(),
                seed_id,
                compounder.farm_contract_id,
                0,
                Gas(3_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_get_unclaimed_reward(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(70_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_get_unclaimed_reward(
        &mut self,
        #[callback_result] reward_amount_result: Result<HashMap<String, U128>, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128> {
        assert!(reward_amount_result.is_ok(), "{}", ERR02_GET_REWARD_FAILED);

        let mut rewards_map = reward_amount_result.unwrap();

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();

        let farm_info = compounder.get_mut_farm_info(farm_id);

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
                panic!("{}", ERR06_ZERO_REWARDS_EARNED)
            }
        }

        PromiseOrValue::Promise(
            ext_ref_farming::claim_reward_by_seed(
                seed_id,
                compounder.farm_contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_claim_reward(
                farm_id_str,
                reward_amount,
                rewards_map,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_claim_reward(
        &mut self,
        #[callback_result] claim_reward_result: Result<(), PromiseError>,
        farm_id_str: String,
        reward_amount: U128,
        rewards_map: HashMap<String, U128>,
    ) -> u128 {
        assert!(
            claim_reward_result.is_ok(),
            "{}",
            ERR04_WITHDRAW_FROM_FARM_FAILED
        );

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        // update strategies with the same seed
        let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();
        compounder.update_strats_by_seed(rewards_map);

        // store the amount of reward earned
        let farm_info = compounder.get_mut_farm_info(farm_id);
        farm_info.last_reward_amount += reward_amount.0;

        farm_info.next_cycle();

        reward_amount.0
    }

    #[private]
    pub fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<bool, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<U128> {
        assert!(
            withdraw_result.is_ok(),
            "{}",
            ERR04_WITHDRAW_FROM_FARM_FAILED
        );

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let data_mut = self.data_mut();

        let compounder = data_mut
            .strategies
            .get_mut(&seed_id)
            .expect(ERR42_TOKEN_NOT_REG)
            .get_compounder_mut();

        let last_reward_amount = compounder
            .get_mut_farm_info(farm_id.clone())
            .last_reward_amount;

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
        compounder
            .get_mut_farm_info(farm_id.clone())
            .last_reward_amount = remaining_amount;

        // amount sent to ref, both remaining value and treasury
        let amount = remaining_amount + protocol_amount;

        PromiseOrValue::Promise(
            ext_reward_token::ft_transfer_call(
                compounder.exchange_contract_id.clone(),
                U128(amount), //Amount after withdraw the rewards
                "".to_string(),
                compounder.get_mut_farm_info(farm_id).reward_token.clone(),
                1,
                Gas(40_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_ft_transfer(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_ft_transfer(
        &mut self,
        #[callback_result] exchange_transfer_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) {
        if exchange_transfer_result.is_err() {
            log!(ERR07_TRANSFER_TO_EXCHANGE);
            return;
        }

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();
        let farm_info_mut = compounder.get_mut_farm_info(farm_id);

        farm_info_mut.next_cycle();
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn callback_post_treasury_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
    ) {
        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!(ERR08_TRANSFER_TO_TREASURE);
            return;
        }

        let data_mut = self.data_mut();
        let amount: u128 = data_mut.treasury.current_amount;

        // reset treasury amount earned since tx was successful
        data_mut.treasury.current_amount = 0;

        log!("Transfer {} to treasure succeeded", amount)
    }

    #[private]
    pub fn callback_post_creator_ft_transfer(
        &mut self,
        #[callback_result] strat_creator_transfer_result: Result<(), PromiseError>,
        seed_id: String,
    ) {
        if strat_creator_transfer_result.is_err() {
            log!(ERR09_TRANSFER_TO_CREATOR);
            return;
        }

        let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();

        compounder.admin_fees.strat_creator.current_amount = 0;

        log!("Transfer fees to the creator of the strategy succeeded");
    }

    #[private]
    pub fn get_tokens_return(
        &self,
        farm_id_str: String,
        amount_token_1: U128,
        amount_token_2: U128,
        common_token: u64,
    ) -> Promise {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder = self.get_strat(&seed_id).get_compounder();
        let farm_info = compounder.get_farm_info(&farm_id);

        if common_token == 1 {
            // TODO: can be shortened by call_get_return
            ext_ref_exchange::get_return(
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                amount_token_2,
                compounder.token2_address.clone(),
                compounder.exchange_contract_id,
                0,
                Gas(10_000_000_000_000),
            )
            .then(callback_ref_finance::callback_get_token_return(
                common_token,
                amount_token_1,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else if common_token == 2 {
            ext_ref_exchange::get_return(
                farm_info.pool_id_token1_reward,
                farm_info.reward_token,
                amount_token_1,
                compounder.token1_address.clone(),
                compounder.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .then(callback_ref_finance::callback_get_token_return(
                common_token,
                amount_token_2,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else {
            ext_ref_exchange::get_return(
                farm_info.pool_id_token1_reward,
                farm_info.reward_token.clone(),
                amount_token_1,
                compounder.token1_address.clone(),
                compounder.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .and(ext_ref_exchange::get_return(
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                amount_token_2,
                compounder.token2_address.clone(),
                compounder.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            ))
            .then(callback_ref_finance::callback_get_tokens_return(
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        }
    }

    #[private]
    pub fn callback_get_token_return(
        &self,
        #[callback_result] token_out: Result<U128, PromiseError>,
        common_token: u64,
        amount_token: U128,
    ) -> (U128, U128) {
        assert!(
            token_out.is_ok(),
            "{}",
            ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN
        );

        let amount: U128 = token_out.unwrap();

        assert!(amount.0 > 0u128, "{}", ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN);

        if common_token == 1 {
            (amount_token, amount)
        } else {
            (amount, amount_token)
        }
    }

    #[private]
    pub fn callback_get_tokens_return(
        &self,
        #[callback_result] token1_out: Result<U128, PromiseError>,
        #[callback_result] token2_out: Result<U128, PromiseError>,
    ) -> (U128, U128) {
        assert!(
            token1_out.is_ok(),
            "{}",
            ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN
        );
        assert!(
            token2_out.is_ok(),
            "{}",
            ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN
        );

        let amount_token1: U128 = token1_out.unwrap();
        let amount_token2: U128 = token2_out.unwrap();

        assert!(
            amount_token1.0 > 0u128,
            "{}",
            ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN
        );
        assert!(
            amount_token2.0 > 0u128,
            "{}",
            ERR05_COULD_NOT_GET_RETURN_FOR_TOKEN
        );

        (amount_token1, amount_token2)
    }

    /// Swap the auto-compound rewards
    #[private]
    pub fn swap_to_auto(
        &mut self,
        #[callback_unwrap] tokens: (U128, U128),
        farm_id_str: String,
        amount_in_1: U128,
        amount_in_2: U128,
        common_token: u64,
    ) -> Promise {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();
        let token_out1 = compounder_mut.token1_address.clone();
        let token_out2 = compounder_mut.token2_address.clone();

        let exchange_contract_id: AccountId = compounder_mut.exchange_contract_id.clone();

        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        let pool_id_to_swap1 = farm_info_mut.pool_id_token1_reward;
        let pool_id_to_swap2 = farm_info_mut.pool_id_token2_reward;
        let token_in = farm_info_mut.reward_token.clone();

        let (mut token1_min_out, mut token2_min_out): (U128, U128) = tokens;

        // apply slippage
        let percent = Percentage::from(farm_info_mut.slippage);

        token1_min_out = U128(percent.apply_to(token1_min_out.0));
        token2_min_out = U128(percent.apply_to(token2_min_out.0));

        log!(
            "min amount out: {} for {} and {} for {}",
            token1_min_out.0,
            token_out1,
            token2_min_out.0,
            token_out2
        );

        if common_token == 1 {
            // use the entire amount for the common token
            farm_info_mut.available_balance[0] = amount_in_1.0;

            self.call_swap(
                exchange_contract_id,
                pool_id_to_swap2,
                token_in,
                token_out2,
                Some(amount_in_2),
                token2_min_out,
            )
            .then(callback_ref_finance::callback_post_swap(
                farm_id_str,
                common_token,
                env::current_account_id(),
                0,
                Gas(80_000_000_000_000),
            ))
        } else if common_token == 2 {
            // use the entire amount for the common token
            farm_info_mut.available_balance[1] = amount_in_2.0;
            self.call_swap(
                exchange_contract_id,
                pool_id_to_swap1,
                token_in,
                token_out1,
                Some(amount_in_1),
                token1_min_out,
            )
            .then(callback_ref_finance::callback_post_swap(
                farm_id_str,
                common_token,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        } else {
            self.call_swap(
                exchange_contract_id,
                pool_id_to_swap1,
                token_in,
                token_out1,
                Some(amount_in_1),
                token1_min_out,
            )
            .then(callback_ref_finance::callback_post_first_swap(
                farm_id_str,
                common_token,
                amount_in_2,
                token2_min_out,
                env::current_account_id(),
                0,
                Gas(80_000_000_000_000),
            ))
        }
    }

    #[private]
    pub fn callback_post_first_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
        common_token: u64,
        amount_in: U128,
        token_min_out: U128,
    ) -> PromiseOrValue<u64> {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();

        let exchange_contract_id: AccountId = compounder_mut.exchange_contract_id.clone();

        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            farm_info_mut.increase_slippage();
            log!(ERR10_SWAP_TOKEN);

            return PromiseOrValue::Value(0u64);
        }

        farm_info_mut.available_balance[0] = swap_result.unwrap().0;

        // First swap succeeded, thus decrement the last reward_amount
        let amount_used: u128 = farm_info_mut.last_reward_amount / 2;
        farm_info_mut.last_reward_amount -= amount_used;

        let pool_id_to_swap2 = farm_info_mut.pool_id_token2_reward;
        let token_in2 = farm_info_mut.reward_token.clone();
        let token_out2 = compounder_mut.token2_address.clone();

        PromiseOrValue::Promise(
            callback_ref_finance::call_swap(
                exchange_contract_id,
                pool_id_to_swap2,
                token_in2,
                token_out2,
                Some(amount_in),
                token_min_out,
                env::current_account_id(),
                0,
                Gas(30_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_swap(
                farm_id_str,
                common_token,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
        common_token: u64,
    ) {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);
        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            farm_info_mut.increase_slippage();
            log!(ERR10_SWAP_TOKEN);
            return;
        }

        // no more rewards to spend
        farm_info_mut.last_reward_amount = 0;

        // update balance to add liquidity
        if common_token == 1 {
            // update missing balance
            farm_info_mut.available_balance[1] = swap_result.unwrap().0;
        } else if common_token == 2 {
            // update missing balance
            farm_info_mut.available_balance[0] = swap_result.unwrap().0;
        } else {
            farm_info_mut.available_balance[1] = swap_result.unwrap().0;
        }

        // reset slippage
        farm_info_mut.slippage = 100 - MIN_SLIPPAGE_ALLOWED;

        // after both swaps succeeded, it's ready to stake
        farm_info_mut.next_cycle();
    }

    #[private]
    pub fn callback_post_sentry(
        &mut self,
        #[callback_result] result: Result<Option<StorageBalance>, PromiseError>,
        farm_id_str: String,
        sentry_acc_id: AccountId,
        reward_token: AccountId,
    ) -> Promise {
        // TODO: propagate error
        match result {
            Ok(balance_op) => match balance_op {
                Some(balance) => assert!(balance.total.0 > 1),
                _ => {
                    let msg = format!(
                        "{}{:#?}",
                        ERR11_NOT_ENOUGH_BALANCE,
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
            Err(_) => env::panic_str(ERR12_CALLER_NOT_REGISTER),
        }

        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.clone());
        let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();

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
        .then(callback_ref_finance::callback_post_sentry_mft_transfer(
            farm_id_str,
            sentry_acc_id,
            amount,
            env::current_account_id(),
            0,
            Gas(200_000_000_000_000),
        ))
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn callback_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        farm_id_str: String,
        sentry_id: AccountId,
        amount_earned: u128,
    ) -> PromiseOrValue<u64> {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!(ERR13_TRANSFER_TO_SENTRY);

            let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();

            // store amount earned by sentry to be redeemed
            compounder
                .admin_fees
                .sentries
                .insert(sentry_id, amount_earned);
        } else {
            log!("Transfer to sentry succeeded".to_string());
        }

        let compounder = self.get_strat(&seed_id).get_compounder();
        let farm_info = compounder.get_farm_info(&farm_id);

        // if farm is ended, there is no more actions to do
        if farm_info.state == AutoCompounderState::Ended {
            let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();
            let farm_info = compounder.get_mut_farm_info(farm_id);
            farm_info.state = AutoCompounderState::Cleared;

            log!("There farm {} ended. Strategy is now Cleared.", farm_id_str);
            return PromiseOrValue::Value(0u64);
        }

        let pool_id: u64 = compounder.pool_id;

        let token1_amount = farm_info.available_balance[0];
        let token2_amount = farm_info.available_balance[1];

        PromiseOrValue::Promise(
            ext_ref_exchange::add_liquidity(
                pool_id,
                vec![U128(token1_amount), U128(token2_amount)],
                None,
                compounder.exchange_contract_id.clone(),
                970000000000000000000, // TODO: create const to do a meaningful name to this value
                Gas(30_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_add_liquidity(
                farm_id_str.clone(),
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
            // Get the shares
            .then(ext_ref_exchange::get_pool_shares(
                pool_id,
                env::current_account_id(),
                compounder.exchange_contract_id,
                0,
                Gas(10_000_000_000_000),
            ))
            // Update user balance and stake
            .then(callback_ref_finance::callback_post_get_pool_shares(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(120_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_add_liquidity(
        &mut self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> U128 {
        assert!(shares_result.is_ok(), "{}", ERR14_ADD_LIQUIDITY);

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        // ensure that in the next run we won't have a balance unless previous steps succeeds
        farm_info_mut.available_balance[0] = 0u128;
        farm_info_mut.available_balance[1] = 0u128;

        // update owned shares for given seed
        let shares_received = shares_result.unwrap().0;

        let total_seed = self.seed_total_amount(&seed_id);

        log!(
            "Received {} shares. The current number of shares is {}",
            shares_received,
            total_seed
        );

        let data = self.data_mut();

        data.seed_id_amount
            .insert(&seed_id, &(total_seed + shares_received));

        U128(shares_received)
    }

    /// Receives shares from auto-compound and stake it
    /// Change the user_balance and the auto_compounder balance of lps/shares
    #[private]
    pub fn callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128> {
        assert!(total_shares_result.is_ok(), "{}", ERR17_GET_POOL_SHARES);

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str);
        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();

        let exchange_contract_id: AccountId = compounder_mut.exchange_contract_id.clone();
        let farm_contract_id: AccountId = compounder_mut.farm_contract_id.clone();

        compounder_mut.harvest_timestamp = env::block_timestamp_ms();

        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

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
