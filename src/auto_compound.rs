use crate::*;

#[near_bindgen]
impl Contract {
    /// Step 1
    /// Function to claim the reward from the farm contract
    pub fn claim_reward(&mut self, token_id: String) {
        self.assert_contract_running();
        self.is_allowed_account();

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let seed_id: String = strat.clone().get().seed_id;

        ext_farm::claim_reward_by_seed(
            seed_id,
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        );
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    pub fn withdraw_of_reward(&mut self, token_id: String) -> Promise {
        self.assert_contract_running();
        self.is_allowed_account();

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let reward_token: AccountId = strat.clone().get().reward_token;

        ext_farm::get_reward(
            env::current_account_id(),
            reward_token.clone(),
            self.farm_contract_id.clone(),
            1,
            Gas(3_000_000_000_000),
        )
        .then(ext_self::callback_withdraw_rewards(
            reward_token.to_string(),
            token_id,
            env::current_account_id(),
            0,
            Gas(217_000_000_000_000),
        ))
    }

    /// Get the reward claimed and withdraw it.
    #[private]
    pub fn callback_withdraw_rewards(
        &mut self,
        #[callback_result] reward_result: Result<U128, PromiseError>,
        reward_token: String,
        token_id: String,
    ) -> Promise {
        assert!(reward_result.is_ok(), "ERR_COULD_NOT_GET_REWARD");
        let amount: U128 = reward_result.unwrap();

        // TODO: should this method return amount like the old impl?
        //      if so, should be after withdraw_reward succeeds
        ext_farm::withdraw_reward(
            reward_token.clone(),
            amount.clone(),
            "false".to_string(),
            self.farm_contract_id.clone(),
            1,
            Gas(180_000_000_000_000),
        )
        .then(ext_self::callback_post_withdraw(
            token_id,
            amount,
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
        assert!(withdraw_result.is_ok(), "ERR_COULD_NOT_WITHDRAW");

        let strat = self
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);

        let compounder = strat.get_mut();
        compounder.last_reward_amount = amount.into();

        amount
    }

    /// Step 3
    /// Transfer lp tokens to ref-exchange then swap the amount the contract has in the exchange
    pub fn autocompounds_swap(&mut self, token_id: String) -> Promise {
        self.assert_contract_running();
        self.is_allowed_account();

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();

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
            env::current_account_id(),
            0,
            Gas(60_000_000_000_000),
        ))
        .then(ext_self::swap_to_auto(
            token_id,
            amount_in,
            amount_in,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    // TODO: should get the callback from ft_transfer_call and check if it was successful
    #[private]
    pub fn get_tokens_return(
        &self,
        token_id: String,
        amount_token_1: U128,
        amount_token_2: U128,
    ) -> Promise {
        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();

        let token1 = compounder.token1_address.clone();
        let token2 = compounder.token2_address.clone();
        let reward = compounder.reward_token.clone();

        if reward == token1 {
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
                1u64,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else if reward == token2 {
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
                2u64,
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
    ) -> (U128, U128) {
        assert!(token_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_RETURN");

        let amount: U128 = token_out.unwrap();

        assert!(amount.0 > 0u128, "ERR_SLIPPAGE_TOO_HIGH");

        if common_token == 1 {
            (U128(0), amount)
        } else {
            (amount, U128(0))
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

        //Actualization of reward amount
        compounder.last_reward_amount = 0;

        if token1_min_out == U128(0) {
            ext_self::call_swap(
                pool_id_to_swap2,
                token_in2.clone(),
                token_out2.clone(),
                Some(amount_in_2),
                token2_min_out,
                contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
            )
        } else if token2_min_out == U128(0) {
            ext_self::call_swap(
                pool_id_to_swap1,
                token_in1.clone(),
                token_out1.clone(),
                Some(amount_in_1),
                token1_min_out,
                contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
            )
        } else {
            // TODO: call exchange directly
            ext_self::call_swap(
                pool_id_to_swap1,
                token_in1.clone(),
                token_out1.clone(),
                Some(amount_in_1),
                token1_min_out,
                contract_id.clone(),
                0,
                Gas(40_000_000_000_000),
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
    pub fn autocompounds_liquidity_and_stake(&mut self, token_id: String) {
        // TODO: do not need to be mut
        self.assert_contract_running();
        self.is_allowed_account();

        ext_exchange::get_deposits(
            env::current_account_id(),
            self.exchange_contract_id.clone(),
            1,
            Gas(9_000_000_000_000),
        )
        // Add liquidity and stake once again
        .then(ext_self::stake_and_liquidity_auto(
            token_id,
            env::current_account_id(),
            env::current_account_id(),
            0,
            Gas(200_000_000_000_000),
        ));
    }

    /// Auto-compound function.
    ///
    /// Responsible to add liquidity and stake.
    #[private]
    pub fn stake_and_liquidity_auto(
        &mut self,
        #[callback_result] deposits_result: Result<HashMap<AccountId, U128>, PromiseError>,
        token_id: String,
        account_id: AccountId,
    ) {
        // TODO: do not need to be mut
        assert!(deposits_result.is_ok(), "ERR_COULD_NOT_GET_DEPOSITS");

        let strat = self.strategies.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.clone().get();

        let tokens: HashMap<AccountId, U128> = deposits_result.unwrap();

        let pool_id: u64 = compounder.pool_id;
        let token_out1 = compounder.token1_address.to_string();
        let token_out2 = compounder.token2_address.to_string();
        let mut quantity_of_token1 = U128(0);
        let mut quantity_of_token2 = U128(0);

        for (key, val) in tokens.iter() {
            if key.to_string() == token_out1 {
                quantity_of_token1 = *val
            };
            if key.to_string() == token_out2 {
                quantity_of_token2 = *val
            };
        }

        // TODO: require that both quantity are greater than 0

        // instead of passing token1, token2 separated
        // use a vec, in the correct format, then you can easily do this op
        // without any further problems

        // Add liquidity
        self.call_add_liquidity(pool_id, vec![quantity_of_token1, quantity_of_token2], None)
            // Get the shares
            .then(ext_self::callback_stake(
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
            .and(ext_exchange::get_pool_shares(
                pool_id,
                account_id.clone(),
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
        // TODO: do not need to be mut
        assert!(total_shares_result.is_ok(), "ERR");
        let shares_amount = minted_shares_result.0;

        // TODO: do not need to be mut
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

        // TODO: Should call it right away and then use a callback to check the result
        self.call_stake(
            self.farm_contract_id.clone(),
            token_id,
            U128(accumulated_shares),
            "".to_string(),
        );
    }
}
