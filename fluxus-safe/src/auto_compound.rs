use near_sdk::PromiseOrValue;

use crate::*;

const MAX_SLIPPAGE_ALLOWED: u128 = 20;
const MIN_SLIPPAGE_ALLOWED: u128 = 1;

#[near_bindgen]
impl Contract {
    /// Step 1
    /// Function to claim the reward from the farm contract
    #[private]
    pub fn claim_reward(&mut self, token_id: String) -> Promise {
        self.assert_strategy_running(token_id.clone());

        let strat = self.get_strat(&token_id);
        let seed_id: String = strat.get_ref().seed_id.clone();
        let farm_id: String = strat.get_ref().farm_id.clone();

        ext_farm::list_farms_by_seed(
            seed_id,
            self.data().farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(ext_self::callback_list_farms_by_seed(
            token_id,
            farm_id,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    pub fn callback_list_farms_by_seed(
        &mut self,
        #[callback_result] farms_result: Result<Vec<FarmInfo>, PromiseError>,
        token_id: String,
        farm_id: String,
    ) -> PromiseOrValue<String> {
        assert!(farms_result.is_ok(), "ERR_LIST_FARMS_FAILED");

        let farms = farms_result.unwrap();

        for farm in farms.iter() {
            if farm.farm_id == farm_id && farm.farm_status != *"Running" {
                let strat = self
                    .data_mut()
                    .strategies
                    .get_mut(&token_id)
                    .expect(ERR21_TOKEN_NOT_REG);
                let compounder = strat.get_mut();

                compounder.state = AutoCompounderState::Ended;
                return PromiseOrValue::Value(format!("The farm {:#?} ended", farm_id));
            }
        }

        PromiseOrValue::Promise(
            ext_farm::get_unclaimed_reward(
                env::current_account_id(),
                farm_id,
                self.data().farm_contract_id.clone(),
                1,
                Gas(3_000_000_000_000),
            )
            .then(ext_self::callback_post_get_unclaimed_reward(
                token_id,
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
        token_id: String,
    ) -> Promise {
        assert!(reward_amount_result.is_ok(), "ERR_GET_REWARD_FAILED");

        let reward_amount = reward_amount_result.unwrap();
        assert!(reward_amount.0 > 0u128, "ERR_ZERO_REWARDS_EARNED");

        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();

        // store the amount of reward earned
        compounder.last_reward_amount = reward_amount.0;

        ext_farm::claim_reward_by_farm(
            strat.get_ref().farm_id.clone(),
            self.data().farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(ext_self::callback_post_claim_reward(
            token_id,
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_post_claim_reward(
        &mut self,
        #[callback_result] claim_reward_result: Result<(), PromiseError>,
        token_id: String,
    ) {
        assert!(claim_reward_result.is_ok(), "ERR_WITHDRAW_FAILED");

        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();
        compounder.next_cycle();
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    #[private]
    pub fn withdraw_of_reward(&mut self, token_id: String) -> Promise {
        self.assert_strategy_running(token_id.clone());

        let strat = self.get_strat(&token_id);
        let reward_token: AccountId = strat.clone().get().reward_token;

        let compounder = strat.get_ref();

        // contract_id does not exist on sentries
        if !compounder
            .admin_fees
            .sentries
            .contains_key(&env::current_account_id())
        {
            let amount_to_withdraw = compounder.last_reward_amount;
            ext_farm::withdraw_reward(
                reward_token.to_string(),
                U128(amount_to_withdraw),
                "false".to_string(),
                self.data().farm_contract_id.clone(),
                1,
                Gas(180_000_000_000_000),
            )
            .then(ext_self::callback_post_withdraw(
                token_id,
                env::current_account_id(),
                0,
                Gas(80_000_000_000_000),
            ))
        } else {
            // the withdraw succeeded but not the transfer
            ext_reward_token::ft_transfer_call(
                self.exchange_acc(),
                U128(compounder.last_reward_amount + self.data().treasury.current_amount), //Amount after withdraw the rewards
                "".to_string(),
                compounder.reward_token.clone(),
                1,
                Gas(40_000_000_000_000),
            )
            .then(ext_self::callback_post_ft_transfer(
                token_id,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }

        // do B
    }

    #[private]
    pub fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<U128, PromiseError>,
        token_id: String,
    ) -> PromiseOrValue<U128> {
        assert!(withdraw_result.is_ok(), "ERR_WITHDRAW_FROM_FARM_FAILED");

        let exchange_id = self.exchange_acc();

        let treasury_fee_percentage: u128 = self.data().treasury.fee_percentage;

        let data_mut = self.data_mut();

        let strat = data_mut
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();

        let (remaining_amount, protocol_amount, sentry_amount, strat_creator_amount) =
            compounder.compute_fees(compounder.last_reward_amount, treasury_fee_percentage);

        // TODO: store the amount earned by the strat creator

        // store sentry amount under contract account id to be used in the last step
        compounder
            .admin_fees
            .sentries
            .insert(env::current_account_id(), sentry_amount);

        // increase protocol amount to cover the case that the last transfer failed
        data_mut.treasury.current_amount += protocol_amount;

        // remaining amount to reinvest
        compounder.last_reward_amount = remaining_amount;

        // amount sent to ref, both remaining value and treasury
        let amount = remaining_amount + protocol_amount;

        PromiseOrValue::Promise(
            ext_reward_token::ft_transfer_call(
                exchange_id,
                U128(amount), //Amount after withdraw the rewards
                "".to_string(),
                compounder.reward_token.clone(),
                1,
                Gas(40_000_000_000_000),
            )
            .then(ext_self::callback_post_ft_transfer(
                token_id,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_ft_transfer(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        token_id: String,
    ) {
        if transfer_result.is_err() {
            log!("ERR_TRANSFER_TO_EXCHANGE");
            return;
        }

        let data_mut = self.data_mut();
        let strat = data_mut
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();
        compounder.next_cycle();
    }

    /// Step 3
    /// Transfer lp tokens to ref-exchange then swap the amount the contract has in the exchange
    #[private]
    pub fn autocompounds_swap(&mut self, token_id: String) -> Promise {
        // TODO: take string as ref
        self.assert_strategy_running(token_id.clone());

        let treasury_acc: AccountId = self.treasure_acc();
        let treasury_curr_amount: u128 = self.data_mut().treasury.current_amount;

        let strat = self
            .data()
            .strategies
            .get(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_ref();

        let token1 = compounder.token1_address.clone();
        let token2 = compounder.token2_address.clone();
        let reward = compounder.reward_token.clone();

        let mut common_token = 9999;

        if token1 == reward {
            common_token = 1;
        } else if token2 == reward {
            common_token = 2;
        }

        let reward_amount = compounder.last_reward_amount;

        // This works by increasing gradually the slippage allowed
        // It will be used only in the cases where the first swaps succeed but not the second
        if compounder.available_balance[0] > 0 {
            common_token = 1;

            return self
                .get_tokens_return(
                    token_id.clone(),
                    U128(compounder.available_balance[0]),
                    U128(reward_amount),
                    common_token,
                )
                .then(ext_self::swap_to_auto(
                    token_id,
                    U128(compounder.available_balance[0]),
                    U128(reward_amount),
                    common_token,
                    env::current_account_id(),
                    0,
                    Gas(140_000_000_000_000),
                ));
        }

        let amount_in = U128(reward_amount / 2);

        // TODO: transfer value to strategy creator
        ext_exchange::mft_transfer(
            compounder.reward_token.to_string(),
            treasury_acc,
            U128(treasury_curr_amount),
            Some("".to_string()),
            self.data().exchange_contract_id.clone(),
            1,
            Gas(20_000_000_000_000),
        )
        .then(ext_self::callback_post_treasury_mft_transfer(
            token_id.clone(),
            env::current_account_id(),
            0,
            Gas(20_000_000_000_000),
        ));

        self.get_tokens_return(token_id.clone(), amount_in, amount_in, common_token)
            .then(ext_self::swap_to_auto(
                token_id,
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
        token_id: String,
    ) {
        let data_mut = self.data_mut();

        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!("Transfer to treasure failed".to_string());
            return;
        }

        let amount: u128 = data_mut.treasury.current_amount;

        // reset treasury amount earned since tx was successful
        data_mut.treasury.current_amount = 0;

        log!("Transfer {} to treasure succeeded", amount)
    }

    #[private]
    pub fn get_tokens_return(
        &self,
        token_id: String,
        amount_token_1: U128,
        amount_token_2: U128,
        common_token: u64,
    ) -> Promise {
        let strat = self.get_strat(&token_id);
        let compounder = strat.get_ref();

        if common_token == 1 {
            // TODO: can be shortened by call_get_return
            ext_exchange::get_return(
                compounder.pool_id_token2_reward,
                compounder.reward_token.clone(),
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
                compounder.pool_id_token1_reward,
                compounder.reward_token.clone(),
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
                compounder.pool_id_token1_reward,
                compounder.reward_token.clone(),
                amount_token_1,
                compounder.token1_address.clone(),
                self.data().exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .and(ext_exchange::get_return(
                compounder.pool_id_token2_reward,
                compounder.reward_token.clone(),
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
        token_id: String,
        amount_in_1: U128,
        amount_in_2: U128,
        common_token: u64,
    ) -> Promise {
        let (_, contract_id) = self.get_predecessor_and_current_account();

        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        let pool_id_to_swap1 = compounder.pool_id_token1_reward;
        let pool_id_to_swap2 = compounder.pool_id_token2_reward;
        let token_in1 = compounder.reward_token.clone();
        let token_in2 = compounder.reward_token.clone();
        let token_out1 = compounder.token1_address.clone();
        let token_out2 = compounder.token2_address.clone();

        let (mut token1_min_out, mut token2_min_out): (U128, U128) = tokens;

        // apply slippage
        let percent = Percentage::from(compounder.slippage);

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
            compounder.available_balance[0] = amount_in_1.0;
            self.call_swap(
                pool_id_to_swap2,
                token_in2,
                token_out2,
                Some(amount_in_2),
                token2_min_out,
            )
            .then(ext_self::callback_post_swap(
                token_id,
                common_token,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        } else if common_token == 2 {
            // use the entire amount for the common token
            compounder.available_balance[1] = amount_in_2.0;
            self.call_swap(
                pool_id_to_swap1,
                token_in1,
                token_out1,
                Some(amount_in_1),
                token1_min_out,
            )
            .then(ext_self::callback_post_swap(
                token_id,
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
                token_id.clone(),
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
            .then(ext_self::call_swap(
                pool_id_to_swap2,
                token_in2,
                token_out2,
                Some(amount_in_2),
                U128(token2_min_out.0),
                contract_id,
                0,
                Gas(30_000_000_000_000),
            ))
            .then(ext_self::callback_post_swap(
                token_id,
                common_token,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }
    }

    #[private]
    pub fn callback_post_first_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        token_id: String,
    ) {
        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        if swap_result.is_err() {
            log!("ERR_FIRST_SWAP_FAILED");
            if 100u128 - compounder.slippage < MAX_SLIPPAGE_ALLOWED {
                // increment slippage
                compounder.slippage -= 1;
            }
        }

        compounder.available_balance[0] = swap_result.unwrap().0;

        // First swap succeeded, thus decrement the last reward_amount
        let amount_used: u128 = compounder.last_reward_amount / 2;
        compounder.last_reward_amount -= amount_used;
    }

    #[private]
    pub fn callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        token_id: String,
        common_token: u64,
    ) {
        let data_mut = self.data_mut();
        let strat = data_mut
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        if swap_result.is_err() {
            if 100u128 - compounder.slippage < MAX_SLIPPAGE_ALLOWED {
                // increment slippage
                compounder.slippage -= 1;
            }
            // TODO: cannot panic because this would invalidate the slippage update
            log!("ERR_SECOND_SWAP_FAILED");
            return;
        }

        // no more rewards to spend
        compounder.last_reward_amount = 0;
        // update balance to add liquidity
        if common_token == 1 {
            // update missing balance
            compounder.available_balance[1] = swap_result.unwrap().0;
        } else if common_token == 2 {
            // update missing balance
            compounder.available_balance[0] = swap_result.unwrap().0;
        } else {
            compounder.available_balance[1] = swap_result.unwrap().0;
        }
        // reset slippage
        compounder.slippage = 100 - MIN_SLIPPAGE_ALLOWED;
        // after both swaps succeeded, it's ready to stake
        compounder.next_cycle();
    }

    /// Step 4
    /// Get amount of tokens available then stake it
    #[private]
    pub fn autocompounds_liquidity_and_stake(&mut self, token_id: String) -> Promise {
        self.assert_strategy_running(token_id.clone());

        let strat = self
            .data()
            .strategies
            .get(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_ref();

        let pool_id: u64 = compounder.pool_id;

        let token1_amount = compounder.available_balance[0];
        let token2_amount = compounder.available_balance[1];

        // send reward to contract caller
        self.send_reward_to_sentry(token_id.clone(), env::predecessor_account_id())
            // Add liquidity
            .then(ext_exchange::add_liquidity(
                pool_id,
                vec![U128(token1_amount), U128(token2_amount)],
                None,
                self.data().exchange_contract_id.clone(),
                970000000000000000000, // TODO: create const to do a meaningful name to this value
                Gas(30_000_000_000_000),
            ))
            .then(ext_self::callback_stake(
                token_id.clone(),
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
                token_id,
                env::current_account_id(),
                0,
                Gas(120_000_000_000_000),
            ))
    }

    #[private]
    pub fn callback_stake(
        &mut self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
    ) -> U128 {
        assert!(shares_result.is_ok(), "ERR");

        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        // ensure that in the next run we won't have a balance unless previous steps succeeds
        compounder.available_balance[0] = 0u128;
        compounder.available_balance[1] = 0u128;

        let shares_received = shares_result.unwrap().0;

        let mut total_shares: u128 = 0;

        for (_, balance) in compounder.user_shares.iter() {
            total_shares += balance.total;
        }

        compounder.balance_update(total_shares, shares_received);

        U128(shares_received)
    }

    /// Receives shares from auto-compound and stake it
    /// Change the user_balance and the auto_compounder balance of lps/shares
    #[private]
    pub fn callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        token_id: String,
    ) {
        assert!(total_shares_result.is_ok(), "ERR");

        let strat = self.get_strat_mut(&token_id);

        let compounder = strat.get_mut();

        compounder.next_cycle();

        let accumulated_shares = total_shares_result.unwrap().0;

        // Prevents failing on stake if below minimum deposit
        if accumulated_shares < compounder.seed_min_deposit.into() {
            log!(
                "The current number of shares {} is below minimum deposit",
                accumulated_shares
            );
            return;
        }

        self.call_stake(
            self.data().farm_contract_id.clone(),
            token_id,
            U128(accumulated_shares),
            "".to_string(),
        );
    }
}
