use crate::*;

/// Auto-compounder strategy methods
#[near_bindgen]
impl Contract {
    // TODO: this method should register in the correct pool/farm
    pub fn create_auto_compounder(
        &mut self,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm: String,
        pool_id: u64,
        seed_min_deposit: U128,
    ) {
        let seed_id: String = format!("{}@{}", self.exchange_contract_id, pool_id);

        let token_id = self.wrap_mft_token_id(&pool_id.to_string());
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

        self.strategies.insert(token_id, compounder);
    }

    #[private]
    pub fn stake(&self, token_id: String, account_id: &AccountId, shares: u128) -> Promise {
        ext_exchange::mft_transfer_call(
            self.farm_contract_id.clone(),
            token_id.clone(),
            U128(shares),
            "".to_string(),
            self.exchange_contract_id.clone(),
            1,
            Gas(80_000_000_000_000),
        )
        .then(ext_self::callback_stake_result(
            token_id,
            account_id.clone(),
            shares,
            env::current_account_id(),
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
        // TODO: remove generic promise check
        assert!(self.check_promise(), "ERR_STAKE_FAILED");

        let strat = self
            .strategies2
            .get_mut(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let compounder = strat.get_mut();

        // let compounder = self
        //     .strategies
        //     .get_mut(&token_id)
        //     .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        // TODO: should each auto-compounder store the amount each address have
        //      or should the contract store it?
        // increment total shares deposited by account
        compounder.increment_user_shares(&account_id, shares);

        format!("The {} added {} to {}", account_id, shares, token_id)
    }

    // #[private]
    // pub fn callback_get_deposits(&self) -> Promise {
    //     assert!(self.check_promise(), "Previous tx failed.");

    //     let (_, contract_id) = self.get_predecessor_and_current_account();
    //     ext_exchange::get_deposits(
    //         contract_id,
    //         self.exchange_contract_id,
    //         1,
    //         Gas(12_000_000_000_000),
    //     )
    // }

    /// Withdraw user lps and send it to the contract.
    pub fn unstake(&mut self, token_id: String, amount_withdrawal: Option<U128>) -> Promise {
        let (caller_id, contract_id) = self.get_predecessor_and_current_account();

        let compounder = self
            .strategies
            .get(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let user_shares = compounder
            .user_shares
            .get(&caller_id)
            .expect("ERR_ACCOUNT_DOES_NOT_EXIST");

        assert!(
            *user_shares != 0,
            "User does not have enough lps to withdraw"
        );

        let amount: U128 = amount_withdrawal.unwrap_or(U128(*user_shares));
        assert!(
            *user_shares >= amount.0,
            "{} is trying to withdrawal {} and only has {}",
            caller_id.clone(),
            amount.0,
            user_shares
        );

        let seed_id: String = compounder.seed_id.clone();

        log!("{} is trying to withdrawal {}", caller_id, amount.0);

        // Unstake shares/lps
        ext_farm::withdraw_seed(
            seed_id,
            amount.clone(),
            "".to_string(),
            self.farm_contract_id.clone(),
            1,
            Gas(180_000_000_000_000),
        )
        .then(ext_exchange::mft_transfer(
            token_id.clone(),
            caller_id.clone(),
            amount.clone(),
            Some("".to_string()),
            self.exchange_contract_id.clone(),
            1,
            Gas(50_000_000_000_000),
        ))
        .then(ext_self::callback_withdraw_shares(
            token_id,
            caller_id,
            amount.clone().0,
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
    ) {
        // TODO: remove generic promise check
        assert!(self.check_promise());
        // assert!(mft_transfer_result.is_ok());

        // Decrement user shares
        let compounder = self
            .strategies
            .get_mut(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        compounder.decrement_user_shares(&account_id, amount);
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
            self.exchange_contract_id.clone(),
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
            self.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
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
            self.exchange_contract_id.clone(),
            970000000000000000000,
            Gas(30_000_000_000_000),
        )
    }

    /// Call the ref get_deposits function.
    fn call_get_deposits(&self, account_id: AccountId) -> Promise {
        ext_exchange::get_deposits(
            account_id,
            self.exchange_contract_id.clone(),
            1,
            Gas(15_000_000_000_000),
        )
    }

    /// Call the ref user_register function.
    /// TODO: remove this if not necessary
    pub fn call_user_register(&self, account_id: AccountId) -> Promise {
        ext_exchange::storage_deposit(
            account_id,
            self.exchange_contract_id.clone(),
            10000000000000000000000,
            Gas(3_000_000_000_000),
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
            self.exchange_contract_id.clone(),
            1,
            Gas(80_000_000_000_000),
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
            self.farm_contract_id.clone(),
            1,
            Gas(180_000_000_000_000),
        )
    }
}

/// Auto-compounder functionality methods
// #[near_bindgen]
// impl Contract {
//     pub fn update_seed_min_deposit(&mut self, min_deposit: U128) -> U128 {
//         self.is_owner();
//         self.seed_min_deposit = min_deposit;
//         self.seed_min_deposit
//     }
// }

/// Auto-compounder internal methods
impl Contract {}
