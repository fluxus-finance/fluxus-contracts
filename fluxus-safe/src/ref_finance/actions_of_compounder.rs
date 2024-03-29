use crate::*;

use crate::callback::*;

/// Auto-compounder strategy methods
#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_stake_result(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        seed_id: String,
        account_id: AccountId,
        shares: u128,
    ) -> String {
        if let Ok(amount) = transfer_result {
            assert_eq!(amount.0, 0, "ERR_STAKE_FAILED");
        } else {
            panic!("ERR_STAKE_FAILED");
        }

        //Total fft_share
        let total_fft = self.total_supply_by_pool_id(seed_id.clone());

        let fft_share_id = self.get_fft_share_id_from_seed(seed_id.clone());

        let data = self.data_mut();

        //Total seed_id
        let total_seed = data.seed_id_amount.get(&seed_id).unwrap_or_default();

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &(total_seed + shares));

        let fft_share_amount = if total_fft == 0 {
            shares
        } else {
            (U256::from(shares) * U256::from(total_fft) / U256::from(total_seed)).as_u128()
        };

        log!(
            "{} {} will be minted for {}",
            fft_share_amount,
            fft_share_id,
            account_id.to_string()
        );
        self.mft_mint(fft_share_id, fft_share_amount, account_id.to_string());

        format!(
            "The {} added {} to {}",
            account_id, fft_share_amount, seed_id
        )
    }

    /// Withdraw user lps and send it to the contract.
    pub fn unstake(&self, seed_id: String, amount_withdrawal: Option<U128>) -> Promise {
        let (caller_id, _) = get_predecessor_and_current_account();

        let strat = self.get_strat(&seed_id);

        let fft_share_id = self.get_fft_share_id_from_seed(seed_id.clone());

        let mut user_fft_shares =
            self.users_fft_share_amount(fft_share_id.clone(), caller_id.to_string());

        assert!(
            user_fft_shares > 0,
            "err: {} does not have enough shares. Only has {} shares",
            caller_id,
            user_fft_shares
        );

        //Total fft_share
        let total_fft = self.total_supply_amount(fft_share_id);

        //Total seed_id
        let total_seed = self.seed_total_amount(&seed_id);

        //Converting user total fft_shares in seed_id:
        let user_shares = (U256::from(user_fft_shares) * U256::from(total_seed)
            / U256::from(total_fft))
        .as_u128();

        let amount: U128;

        if let Some(amount_withdrawal) = amount_withdrawal {
            amount = amount_withdrawal;
            user_fft_shares = (U256::from(amount_withdrawal.0) * U256::from(total_fft)
                / U256::from(total_seed))
            .as_u128();
        } else {
            amount = U128(user_shares);
        }

        assert!(
            user_shares >= amount.0,
            "{} is trying to withdrawal {} and only has {}",
            caller_id,
            amount.0,
            user_shares
        );

        strat.unstake(seed_id, caller_id, amount.0, user_fft_shares)
    }

    #[private]
    pub fn callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise {
        assert!(
            shares_result.is_ok(),
            "ERR: failed to get shares from exchange"
        );

        let compounder = self.get_strat(&seed_id).get_compounder();

        let shares_on_exchange: u128 = shares_result.unwrap().into();

        if shares_on_exchange >= withdraw_amount {
            ext_ref_exchange::mft_transfer(
                token_id,
                receiver_id.clone(),
                U128(withdraw_amount),
                Some("".to_string()),
                compounder.exchange_contract_id,
                1,
                Gas(30_000_000_000_000),
            )
            .then(callback_ref_finance::callback_withdraw_shares(
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        } else {
            let amount = withdraw_amount - shares_on_exchange;

            // withdraw missing amount from farm
            ext_ref_farming::unlock_and_withdraw_seed(
                compounder.seed_id,
                U128(0),
                U128(amount),
                compounder.farm_contract_id.clone(),
                1,
                Gas(180_000_000_000_000),
            )
            // TODO: add callback and then call mft_transfer
            // transfer the total amount required
            .then(ext_ref_exchange::mft_transfer(
                token_id,
                receiver_id.clone(),
                U128(withdraw_amount),
                Some("".to_string()),
                compounder.exchange_contract_id,
                1,
                Gas(30_000_000_000_000),
            ))
            .then(callback_ref_finance::callback_withdraw_shares(
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }
    }

    #[private]
    pub fn callback_withdraw_shares(
        &mut self,
        #[callback_result] mft_transfer_result: Result<(), PromiseError>,
        seed_id: String,
        account_id: AccountId,
        amount: Balance,
        fft_shares: Balance,
    ) {
        assert!(
            mft_transfer_result.is_ok(),
            "ERR: failed to transfer shares to {}",
            account_id
        );

        let data = self.data_mut();
        let total_seed = data.seed_id_amount.get(&seed_id).unwrap_or_default();

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &(total_seed - amount));

        let fft_share_id = self
            .data()
            .fft_share_by_seed_id
            .get(&seed_id)
            .unwrap()
            .clone();

        self.mft_burn(fft_share_id, fft_shares, account_id.to_string());
    }

    #[private]
    pub fn stable_callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise {
        assert!(
            shares_result.is_ok(),
            "ERR: failed to get shares from exchange"
        );

        let stable_compounder = self.get_strat(&seed_id).get_stable_compounder();

        let shares_on_exchange: u128 = shares_result.unwrap().into();

        if shares_on_exchange >= withdraw_amount {
            ext_ref_exchange::mft_transfer(
                token_id,
                receiver_id.clone(),
                U128(withdraw_amount),
                Some("".to_string()),
                stable_compounder.exchange_contract_id,
                1,
                Gas(30_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_withdraw_shares(
                    seed_id,
                    receiver_id,
                    withdraw_amount,
                    user_fft_shares,
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            )
        } else {
            let amount = withdraw_amount - shares_on_exchange;

            // withdraw missing amount from farm
            ext_ref_farming::unlock_and_withdraw_seed(
                stable_compounder.seed_id,
                U128(0),
                U128(amount),
                stable_compounder.farm_contract_id.clone(),
                1,
                Gas(180_000_000_000_000),
            )
            // TODO: add callback and then call mft_transfer
            // transfer the total amount required
            .then(ext_ref_exchange::mft_transfer(
                token_id,
                receiver_id.clone(),
                U128(withdraw_amount),
                Some("".to_string()),
                stable_compounder.exchange_contract_id,
                1,
                Gas(30_000_000_000_000),
            ))
            .then(
                callback_stable_ref_finance::stable_callback_withdraw_shares(
                    seed_id,
                    receiver_id,
                    withdraw_amount,
                    user_fft_shares,
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            )
        }
    }

    #[private]
    pub fn stable_callback_withdraw_shares(
        &mut self,
        #[callback_result] mft_transfer_result: Result<(), PromiseError>,
        seed_id: String,
        account_id: AccountId,
        amount: Balance,
        fft_shares: Balance,
    ) {
        assert!(
            mft_transfer_result.is_ok(),
            "ERR: failed to transfer shares to {}",
            account_id
        );

        let data = self.data_mut();
        let total_seed = data.seed_id_amount.get(&seed_id).unwrap_or_default();

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &(total_seed - amount));

        let fft_share_id = self
            .data()
            .fft_share_by_seed_id
            .get(&seed_id)
            .unwrap()
            .clone();

        self.mft_burn(fft_share_id, fft_shares, account_id.to_string());
    }
}

/// Auto-compounder ref-exchange wrapper
#[near_bindgen]
impl Contract {
    /// Call the swap function in the exchange. It can be used by itself or as a callback.
    #[private]
    pub fn call_swap(
        &self,
        exchange_contract_id: AccountId,
        pool_id: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    ) -> Promise {
        ext_ref_exchange::swap(
            vec![SwapAction {
                pool_id,
                token_in,
                token_out,
                amount_in,
                min_amount_out,
            }],
            None,
            exchange_contract_id,
            1,
            Gas(20_000_000_000_000),
        )
    }

    #[private]
    pub fn jumbo_call_swap(
        &self,
        exchange_contract_id: AccountId,
        pool_id: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    ) -> Promise {
        ext_ref_exchange::swap(
            vec![SwapAction {
                pool_id,
                token_in,
                token_out,
                amount_in,
                min_amount_out,
            }],
            None,
            exchange_contract_id,
            1,
            Gas(80_000_000_000_000),
        )
    }

    /// Call the ref get_pool_shares function.
    #[private]
    pub fn call_get_pool_shares(
        &self,
        exchange_contract_id: AccountId,
        pool_id: u64,
        account_id: AccountId,
    ) -> Promise {
        assert!(self.check_promise(), "Previous tx failed.");
        ext_ref_exchange::get_pool_shares(
            pool_id,
            account_id,
            exchange_contract_id,
            0,
            Gas(10_000_000_000_000),
        )
    }

    /// Ref function to add liquidity in the pool.
    #[private]
    pub fn call_add_liquidity(
        &self,
        exchange_contract_id: AccountId,
        pool_id: u64,
        amounts: Vec<U128>,
        min_amounts: Option<Vec<U128>>,
    ) -> Promise {
        ext_ref_exchange::add_liquidity(
            pool_id,
            amounts,
            min_amounts,
            exchange_contract_id,
            970000000000000000000,
            Gas(30_000_000_000_000),
        )
    }

    /// Ref function to stake the lps/shares.
    #[private]
    pub fn call_stake(
        &self,
        exchange_contract_id: AccountId,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        msg: String,
    ) -> Promise {
        ext_ref_exchange::mft_transfer_call(
            receiver_id,
            token_id,
            amount,
            msg,
            exchange_contract_id,
            1,
            Gas(70_000_000_000_000),
        )
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
    //     ext_ref_exchange::mft_transfer(
    //         compounder.reward_token.to_string(),
    //         sentry_acc_id.clone(),
    //         U128(amount),
    //         Some("".to_string()),
    //         self.data().exchange_contract_id.clone(),
    //         1,
    //         Gas(20_000_000_000_000),
    //     )
    //     .then(callback_ref_finance::callback_post_sentry_mft_transfer(
    //         token_id,
    //         sentry_acc_id,
    //         amount,
    //         env::current_account_id(),
    //         0,
    //         Gas(20_000_000_000_000),
    //     ))
    // }
}

// Auto-compounder functionality methods
// #[near_bindgen]
// impl Contract {
//     pub fn update_seed_min_deposit(&mut self, min_deposit: U128) -> U128 {
//         self.is_owner();
//         self.seed_min_deposit = min_deposit;
//         self.seed_min_deposit
//     }
// }
