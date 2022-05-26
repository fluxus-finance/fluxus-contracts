use crate::*;

#[near_bindgen]
impl Contract {
    /// Step 1
    /// Function to claim the reward from the farm contract
    pub fn claim_reward(&mut self, token_id: String) {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        let compounder = self.seeds.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let seed_id: String = compounder.seed_id.clone();

        ext_farm::claim_reward_by_seed(
            seed_id,
            self.farm_contract_id.clone(), // contract account id
            0,                             // yocto NEAR to attach
            Gas(40_000_000_000_000),       // gas to attach//was 40?
        );
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    pub fn withdraw_of_reward(&mut self, token_id: String) -> Promise {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        let compounder = self.seeds.get(&token_id).expect(ERR21_TOKEN_NOT_REG);
        let reward_token: AccountId = compounder.reward_token.clone();

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

        let compounder = self.seeds.get_mut(&token_id).expect(ERR21_TOKEN_NOT_REG);
        compounder.last_reward_amount = amount.into();

        amount
    }

    /// Step 3
    /// Transfer lp tokens to ref-exchange then swap the amount the contract has in the exchange
    pub fn autocompounds_swap(&mut self, token_id: String) -> Promise {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        let compounder = self.seeds.get(&token_id).expect(ERR21_TOKEN_NOT_REG);

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

    #[private]
    pub fn get_tokens_return(
        &self,
        token_id: String,
        amount_token_1: U128,
        amount_token_2: U128,
    ) -> Promise {
        let compounder = self.seeds.get(&token_id).expect(ERR21_TOKEN_NOT_REG);

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
        .then(ext_self::callback_get_return(
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_get_return(
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

        let compounder = self.seeds.get(&token_id).expect(ERR21_TOKEN_NOT_REG);

        let pool_id_to_swap1 = compounder.pool_id_token1_reward;
        let pool_id_to_swap2 = compounder.pool_id_token2_reward;
        let token_in1 = compounder.reward_token.clone();
        let token_in2 = compounder.reward_token.clone();
        let token_out1 = compounder.token1_address.clone();
        let token_out2 = compounder.token2_address.clone();

        let (token1_min_out, token2_min_out): (U128, U128) = tokens;

        //Actualization of reward amount
        // TODO: move to callback_swaps
        compounder.clone().last_reward_amount = 0;

        // TODO: call exchange directly
        ext_self::call_swap(
            pool_id_to_swap1,
            token_in1,
            token_out1,
            Some(amount_in_1),
            token1_min_out,
            contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        // TODO: should use and
        .then(ext_self::call_swap(
            pool_id_to_swap2,
            token_in2,
            token_out2,
            Some(amount_in_2),
            token2_min_out,
            contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )) // TODO: should use a callback to assert that both tx succeeded
    }

    // /// Step 4
    // /// Get amount of tokens available then stake it
    // #[payable]
    // pub fn autocompounds_liquidity_and_stake(&mut self) {
    //     self.assert_contract_running();
    //     self.check_autocompounds_caller();

    //     ext_exchange::get_deposits(
    //         env::current_account_id().try_into().unwrap(),
    //         self.exchange_contract_id.parse().unwrap(), // contract account id
    //         1,                                          // yocto NEAR to attach
    //         Gas(9_000_000_000_000),                     // gas to attach
    //     )
    //     // Add liquidity and stake once again
    //     .then(ext_self::stake_and_liquidity_auto(
    //         env::current_account_id().try_into().unwrap(),
    //         env::current_account_id(), // auto_compounder contract id
    //         970000000000000000000,     // yocto NEAR to attach
    //         Gas(200_000_000_000_000),  // gas to attach
    //     ));
    // }

    // /// Auto-compound function.
    // ///
    // /// Responsible to add liquidity and stake.
    // #[private]
    // #[payable]
    // pub fn stake_and_liquidity_auto(&mut self, account_id: AccountId) {
    //     assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
    //     let is_tokens = match env::promise_result(0) {
    //         PromiseResult::NotReady => unreachable!(),
    //         PromiseResult::Successful(tokens) => {
    //             if let Ok(is_tokens) =
    //                 near_sdk::serde_json::from_slice::<HashMap<AccountId, U128>>(&tokens)
    //             {
    //                 is_tokens
    //             } else {
    //                 env::panic_str("ERR_WRONG_VAL_RECEIVED")
    //             }
    //         }
    //         PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
    //     };

    //     let pool_id_to_add_liquidity = self.pool_id;
    //     let token_out1 = self.token1_address.to_string();
    //     let token_out2 = self.token2_address.to_string();
    //     let mut quantity_of_token1 = U128(0);
    //     let mut quantity_of_token2 = U128(0);

    //     for (key, val) in is_tokens.iter() {
    //         if key.to_string() == token_out1 {
    //             quantity_of_token1 = *val
    //         };
    //         if key.to_string() == token_out2 {
    //             quantity_of_token2 = *val
    //         };
    //     }
    //     let pool_id: u64 = self.pool_id;

    //     // Add liquidity
    //     self.call_add_liquidity(
    //         pool_id_to_add_liquidity,
    //         vec![quantity_of_token2, quantity_of_token1],
    //         None,
    //     )
    //     // Get the shares
    //     .then(ext_exchange::get_pool_shares(
    //         pool_id,
    //         account_id.clone().try_into().unwrap(),
    //         self.exchange_contract_id.parse().unwrap(), // contract account id
    //         0,                                          // yocto NEAR to attach
    //         Gas(10_000_000_000_000),                    // gas to attach
    //     ))
    //     // Update user balance
    //     .then(ext_self::callback_to_balance(
    //         env::current_account_id(),
    //         0,
    //         Gas(15_000_000_000_000),
    //     ))
    //     .then(ext_self::callback_stake(
    //         env::current_account_id(),
    //         0,
    //         Gas(90_000_000_000_000),
    //     ));
    // }

    // /// Read shares for each account registered.
    // #[private]
    // pub fn callback_to_balance(&mut self) -> String {
    //     assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
    //     let shares = match env::promise_result(0) {
    //         PromiseResult::NotReady => unreachable!(),
    //         PromiseResult::Successful(tokens) => {
    //             if let Ok(shares) = near_sdk::serde_json::from_slice::<String>(&tokens) {
    //                 shares
    //             } else {
    //                 env::panic_str("ERR_WRONG_VAL_RECEIVED")
    //             }
    //         }
    //         PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
    //     };

    //     if shares.parse::<u128>().unwrap() > 0 {
    //         let mut total_shares: u128 = 0;

    //         for (_, val) in self.user_shares.iter() {
    //             total_shares += *val;
    //         }

    //         self.balance_update(total_shares, shares.clone());
    //     };
    //     shares
    // }
}
