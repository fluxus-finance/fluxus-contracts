use crate::*;

#[near_bindgen]
impl Contract {
    /// Function to claim the reward from the farm contract
    pub fn claim_reward(&mut self) {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        ext_farm::claim_reward_by_seed(
            self.seed_id.to_string(),
            self.farm_contract_id.parse().unwrap(), // contract account id
            0,                                      // yocto NEAR to attach
            Gas(40_000_000_000_000),                // gas to attach//was 40?
        );
    }

    /// Function to claim the reward from the farm contract
    #[payable]
    pub fn withdraw_of_reward(&mut self) {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        let token_id: AccountId = self.reward_token.parse().unwrap();

        ext_farm::get_reward(
            env::current_account_id().try_into().unwrap(),
            token_id.clone(),
            self.farm_contract_id.parse().unwrap(), // contract account id
            1,                                      // yocto NEAR to attach
            Gas(3_000_000_000_000),                 // gas to attach
        )
        .then(ext_self::callback_withdraw_rewards(
            token_id.to_string(),
            env::current_account_id(),
            1,
            // obs: pass exactly 190
            Gas(190_000_000_000_000),
        ));
    }

    /// Get the reward claimed and withdraw it.
    #[private]
    #[payable]
    pub fn callback_withdraw_rewards(&mut self, token_id: String) -> U128 {
        assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
        let amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(tokens) => {
                if let Ok(amount) = near_sdk::serde_json::from_slice::<U128>(&tokens) {
                    ext_farm::withdraw_reward(
                        token_id,
                        amount,
                        "false".to_string(),
                        self.farm_contract_id.parse().unwrap(), // contract account id
                        1,                                      // yocto NEAR to attach
                        Gas(180_000_000_000_000),               // gas to attach
                    );
                    amount
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };

        //Storing reward amount
        let amount_in_u128: u128 = amount.into();

        let residue: u128 = self.last_reward_amount;
        self.last_reward_amount = amount_in_u128 + residue;

        amount
    }

    /// Transfer lp tokens to ref-exchange then swap the amount the contract has in the exchange
    #[private]
    #[payable]
    pub fn autocompounds_swap(&mut self) -> Promise {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        let amount_in = U128(self.last_reward_amount / 2);

        ext_reward_token::ft_transfer_call(
            self.exchange_contract_id.parse().unwrap(), // receiver_id,
            self.last_reward_amount.to_string(),        //Amount after withdraw the rewards
            "".to_string(),
            self.reward_token.parse().unwrap(),
            1,
            Gas(45_000_000_000_000),
        )
        .then(ext_self::get_tokens_return(
            amount_in,
            amount_in,
            env::current_account_id(),
            0,
            Gas(60_000_000_000_000),
        ))
        .then(ext_self::swap_to_auto(
            amount_in,
            amount_in,
            env::current_account_id(),
            0,
            Gas(141_000_000_000_000),
        ))
    }

    #[private]
    pub fn get_tokens_return(&self, amount_token_1: U128, amount_token_2: U128) -> Promise {
        ext_exchange::get_return(
            self.pool_id_token1_reward,
            self.reward_token.parse().unwrap(),
            amount_token_1,
            self.token1_address.parse().unwrap(),
            self.exchange_contract_id.parse().unwrap(),
            0,
            Gas(10_000_000_000_000),
        )
        .and(ext_exchange::get_return(
            self.pool_id_token2_reward,
            self.reward_token.parse().unwrap(),
            amount_token_2,
            self.token2_address.parse().unwrap(),
            self.exchange_contract_id.parse().unwrap(),
            0,
            Gas(10_000_000_000_000),
        ))
        .then(ext_self::callback_get_return(
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    /// Swap the auto-compound rewards
    #[private]
    pub fn swap_to_auto(
        &mut self,
        #[callback_unwrap] tokens: (U128, U128),
        amount_in_1: U128,
        amount_in_2: U128,
    ) -> Promise {
        let (_, contract_id) = self.get_predecessor_and_current_account();

        let pool_id_to_swap1 = self.pool_id_token1_reward;
        let pool_id_to_swap2 = self.pool_id_token2_reward;
        let token_in1 = self.reward_token.parse().unwrap();
        let token_in2 = self.reward_token.parse().unwrap();
        let token_out1 = self.token1_address.parse().unwrap();
        let token_out2 = self.token2_address.parse().unwrap();

        let (token1_min_out, token2_min_out): (U128, U128) = tokens;

        //Actualization of reward amount
        // TODO: move to callback_swaps
        self.last_reward_amount = 0;

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

    /// Get amount of tokens available then stake it
    #[payable]
    pub fn autocompounds_liquidity_and_stake(&mut self) {
        self.assert_contract_running();
        self.check_autocompounds_caller();

        ext_exchange::get_deposits(
            env::current_account_id().try_into().unwrap(),
            self.exchange_contract_id.parse().unwrap(), // contract account id
            1,                                          // yocto NEAR to attach
            Gas(9_000_000_000_000),                     // gas to attach
        )
        // Add liquidity and stake once again
        .then(ext_self::stake_and_liquidity_auto(
            env::current_account_id().try_into().unwrap(),
            env::current_account_id(), // auto_compounder contract id
            970000000000000000000,     // yocto NEAR to attach
            Gas(200_000_000_000_000),  // gas to attach
        ));
    }

    /// Auto-compound function.
    ///
    /// Responsible to add liquidity and stake.
    #[private]
    #[payable]
    pub fn stake_and_liquidity_auto(&mut self, account_id: AccountId) {
        assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
        let is_tokens = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(tokens) => {
                if let Ok(is_tokens) =
                    near_sdk::serde_json::from_slice::<HashMap<AccountId, U128>>(&tokens)
                {
                    is_tokens
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };

        let pool_id_to_add_liquidity = self.pool_id;
        let token_out1 = self.token1_address.to_string();
        let token_out2 = self.token2_address.to_string();
        let mut quantity_of_token1 = U128(0);
        let mut quantity_of_token2 = U128(0);

        for (key, val) in is_tokens.iter() {
            if key.to_string() == token_out1 {
                quantity_of_token1 = *val
            };
            if key.to_string() == token_out2 {
                quantity_of_token2 = *val
            };
        }
        let pool_id: u64 = self.pool_id;

        // Add liquidity
        self.call_add_liquidity(
            pool_id_to_add_liquidity,
            vec![quantity_of_token2, quantity_of_token1],
            None,
        )
        // Get the shares
        .then(ext_exchange::get_pool_shares(
            pool_id,
            account_id.clone().try_into().unwrap(),
            self.exchange_contract_id.parse().unwrap(), // contract account id
            0,                                          // yocto NEAR to attach
            Gas(10_000_000_000_000),                    // gas to attach
        ))
        // Update user balance
        .then(ext_self::callback_to_balance(
            env::current_account_id(),
            0,
            Gas(15_000_000_000_000),
        ))
        .then(ext_self::callback_stake(
            env::current_account_id(),
            0,
            Gas(90_000_000_000_000),
        ));
    }

    /// Read shares for each account registered.
    #[private]
    pub fn callback_to_balance(&mut self) -> String {
        assert_eq!(env::promise_results_count(), 1, "ERR_TOO_MANY_RESULTS");
        let shares = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(tokens) => {
                if let Ok(shares) = near_sdk::serde_json::from_slice::<String>(&tokens) {
                    shares
                } else {
                    env::panic_str("ERR_WRONG_VAL_RECEIVED")
                }
            }
            PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
        };

        if shares.parse::<u128>().unwrap() > 0 {
            let mut total_shares: u128 = 0;

            for (_, val) in self.user_shares.iter() {
                total_shares += *val;
            }

            self.balance_update(total_shares, shares.clone());
        };
        shares
    }
}
