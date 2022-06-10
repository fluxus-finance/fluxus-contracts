use near_sdk::PromiseOrValue;

use crate::*;

#[near_bindgen]
impl Contract {
    /// Step 1
    /// Function to claim the reward from the farm contract
    pub fn claim_reward(&mut self, token_id: String) -> Promise {
        self.assert_strategy_running(token_id.clone());
        self.is_allowed_account();

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let seed_id: String = strat.get_ref().seed_id.clone();
        let farm_id: String = strat.get_ref().farm_id.clone();

        ext_farm::list_farms_by_seed(
            seed_id.clone(),
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(ext_self::callback_list_farms_by_seed(
            token_id,
            farm_id,
            env::current_account_id(),
            0,
            Gas(120_000_000_000_000),
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
            if farm.farm_id == farm_id {
                if farm.farm_status != String::from("Running") {
                    let strat = self
                        .strategies
                        .get_mut(&token_id)
                        .expect(ERR21_TOKEN_NOT_REG);
                    let compounder = strat.get_mut();

                    compounder.state = AutoCompounderState::Ended;
                    return PromiseOrValue::Value(format!("The farm {:#?} ended", farm_id));
                }
            }
        }

        PromiseOrValue::Promise(
            ext_farm::get_unclaimed_reward(
                env::current_account_id(),
                farm_id,
                self.farm_contract_id.clone(),
                1,
                Gas(3_000_000_000_000),
            )
            .then(ext_self::callback_post_get_unclaimed_reward(
                token_id,
                env::current_account_id(),
                0,
                Gas(50_000_000_000_000),
            )),
        )
    }

    #[private]
    pub fn callback_post_get_unclaimed_reward(
        &mut self,
        #[callback_result] reward_amount_result: Result<U128, PromiseError>,
        token_id: String,
    ) -> U128 {
        assert!(reward_amount_result.is_ok(), "ERR_GET_REWARD_FAILED");

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);

        ext_farm::claim_reward_by_farm(
            strat.get_ref().farm_id.clone(),
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        );

        let reward_amount = reward_amount_result.unwrap();
        let strat = self
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();
        compounder.last_reward_amount = reward_amount.into();
        reward_amount
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    pub fn withdraw_of_reward(&mut self, token_id: String) -> Promise {
        self.assert_strategy_running(token_id.clone());
        self.is_allowed_account();

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let reward_token: AccountId = strat.clone().get().reward_token;

        let compounder = strat.get_ref();
        let amount = compounder.last_reward_amount;

        ext_farm::withdraw_reward(
            reward_token.to_string(),
            U128(amount),
            "false".to_string(),
            self.farm_contract_id.clone(),
            1,
            Gas(180_000_000_000_000),
        )
        .then(ext_self::callback_post_withdraw(
            token_id,
            U128(amount),
            env::current_account_id(),
            0,
            Gas(20_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<(), PromiseError>,
        token_id: String,
        amount: U128,
    ) -> U128 {
        let strat = self
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();

        if withdraw_result.is_err() {
            compounder.last_reward_amount = 0u128;
            log!("ERR_WITHDRAW_FROM_FARM_FAILED");
            return U128(0);
        }

        amount
    }

    /// Step 3
    /// Transfer lp tokens to ref-exchange then swap the amount the contract has in the exchange
    pub fn autocompounds_swap(&mut self, token_id: String) -> Promise {
        self.assert_strategy_running(token_id.clone());
        self.is_allowed_account();

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();

        let token1 = compounder.token1_address.clone();
        let token2 = compounder.token2_address.clone();
        let reward = compounder.reward_token.clone();

        let mut common_token = 0;

        if token1 == reward {
            common_token = 1;
        } else if token2 == reward {
            common_token = 2;
        }

        let amount_in = U128(compounder.last_reward_amount / 2);

        ext_reward_token::ft_transfer_call(
            self.exchange_contract_id.clone(),         // receiver_id,
            compounder.last_reward_amount.to_string(), //Amount after withdraw the rewards
            "".to_string(),
            compounder.reward_token.clone(),
            1,
            Gas(40_000_000_000_000),
        )
        .then(ext_self::get_tokens_return(
            token_id.clone(),
            amount_in,
            amount_in,
            common_token,
            env::current_account_id(),
            0,
            Gas(60_000_000_000_000),
        ))
        .then(ext_self::swap_to_auto(
            token_id,
            amount_in,
            amount_in,
            common_token,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    #[private]
    pub fn get_tokens_return(
        &self,
        #[callback_result] ft_transfer_result: Result<U128, PromiseError>,
        token_id: String,
        amount_token_1: U128,
        amount_token_2: U128,
        common_token: u64,
    ) -> Promise {
        assert!(ft_transfer_result.is_ok(), "ERR_REWARD_TRANSFER_FAILED");

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();

        if common_token == 1 {
            ext_exchange::get_return(
                compounder.pool_id_token2_reward,
                compounder.reward_token.clone(),
                amount_token_2,
                compounder.token2_address.clone(),
                self.exchange_contract_id.clone(),
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
                self.exchange_contract_id.clone(),
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
                self.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .and(ext_exchange::get_return(
                compounder.pool_id_token2_reward,
                compounder.reward_token.clone(),
                amount_token_2,
                compounder.token2_address.clone(),
                self.exchange_contract_id.clone(),
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

        assert!(amount.0 > 0u128, "ERR_SLIPPAGE_TOO_HIGH");

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

        assert!(amount_token1.0 > 0u128, "ERR_SLIPPAGE_TOO_HIGH");
        assert!(amount_token2.0 > 0u128, "ERR_SLIPPAGE_TOO_HIGH");

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

        let (token1_min_out, token2_min_out): (U128, U128) = tokens;

        compounder.available_balance[0] = token1_min_out.0;
        compounder.available_balance[1] = token2_min_out.0;

        log!(
            "min amount out: has {} for {} and {} for {}",
            token1_min_out.0,
            token_out1,
            token2_min_out.0,
            token_out2
        );

        //Actualization of reward amount
        compounder.last_reward_amount = 0;

        if common_token == 1 {
            self.call_swap(
                pool_id_to_swap2,
                token_in2.clone(),
                token_out2.clone(),
                Some(amount_in_2),
                token2_min_out,
            )
        } else if common_token == 2 {
            self.call_swap(
                pool_id_to_swap1,
                token_in1.clone(),
                token_out1.clone(),
                Some(amount_in_1),
                token1_min_out,
            )
        } else {
            self.call_swap(
                pool_id_to_swap1,
                token_in1.clone(),
                token_out1.clone(),
                Some(amount_in_1),
                token1_min_out,
            )
            // TODO: should use and
            .then(ext_self::call_swap(
                pool_id_to_swap2,
                token_in2.clone(),
                token_out2.clone(),
                Some(amount_in_2),
                token2_min_out,
                contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
            )) // should use a callback to assert that both tx succeeded
        }
    }

    /// Step 4
    /// Get amount of tokens available then stake it
    pub fn autocompounds_liquidity_and_stake(&mut self, token_id: String) /* -> Promise */
    {
        self.assert_strategy_running(token_id.clone());
        self.is_allowed_account();

        let strat = self
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        let pool_id: u64 = compounder.pool_id;

        let token1_amount = compounder.available_balance[0];
        let token2_amount = compounder.available_balance[1];

        // ensure that in the next run we won't have a balance unless previous steps succeeds
        compounder.available_balance[0] = 0u128;
        compounder.available_balance[1] = 0u128;

        // Add liquidity
        self.call_add_liquidity(
            pool_id,
            vec![U128(token1_amount), U128(token2_amount)],
            None,
        )
        // Get the shares
        .then(ext_self::callback_stake(
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
        .and(ext_exchange::get_pool_shares(
            pool_id,
            env::current_account_id(),
            self.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        ))
        // Update user balance and stake
        .then(ext_self::callback_post_get_pool_shares(
            token_id,
            env::current_account_id(),
            0,
            Gas(120_000_000_000_000),
        ));
    }

    #[private]
    pub fn callback_stake(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
    ) -> U128 {
        assert!(shares_result.is_ok(), "ERR");
        let shares_amount = shares_result.unwrap();

        shares_amount
    }

    /// Receives shares from auto-compound and stake it
    /// Change the user_balance and the auto_compounder balance of lps/shares
    #[private]
    pub fn callback_post_get_pool_shares(
        &mut self,
        #[callback_unwrap] minted_shares_result: U128,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        token_id: String,
    ) {
        assert!(total_shares_result.is_ok(), "ERR");
        let shares_amount = minted_shares_result.0;

        let strat = self
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();

        if shares_amount > 0 {
            let mut total_shares: u128 = 0;

            for (_, balance) in compounder.user_shares.iter() {
                total_shares += balance.total;
            }

            compounder.balance_update(total_shares, shares_amount.clone());
        };

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
            self.farm_contract_id.clone(),
            token_id,
            U128(accumulated_shares),
            "".to_string(),
        );
    }
}
