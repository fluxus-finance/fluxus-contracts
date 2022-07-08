use crate::*;

/// Auto-compounder strategy methods
#[near_bindgen]
impl Contract {
    #[private]
    pub fn stake(&self, token_id: String, account_id: &AccountId, shares: u128) -> Promise {
        // decide which strategies
        ext_exchange::mft_transfer_call(
            self.data().farm_contract_id.clone(),
            token_id.clone(),
            U128(shares),
            "".to_string(),
            self.data().exchange_contract_id.clone(),
            1,
            Gas(80_000_000_000_000),
        )
        // substitute for a generic callback, with a match for next step
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
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let compounder = strat.get_mut();

        // increment total shares deposited by account
        compounder.increment_user_shares(&account_id, shares);

        format!("The {} added {} to {}", account_id, shares, token_id)
    }

    /// Withdraw user lps and send it to the contract.
    pub fn unstake(&self, token_id: String, amount_withdrawal: Option<U128>) -> Promise {
        let (caller_id, contract_id) = self.get_predecessor_and_current_account();

        let strat = self
            .data()
            .strategies
            .get(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let compounder = strat.clone().get();

        let user_shares = compounder
            .user_shares
            .get(&caller_id)
            .expect("ERR_ACCOUNT_DOES_NOT_EXIST");

        assert!(
            user_shares.total != 0,
            "User does not have enough lps to withdraw"
        );

        let amount: U128 = amount_withdrawal.unwrap_or(U128(user_shares.total));
        assert!(
            user_shares.total >= amount.0,
            "{} is trying to withdrawal {} and only has {}",
            caller_id.clone(),
            amount.0,
            user_shares.total
        );

        log!("{} is trying to withdrawal {}", caller_id, amount.0);

        // Unstake shares/lps
        ext_exchange::get_pool_shares(
            compounder.pool_id,
            contract_id.clone(),
            self.data().exchange_contract_id.clone(),
            0,
            Gas(20_000_000_000_000),
        )
        .then(ext_self::callback_get_pool_shares(
            token_id.clone(),
            caller_id.clone(),
            amount.0,
            contract_id.clone(),
            0,
            Gas(230_000_000_000_000),
        ))
        .then(ext_self::callback_withdraw_shares(
            token_id,
            caller_id,
            amount.0,
            contract_id,
            0,
            Gas(20_000_000_000_000),
        ))
    }

    #[private]
    pub fn callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
    ) -> Promise {
        assert!(shares_result.is_ok(), "ERR");

        let strat = self
            .data()
            .strategies
            .get(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let compounder = strat.clone().get();

        let shares_on_exchange: u128 = shares_result.unwrap().into();

        if shares_on_exchange >= withdraw_amount {
            ext_exchange::mft_transfer(
                token_id.clone(),
                receiver_id,
                U128(withdraw_amount),
                Some("".to_string()),
                self.data().exchange_contract_id.clone(),
                1,
                Gas(30_000_000_000_000),
            )
        } else {
            let amount = withdraw_amount - shares_on_exchange;

            // withdraw missing amount from farm
            ext_farm::withdraw_seed(
                compounder.seed_id,
                U128(amount),
                "".to_string(),
                self.data().farm_contract_id.clone(),
                1,
                Gas(180_000_000_000_000),
            )
            // transfer the total amount required
            .then(ext_exchange::mft_transfer(
                token_id.clone(),
                receiver_id,
                U128(withdraw_amount),
                Some("".to_string()),
                self.data().exchange_contract_id.clone(),
                1,
                Gas(30_000_000_000_000),
            ))
        }
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

        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect("ERR_TOKEN_ID_DOES_NOT_EXIST");

        let compounder = strat.get_mut();

        // Decrement user shares
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
        pool_id: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    ) -> Promise {
        ext_exchange::swap(
            vec![SwapAction {
                pool_id,
                token_in,
                token_out,
                amount_in,
                min_amount_out,
            }],
            None,
            self.data().exchange_contract_id.clone(),
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
            self.data().exchange_contract_id.clone(),
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
            self.data().exchange_contract_id.clone(),
            970000000000000000000,
            Gas(30_000_000_000_000),
        )
    }

    /// Call the ref user_register function.
    /// TODO: remove this if not necessary
    pub fn call_user_register(&self, account_id: AccountId) -> Promise {
        ext_exchange::storage_deposit(
            account_id,
            self.data().exchange_contract_id.clone(),
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
            self.data().exchange_contract_id.clone(),
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
            self.data().farm_contract_id.clone(),
            1,
            Gas(180_000_000_000_000),
        )
    }

    #[private]
    pub fn send_reward_to_sentry(&mut self, token_id: String, sentry_acc_id: AccountId) -> Promise {
        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        // reset default sentry address and get last earned amount
        let amount = compounder
            .admin_fees
            .sentries
            .remove(&env::current_account_id())
            .unwrap();

        ext_reward_token::ft_transfer_call(
            sentry_acc_id.clone(),
            U128(amount),
            "".to_string(),
            compounder.reward_token.clone(),
            1,
            Gas(20_000_000_000_000),
        )
        .then(ext_self::callback_post_sentry_mft_transfer(
            token_id,
            sentry_acc_id,
            amount,
            env::current_account_id(),
            0,
            Gas(20_000_000_000_000),
        ))
    }

    /// Callback to verify that transfer to treasure succeeded
    #[private]
    pub fn callback_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        token_id: String,
        sentry_id: AccountId,
        amount_earned: u128,
    ) {
        // in the case where the transfer failed, the next cycle will send it plus the new amount earned
        if ft_transfer_result.is_err() {
            log!("Transfer to sentry failed".to_string());

            let strat = self
                .data_mut()
                .strategies
                .get_mut(&token_id)
                .expect(ERR21_TOKEN_NOT_REG);

            let compounder = strat.get_mut();

            // store amount earned by sentry to be redeemed
            compounder
                .admin_fees
                .sentries
                .insert(sentry_id, amount_earned);

            return;
        }

        log!("Transfer to {} succeeded", sentry_id)
    }

    // /// Sentry user can redeem manually earned reward
    // pub fn redeem_reward(&self, token_id: String) -> Promise {
    //     let sentry_acc_id = env::predecessor_account_id();

    //     let strat = self
    //         .data()
    //         .strategies
    //         .get(&token_id)
    //         .expect(ERR21_TOKEN_NOT_REG);

    //     let compounder = strat.get_ref();

    //     assert!(compounder.admin_fees.sentries.contains_key(&sentry_acc_id));

    //     let amount = *compounder.admin_fees.sentries.get(&sentry_acc_id).unwrap();
    //     ext_exchange::mft_transfer(
    //         compounder.reward_token.to_string(),
    //         sentry_acc_id.clone(),
    //         U128(amount),
    //         Some("".to_string()),
    //         self.data().exchange_contract_id.clone(),
    //         1,
    //         Gas(20_000_000_000_000),
    //     )
    //     .then(ext_self::callback_post_sentry_mft_transfer(
    //         token_id,
    //         sentry_acc_id,
    //         amount,
    //         env::current_account_id(),
    //         0,
    //         Gas(20_000_000_000_000),
    //     ))
    // }
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
