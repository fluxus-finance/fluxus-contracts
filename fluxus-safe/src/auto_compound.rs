use near_sdk::PromiseOrValue;

use crate::*;

const MIN_SLIPPAGE_ALLOWED: u128 = 1;

#[near_bindgen]
impl Contract {
    /// Step 1
    /// Function to claim the reward from the farm contract
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn claim_reward(&mut self, farm_id_str: String) -> Promise {
        self.assert_strategy_running(&farm_id_str);

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        log!("token {} farm {}", token_id, farm_id);

        ext_farm::list_farms_by_seed(
            seed_id,
            self.data().farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(ext_self::callback_list_farms_by_seed(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    /// Check if farm still have rewards to distribute (status == Running)
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn callback_list_farms_by_seed(
        &mut self,
        #[callback_result] farms_result: Result<Vec<FarmInfo>, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<String> {
        assert!(farms_result.is_ok(), "ERR_LIST_FARMS_FAILED");

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let farms = farms_result.unwrap();

        for farm in farms.iter() {
            if farm.farm_id == farm_id && farm.farm_status != *"Running" {
                let compounder = self.get_strat_mut(&token_id.to_string()).get_mut();

                for strat_farm in compounder.farms.iter_mut() {
                    if strat_farm.id == farm_id {
                        strat_farm.state = AutoCompounderState::Ended;
                        return PromiseOrValue::Value(format!("The farm {:#?} ended", farm_id));
                    }
                }
            }
        }

        PromiseOrValue::Promise(
            ext_farm::get_unclaimed_reward(
                env::current_account_id(),
                farm_id_str.clone(),
                self.data().farm_contract_id.clone(),
                1,
                Gas(3_000_000_000_000),
            )
            .then(ext_self::callback_post_get_unclaimed_reward(
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
        #[callback_result] reward_amount_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> Promise {
        assert!(reward_amount_result.is_ok(), "ERR_GET_REWARD_FAILED");

        let reward_amount = reward_amount_result.unwrap();
        assert!(reward_amount.0 > 0u128, "ERR_ZERO_REWARDS_EARNED");

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let strat = self.get_strat_mut(&token_id.to_string());

        let compounder = strat.get_mut();

        let farm_info = compounder.get_mut_farm_info(farm_id);

        // store the amount of reward earned
        farm_info.last_reward_amount = reward_amount.0;

        ext_farm::claim_reward_by_farm(
            farm_id_str.clone(),
            self.data().farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(ext_self::callback_post_claim_reward(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_post_claim_reward(
        &mut self,
        #[callback_result] claim_reward_result: Result<(), PromiseError>,
        farm_id_str: String,
    ) {
        assert!(claim_reward_result.is_ok(), "ERR_WITHDRAW_FAILED");

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder = self.get_strat_mut(&token_id.to_string()).get_mut();
        let farm_info = compounder.get_mut_farm_info(farm_id);
        farm_info.next_cycle();
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn withdraw_of_reward(&mut self, farm_id_str: String) -> Promise {
        self.assert_strategy_running(&farm_id_str);

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let strat = self.get_strat(token_id);
        let compounder = strat.get();
        let farm_info = compounder.get_farm_info(&farm_id);

        // contract_id does not exist on sentries
        if !compounder
            .admin_fees
            .sentries
            .contains_key(&env::current_account_id())
        {
            let amount_to_withdraw = farm_info.last_reward_amount;
            ext_farm::withdraw_reward(
                farm_info.reward_token.to_string(),
                U128(amount_to_withdraw),
                "false".to_string(),
                self.data().farm_contract_id.clone(),
                1,
                Gas(180_000_000_000_000),
            )
            .then(ext_self::callback_post_withdraw(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(80_000_000_000_000),
            ))
        } else {
            // the withdraw succeeded but not the transfer
            ext_reward_token::ft_transfer_call(
                self.exchange_acc(),
                U128(farm_info.last_reward_amount + self.data().treasury.current_amount), //Amount after withdraw the rewards
                "".to_string(),
                farm_info.reward_token,
                1,
                Gas(40_000_000_000_000),
            )
            .then(ext_self::callback_post_ft_transfer(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }
    }

    #[private]
    pub fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<U128> {
        assert!(withdraw_result.is_ok(), "ERR_WITHDRAW_FROM_FARM_FAILED");

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let exchange_id = self.exchange_acc();

        let data_mut = self.data_mut();

        let strat = data_mut
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();

        // let farm_info_mut = compounder.get_mut_farm_info(farm_id);

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
                exchange_id,
                U128(amount), //Amount after withdraw the rewards
                "".to_string(),
                compounder.get_mut_farm_info(farm_id).reward_token.clone(),
                1,
                Gas(40_000_000_000_000),
            )
            .then(ext_self::callback_post_ft_transfer(
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
            log!("ERR_TRANSFER_TO_EXCHANGE");
            return;
        }

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let data_mut = self.data_mut();
        let strat = data_mut
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();
        let farm_info_mut = compounder.get_mut_farm_info(farm_id);
        farm_info_mut.next_cycle();
    }

    /// Step 3
    /// Transfer reward token to ref-exchange then swap the amount the contract has in the exchange
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn autocompounds_swap(&mut self, farm_id_str: String) -> Promise {
        // TODO: take string as ref
        self.assert_strategy_running(&farm_id_str);

        let treasury_acc: AccountId = self.treasure_acc();
        let treasury_curr_amount: u128 = self.data_mut().treasury.current_amount;

        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.clone());
        let strat = self.get_strat(token_id.clone());
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);

        let token1 = compounder.token1_address.clone();
        let token2 = compounder.token2_address.clone();
        let reward = farm_info.reward_token.clone();

        let mut common_token = 0;

        if token1 == reward {
            common_token = 1;
        } else if token2 == reward {
            common_token = 2;
        }

        let reward_amount = farm_info.last_reward_amount;

        // This works by increasing gradually the slippage allowed
        // It will be used only in the cases where the first swaps succeed but not the second
        if farm_info.available_balance[0] > 0 {
            common_token = 1;

            return self
                .get_tokens_return(
                    farm_id_str.clone(),
                    U128(farm_info.available_balance[0]),
                    U128(reward_amount),
                    common_token,
                )
                .then(ext_self::swap_to_auto(
                    farm_id_str,
                    U128(farm_info.available_balance[0]),
                    U128(reward_amount),
                    common_token,
                    env::current_account_id(),
                    0,
                    Gas(140_000_000_000_000),
                ));
        }

        let amount_in = U128(reward_amount / 2);

        if treasury_curr_amount > 0 {
            ext_exchange::mft_transfer(
                farm_info.reward_token.to_string(),
                treasury_acc,
                U128(treasury_curr_amount),
                Some("".to_string()),
                self.data().exchange_contract_id.clone(),
                1,
                Gas(20_000_000_000_000),
            )
            .then(ext_self::callback_post_treasury_mft_transfer(
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ));
        }

        let strat_creator_curr_amount = compounder.admin_fees.strat_creator.current_amount;
        if strat_creator_curr_amount > 0 {
            ext_reward_token::ft_transfer(
                compounder.admin_fees.strat_creator.account_id.clone(),
                U128(strat_creator_curr_amount),
                Some("".to_string()),
                farm_info.reward_token,
                1,
                Gas(20_000_000_000_000),
            )
            .then(ext_self::callback_post_creator_ft_transfer(
                token_id,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ));
        }

        self.get_tokens_return(farm_id_str.clone(), amount_in, amount_in, common_token)
            .then(ext_self::swap_to_auto(
                farm_id_str,
                amount_in,
                amount_in,
                common_token,
                env::current_account_id(),
                0,
                Gas(140_000_000_000_000),
            ))
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn callback_post_treasury_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
    ) {
        let data_mut = self.data_mut();

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!("Transfer to treasure failed");
            return;
        }

        let amount: u128 = data_mut.treasury.current_amount;

        // reset treasury amount earned since tx was successful
        data_mut.treasury.current_amount = 0;

        log!("Transfer {} to treasure succeeded", amount)
    }

    #[private]
    pub fn callback_post_creator_ft_transfer(
        &mut self,
        #[callback_result] strat_creator_transfer_result: Result<(), PromiseError>,
        token_id: String,
    ) {
        if strat_creator_transfer_result.is_err() {
            log!("ERR_TRANSFER_TO_CREATOR");
            return;
        }

        let strat = self.get_strat_mut(&token_id);
        let compounder = strat.get_mut();

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
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str);
        let strat = self.get_strat(token_id);
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);

        if common_token == 1 {
            // TODO: can be shortened by call_get_return
            ext_exchange::get_return(
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                amount_token_2,
                compounder.token2_address.clone(),
                self.data().exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .then(ext_self::callback_get_token_return(
                common_token,
                amount_token_1,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else if common_token == 2 {
            ext_exchange::get_return(
                farm_info.pool_id_token1_reward,
                farm_info.reward_token,
                amount_token_1,
                compounder.token1_address.clone(),
                self.data().exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .then(ext_self::callback_get_token_return(
                common_token,
                amount_token_2,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else {
            ext_exchange::get_return(
                farm_info.pool_id_token1_reward,
                farm_info.reward_token.clone(),
                amount_token_1,
                compounder.token1_address.clone(),
                self.data().exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .and(ext_exchange::get_return(
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                amount_token_2,
                compounder.token2_address.clone(),
                self.data().exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            ))
            .then(ext_self::callback_get_tokens_return(
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
        assert!(token_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_RETURN");

        let amount: U128 = token_out.unwrap();

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
        assert!(token1_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_1_RETURN");
        assert!(token2_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_2_RETURN");

        let amount_token1: U128 = token1_out.unwrap();
        let amount_token2: U128 = token2_out.unwrap();

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
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&token_id).get_mut();
        let token_out1 = compounder_mut.token1_address.clone();
        let token_out2 = compounder_mut.token2_address.clone();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        let pool_id_to_swap1 = farm_info_mut.pool_id_token1_reward;
        let pool_id_to_swap2 = farm_info_mut.pool_id_token2_reward;
        let token_in1 = farm_info_mut.reward_token.clone();
        let token_in2 = farm_info_mut.reward_token.clone();

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
                pool_id_to_swap2,
                token_in2,
                token_out2,
                Some(amount_in_2),
                token2_min_out,
            )
            .then(ext_self::callback_post_swap(
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
                pool_id_to_swap1,
                token_in1,
                token_out1,
                Some(amount_in_1),
                token1_min_out,
            )
            .then(ext_self::callback_post_swap(
                farm_id_str,
                common_token,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        } else {
            self.call_swap(
                pool_id_to_swap1,
                token_in1,
                token_out1,
                Some(amount_in_1),
                token1_min_out,
            )
            .then(ext_self::callback_post_first_swap(
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
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&token_id).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            compounder_mut.increase_slippage();
            log!("ERR_FIRST_SWAP_FAILED");

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
            ext_self::call_swap(
                pool_id_to_swap2,
                token_in2,
                token_out2,
                Some(amount_in),
                token_min_out,
                env::current_account_id(),
                0,
                Gas(30_000_000_000_000),
            )
            .then(ext_self::callback_post_swap(
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
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&token_id).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        // Do not panic if err == true, otherwise the slippage update will not be applied
        if swap_result.is_err() {
            compounder_mut.increase_slippage();
            log!("ERR_SECOND_SWAP_FAILED");
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

    /// Step 4
    /// Get amount of tokens available then stake it
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    #[private]
    pub fn autocompounds_liquidity_and_stake(&mut self, farm_id_str: String) -> Promise {
        self.assert_strategy_running(&farm_id_str);

        // send reward to contract caller
        self.send_reward_to_sentry(farm_id_str, env::predecessor_account_id())
    }

    #[private]
    pub fn send_reward_to_sentry(&self, farm_id_str: String, sentry_acc_id: AccountId) -> Promise {
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let strat = self.get_strat(token_id);
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);

        ext_reward_token::storage_balance_of(
            sentry_acc_id.clone(),
            farm_info.reward_token.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(ext_self::callback_post_sentry(
            farm_id_str,
            sentry_acc_id,
            farm_info.reward_token,
            env::current_account_id(),
            0,
            Gas(240_000_000_000_000),
        ))
    }

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
                    // let msg = ("ERR: callback post Sentry no balance {:#?} ",balance_op);
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

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder = self.get_strat_mut(&token_id.to_string()).get_mut();

        // reset default sentry address and get last earned amount
        let amount = compounder
            .admin_fees
            .sentries
            .remove(&env::current_account_id())
            .unwrap();

        log!("amount {}", amount);

        ext_reward_token::ft_transfer(
            sentry_acc_id.clone(),
            U128(amount),
            Some("".to_string()),
            reward_token,
            1,
            Gas(20_000_000_000_000),
        )
        .then(ext_self::callback_post_sentry_mft_transfer(
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
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!("Transfer to sentry failed".to_string());

            let compounder = self.get_strat_mut(&token_id.to_string()).get_mut();

            // store amount earned by sentry to be redeemed
            compounder
                .admin_fees
                .sentries
                .insert(sentry_id, amount_earned);
        }

        let strat = self.get_strat(token_id);
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);

        let pool_id: u64 = compounder.pool_id;

        let token1_amount = farm_info.available_balance[0];
        let token2_amount = farm_info.available_balance[1];

        PromiseOrValue::Promise(
            ext_exchange::add_liquidity(
                pool_id,
                vec![U128(token1_amount), U128(token2_amount)],
                None,
                self.data().exchange_contract_id.clone(),
                970000000000000000000, // TODO: create const to do a meaningful name to this value
                Gas(30_000_000_000_000),
            )
            .then(ext_self::callback_post_add_liquidity(
                farm_id_str.clone(),
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
            // Get the shares
            .then(ext_exchange::get_pool_shares(
                pool_id,
                env::current_account_id(),
                self.data().exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            ))
            // Update user balance and stake
            .then(ext_self::callback_post_get_pool_shares(
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
        assert!(shares_result.is_ok(), "ERR");

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder_mut = self.get_strat_mut(&token_id).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        // ensure that in the next run we won't have a balance unless previous steps succeeds
        farm_info_mut.available_balance[0] = 0u128;
        farm_info_mut.available_balance[1] = 0u128;

        // update owned shares for given seed
        let shares_received = shares_result.unwrap().0;

        let total_seed = self.seed_total_amount(token_id);

        log!("shares received {}. total {}", shares_received, total_seed);

        let data = self.data_mut();

        data.seed_id_amount
            .insert(seed_id, total_seed + shares_received);

        U128(shares_received)
    }

    /// Receives shares from auto-compound and stake it
    /// Change the user_balance and the auto_compounder balance of lps/shares
    #[private]
    pub fn callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) {
        assert!(total_shares_result.is_ok(), "ERR");

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&token_id.to_string()).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        farm_info_mut.next_cycle();

        let accumulated_shares = total_shares_result.unwrap().0;

        // Prevents failing on stake if below minimum deposit
        if accumulated_shares < compounder_mut.seed_min_deposit.into() {
            log!(
                "The current number of shares {} is below minimum deposit",
                accumulated_shares
            );
            return;
        }

        self.call_stake(
            self.data().farm_contract_id.clone(),
            token_id.to_string(),
            U128(accumulated_shares),
            "".to_string(),
        );
    }
}
