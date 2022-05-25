use crate::*;

/// Auto-compounder strategy methods
#[near_bindgen]
impl Contract {
    // TODO: this method should register in the correct pool/farm
    pub fn create_auto_compounder(
        &mut self,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id_token1_reward: String,
        pool_id_token2_reward: String,
        reward_token: AccountId,
        farm: String,
        pool_id: String,
        seed_min_deposit: U128,
    ) {
        let seed_id: String = format!("{}@{}", self.exchange_contract_id, pool_id);

        let token_id = self.wrap_mft_token_id(&pool_id);
        self.token_ids.push(token_id.clone());

        let compounder = AutoCompounder::new(
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            farm,
            pool_id,
            seed_id,
            seed_min_deposit,
        );

        self.seeds.insert(token_id, compounder.clone());
        self.compounders.push(compounder);
    }

    #[private]
    pub fn stake(&self, token_id: String, account_id: &AccountId, shares: u128) -> Promise {
        let (_, contract) = self.get_predecessor_and_current_account();

        ext_exchange::mft_transfer_call(
            self.farm_contract_id.parse().unwrap(),
            token_id.clone(),
            U128(shares),
            "".to_string(),
            self.exchange_contract_id.parse().unwrap(),
            1,
            Gas(80_000_000_000_000),
        )
        .then(ext_self::callback_stake_result(
            token_id,
            account_id.clone(),
            shares,
            contract,
            0,
            Gas(10_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_stake_result(
        &mut self,
        token_id: String,
        account_id: AccountId,
        shares: u128,
    ) -> String {
        assert!(self.check_promise(), "ERR_STAKE_FAILED");

        // increment total shares deposited by account
        // TODO: should each auto-compounder store the amount each address have
        //      or should the contract store it?
        // increment_user_shares(&account_id, shares);
        // self.user_shares.insert(account_id.clone(), new_shares);

        let compounder = self
            .seeds
            .get_mut(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        compounder
            .user_shares
            .insert(account_id.clone(), shares.clone());

        format!("The {} added {} to {}", account_id, shares, token_id)
    }

    // #[private]
    // pub fn callback_get_return(
    //     &self,
    //     #[callback_result] token1_out: Result<U128, PromiseError>,
    //     #[callback_result] token2_out: Result<U128, PromiseError>,
    // ) -> (U128, U128) {
    //     assert!(token1_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_1_RETURN");
    //     assert!(token2_out.is_ok(), "ERR_COULD_NOT_GET_TOKEN_2_RETURN");

    //     let mut amount_token1: u128;
    //     let mut amount_token2: u128;

    //     if let Ok(s) = token1_out.as_ref() {
    //         let val: u128 = s.0;
    //         require!(val > 0u128);
    //         amount_token1 = val;
    //     } else {
    //         env::panic_str("ERR_COULD_NOT_DESERIALIZE_TOKEN_1")
    //     }

    //     if let Ok(s) = token2_out.as_ref() {
    //         let val: u128 = s.0;
    //         require!(val > 0u128);
    //         amount_token2 = val;
    //     } else {
    //         env::panic_str("ERR_COULD_NOT_DESERIALIZE_TOKEN_2")
    //     }

    //     (U128(amount_token1), U128(amount_token2))
    // }

    // /// Receives shares from auto-compound and stake it
    // #[private]
    // pub fn callback_stake(&mut self) {
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

    //     let amount: u128 = shares.parse::<u128>().unwrap();
    //     assert!(
    //         amount >= self.seed_min_deposit.into(),
    //         "ERR_NOT_ENOUGH_SHARES_TO_STAKE"
    //     );

    //     //Concatenate ":" with pool id because ref's testnet contract need an argument like this. Ex -> :193
    //     //For Mainnet, probability it is not necessary concatenate the ":"
    //     let pool_id: String = ":".to_string() + &self.pool_id.to_string();

    //     // TODO: Should call it right away and then use a callback to check the result
    //     self.call_stake(
    //         self.farm_contract_id.parse().unwrap(),
    //         pool_id,
    //         U128(amount),
    //         "".to_string(),
    //     );
    // }

    // /// Change the user_balance and the auto_compounder balance of lps/shares
    // #[private]
    // pub fn callback_update_user_balance(
    //     &mut self,
    //     account_id: AccountId,
    //     #[callback_result] shares: Result<String, PromiseError>,
    // ) -> String {
    //     require!(shares.is_ok());

    //     let protocol_shares_on_pool: u128 = match shares {
    //         Ok(shares_) => shares_.parse::<u128>().unwrap(),
    //         _ => env::panic_str("Unknown error occurred"),
    //     };

    //     let shares_added_to_pool = protocol_shares_on_pool - self.protocol_shares;
    //     let user_shares = self.get_user_shares(account_id.clone());

    //     if user_shares == None {
    //         self.user_shares.insert(account_id.clone(), 0);
    //     }

    //     let mut new_user_balance: u128 = 0;

    //     if protocol_shares_on_pool > self.protocol_shares {
    //         if let Some(x) = self.get_user_shares(account_id.clone()) {
    //             Some(new_user_balance = x.parse::<u128>().unwrap() + shares_added_to_pool)
    //         } else {
    //             None
    //         };
    //         self.user_shares.insert(account_id, new_user_balance);
    //         log!("User_shares = {}", new_user_balance);
    //     };
    //     self.protocol_shares = protocol_shares_on_pool;

    //     protocol_shares_on_pool.to_string()
    // }

    // #[private]
    // pub fn callback_get_deposits(&self) -> Promise {
    //     assert!(self.check_promise(), "Previous tx failed.");

    //     let (_, contract_id) = self.get_predecessor_and_current_account();
    //     ext_exchange::get_deposits(
    //         contract_id,
    //         self.exchange_contract_id.parse().unwrap(),
    //         1,                       // yocto NEAR to attach
    //         Gas(12_000_000_000_000), // gas to attach
    //     )
    // }

    /// Withdraw user lps and send it to the contract.
    pub fn unstake(&mut self, token_id: String, amount_withdrawal: Option<U128>) -> Promise {
        let (caller_id, contract_id) = self.get_predecessor_and_current_account();

        // TODO: require that token_id exist
        let compounder = self
            .seeds
            .get(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        // TODO require!(ACCOUNT_EXIST)
        let user_shares = compounder
            .user_shares
            .get(&caller_id)
            .expect("ERR_ACCOUNT_DOES_NOT_EXIST");

        let seed_id: String = compounder.seed_id.clone();
        let shares_available: u128 = *user_shares;

        // TODO: rewrite asserts
        assert!(
            shares_available != 0,
            "User does not have enough lps to withdraw"
        );

        let amount = amount_withdrawal.unwrap_or(U128(shares_available));
        log!("Unstake amount = {}", amount.0);
        assert!(amount.0 != 0, "User is trying to withdraw 0 shares");

        assert!(
            shares_available >= amount.0,
            "User is trying to withdrawal {} and only has {}",
            amount.0,
            shares_available
        );

        // Unstake shares/lps
        ext_farm::withdraw_seed(
            seed_id,
            amount.clone(),
            "".to_string(),
            self.farm_contract_id.parse().unwrap(),
            1,
            Gas(180_000_000_000_000),
        )
        .then(ext_exchange::mft_transfer(
            token_id.clone(),
            caller_id.clone(),
            amount.clone(),
            Some("".to_string()),
            self.exchange_contract_id.parse().unwrap(),
            1,
            Gas(50_000_000_000_000),
        ))
        .then(ext_self::callback_withdraw_shares(
            token_id,
            caller_id,
            amount.clone().0,
            shares_available,
            contract_id,
            0,
            Gas(20_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_withdraw_shares(
        &mut self,
        token_id: String,
        account_id: AccountId,
        amount: Balance,
        shares_available: Balance,
    ) {
        assert!(self.check_promise());
        // TODO: remove generic promise check
        // assert!(mft_transfer_result.is_ok());

        let mut compounder = self
            .seeds
            .get(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let new_shares: u128 = shares_available - amount;
        // self.user_shares.insert(account_id.clone(), new_shares);

        compounder
            .user_shares
            .clone()
            .insert(account_id, new_shares);
    }
}

/// Auto-compounder ref-exchange wrapper
#[near_bindgen]
impl Contract {
    /// Call the swap function in the exchange. It can be used by itself or as a callback.
    #[private]
    #[payable]
    pub fn call_swap(
        &mut self,
        pool_id_to_swap: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    ) -> Promise {
        assert!(self.check_promise(), "Previous tx failed.");
        ext_exchange::swap(
            vec![SwapAction {
                pool_id: pool_id_to_swap,
                token_in: token_in,
                token_out: token_out,
                amount_in: amount_in,
                min_amount_out: min_amount_out,
            }],
            None,
            self.exchange_contract_id.parse().unwrap(),
            1,
            Gas(20_000_000_000_000),
        )
    }

    /// Call the ref get_pool_shares function.
    #[private]
    pub fn call_get_pool_shares(&self, pool_id: u64, account_id: AccountId) -> Promise {
        assert!(self.check_promise(), "Previous tx failed.");
        ext_exchange::get_pool_shares(
            pool_id,
            account_id,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            0,                                          // yocto NEAR to attach
            Gas(10_000_000_000_000),                    // gas to attach
        )
    }

    /// Ref function to add liquidity in the pool.
    pub fn call_add_liquidity(
        &self,
        pool_id: u64,
        amounts: Vec<U128>,
        min_amounts: Option<Vec<U128>>,
    ) -> Promise {
        ext_exchange::add_liquidity(
            pool_id,
            amounts,
            min_amounts,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            970000000000000000000,                      // yocto NEAR to attach /////////////
            Gas(30_000_000_000_000),                    // gas to attach
        )
    }

    /// Call the ref get_deposits function.
    fn call_get_deposits(&self, account_id: AccountId) -> Promise {
        ext_exchange::get_deposits(
            account_id,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            1,                                          // yocto NEAR to attach
            Gas(15_000_000_000_000),                    // gas to attach
        )
    }

    /// Call the ref user_register function.
    /// TODO: remove this if not necessary
    pub fn call_user_register(&self, account_id: AccountId) -> Promise {
        ext_exchange::storage_deposit(
            account_id,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            10000000000000000000000,                    // yocto NEAR to attach
            Gas(3_000_000_000_000),                     // gas to attach
        )
    }

    /// Ref function to stake the lps/shares.
    pub fn call_stake(
        &self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        msg: String,
    ) -> Promise {
        ext_exchange::mft_transfer_call(
            receiver_id,
            token_id,
            amount,
            msg,
            self.exchange_contract_id.parse().unwrap(), // contract account id
            1,                                          // yocto NEAR to attach
            Gas(80_000_000_000_000),                    // gas to attach
        )
    }

    /// Ref function to withdraw the rewards to exchange ref contract.
    pub fn call_withdraw_reward(
        &self,
        token_id: String,
        amount: U128,
        unregister: String,
    ) -> Promise {
        ext_farm::withdraw_reward(
            token_id,
            amount,
            unregister,
            self.farm_contract_id.parse().unwrap(), // contract account id
            1,                                      // yocto NEAR to attach
            Gas(180_000_000_000_000),               // gas to attach
        )
    }
}

/// Auto-compounder functionality methods
// #[near_bindgen]
// impl Contract {
//     pub fn update_seed_min_deposit(&mut self, min_deposit: U128) -> U128 {
//         self.check_permission();
//         self.seed_min_deposit = min_deposit;
//         self.seed_min_deposit
//     }
// }

/// Auto-compounder internal methods
impl Contract {}
