use crate::*;

/// Defining cross-contract interface. This allows to create a new promise.
#[ext_contract(callback_ref_finance)]
pub trait RefExchangeAutoCompound {
    fn callback_list_farms_by_seed(
        &self,
        #[callback_result] farms_result: Result<Vec<FarmInfoBoost>, PromiseError>,
        farm_id_str: String,
    ) -> Promise;
    fn callback_post_get_unclaimed_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128>;
    fn callback_post_claim_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        farm_id_str: String,
        reward_amount: U128,
        rewards_map: HashMap<String, U128>,
    ) -> Promise;
    fn callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<bool, PromiseError>,
        farm_id_str: String,
    ) -> Promise;
    fn callback_post_ft_transfer(
        &mut self,
        #[callback_result] exchange_transfer_result: Result<U128, PromiseError>,
        farm_id_str: String,
    );
    fn swap_to_auto(
        &mut self,
        farm_id_str: String,
        amount_in_1: U128,
        amount_in_2: U128,
        common_token: u64,
    );
    fn callback_post_treasury_mft_transfer(
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
    );
    fn callback_post_creator_ft_transfer(
        &mut self,
        #[callback_result] strat_creator_transfer_result: Result<U128, PromiseError>,
        seed_id: String,
    );
    fn callback_get_token_return(&self, common_token: u64, amount_token: U128) -> (U128, U128);
    fn callback_get_tokens_return(&self) -> (U128, U128);
    fn callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
        common_token: u64,
    );
    fn callback_post_first_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
        common_token: u64,
        amount_in: U128,
        token_min_out: U128,
    ) -> PromiseOrValue<u64>;
    fn callback_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        farm_id_str: String,
        sentry_id: AccountId,
        amount_earned: u128,
    );
    // TODO: REMOVE this
    fn call_get_pool_shares(&mut self, pool_id: u64, account_id: AccountId) -> String;
    // TODO: REMOVE this
    fn call_swap(
        &self,
        exchange_contract_id: AccountId,
        pool_id: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    );
    fn callback_update_user_balance(&mut self, account_id: AccountId) -> String;
    fn callback_withdraw_rewards(
        &mut self,
        #[callback_result] reward_result: Result<U128, PromiseError>,
        reward_token: String,
        token_id: String,
    ) -> String;
    fn callback_withdraw_shares(
        &mut self,
        #[callback_result] mft_transfer_result: Result<(), PromiseError>,
        seed_id: String,
        account_id: AccountId,
        amount: Balance,
        fft_shares: Balance,
    );
    fn callback_get_deposits(&self) -> Promise;
    fn callback_post_add_liquidity(
        &mut self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    );
    fn callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    );
    fn callback_stake_result(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        seed_id: String,
        account_id: AccountId,
        shares: u128,
    );

    fn stake_and_liquidity_auto(
        &mut self,
        #[callback_result] deposits_result: Result<HashMap<AccountId, U128>, PromiseError>,
        token_id: String,
        account_id: AccountId,
    );
    fn get_tokens_return(
        &self,
        farm_id_str: String,
        amount_token_1: U128,
        amount_token_2: U128,
        common_token: u64,
    ) -> Promise;

    fn callback_post_unclaimed_rewards(
        &self,
        #[callback_result] rewards_result: Result<HashMap<String, U128>, PromiseError>,
        reward_token: AccountId,
    );
    fn callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise;

    fn callback_post_sentry(
        &self,
        #[callback_result] result: Result<U128, PromiseError>,
        farm_id_str: String,
        sentry_acc_id: AccountId,
        reward_token: AccountId,
    );
}

#[ext_contract(callback_stable_ref_finance)]
pub trait RefExchangeStableAutoCompound {
    fn stable_callback_list_farms_by_seed(
        &self,
        #[callback_result] farms_result: Result<Vec<FarmInfoBoost>, PromiseError>,
        farm_id_str: String,
    ) -> Promise;
    fn stable_callback_post_get_unclaimed_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        farm_id_str: String,
    ) -> PromiseOrValue<u128>;
    fn stable_callback_post_claim_reward(
        &self,
        #[callback_result] claim_result: Result<(), PromiseError>,
        farm_id_str: String,
        reward_amount: U128,
        rewards_map: HashMap<String, U128>,
    ) -> Promise;
    fn stable_callback_post_withdraw(
        &mut self,
        #[callback_result] withdraw_result: Result<bool, PromiseError>,
        farm_id_str: String,
    ) -> Promise;
    fn stable_callback_post_ft_transfer(
        &mut self,
        #[callback_result] exchange_transfer_result: Result<U128, PromiseError>,
        farm_id_str: String,
    );
    fn stable_swap_to_auto(
        &mut self,
        farm_id_str: String,
        amount_in_1: U128,
        amount_in_2: U128,
        common_token: u64,
    );
    fn stable_callback_post_treasury_mft_transfer(
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
    );
    fn stable_callback_post_creator_ft_transfer(
        &mut self,
        #[callback_result] strat_creator_transfer_result: Result<U128, PromiseError>,
        seed_id: String,
    );
    fn stable_callback_get_token_return(&self, farm_id_str: String, amount_token: U128) -> Promise;
    fn stable_callback_get_tokens_return(&self) -> (U128, U128);
    fn stable_callback_post_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
    );
    fn stable_callback_post_first_swap(
        &mut self,
        #[callback_result] swap_result: Result<U128, PromiseError>,
        farm_id_str: String,
        common_token: u64,
        amount_in: U128,
        token_min_out: U128,
    ) -> PromiseOrValue<u64>;
    fn stable_callback_post_sentry_mft_transfer(
        &mut self,
        #[callback_result] ft_transfer_result: Result<(), PromiseError>,
        farm_id_str: String,
        sentry_id: AccountId,
        amount_earned: u128,
    );
    // TODO: REMOVE this
    fn stable_call_get_pool_shares(&mut self, pool_id: u64, account_id: AccountId) -> String;
    // TODO: REMOVE this
    fn stable_call_swap(
        &self,
        exchange_contract_id: AccountId,
        pool_id: u64,
        token_in: AccountId,
        token_out: AccountId,
        amount_in: Option<U128>,
        min_amount_out: U128,
    );
    fn stable_callback_update_user_balance(&mut self, account_id: AccountId) -> String;
    fn stable_callback_withdraw_rewards(
        &mut self,
        #[callback_result] reward_result: Result<U128, PromiseError>,
        reward_token: String,
        token_id: String,
    ) -> String;
    fn stable_callback_withdraw_shares(
        &mut self,
        #[callback_result] mft_transfer_result: Result<(), PromiseError>,
        seed_id: String,
        account_id: AccountId,
        amount: Balance,
        fft_shares: Balance,
    );
    fn stable_callback_get_deposits(&self) -> Promise;
    fn stable_callback_post_add_stable_liquidity(
        &mut self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    ) -> Promise;
    fn stable_callback_post_get_pool_shares(
        &mut self,
        #[callback_result] total_shares_result: Result<U128, PromiseError>,
        farm_id_str: String,
    );
    fn stable_callback_stake_result(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        seed_id: String,
        account_id: AccountId,
        shares: u128,
    );

    fn stable_stake_and_liquidity_auto(
        &mut self,
        #[callback_result] deposits_result: Result<HashMap<AccountId, U128>, PromiseError>,
        token_id: String,
        account_id: AccountId,
    );
    fn stable_get_tokens_return(
        &self,
        farm_id_str: String,
        amount_token_1: U128,
        amount_token_2: U128,
        common_token: u64,
    ) -> Promise;

    fn stable_callback_post_unclaimed_rewards(
        &self,
        #[callback_result] rewards_result: Result<HashMap<String, U128>, PromiseError>,
        reward_token: AccountId,
    );
    fn stable_callback_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise;

    fn stable_callback_post_sentry(
        &self,
        #[callback_result] result: Result<U128, PromiseError>,
        farm_id_str: String,
        sentry_acc_id: AccountId,
        reward_token: AccountId,
    );
}

// pub trait RefExchangeAutoCompound {
//     fn stake(&self, token_id: String, account_id: &AccountId, shares: u128) -> Promise;
//     fn unstake(&mut self, token_id: String, amount_withdrawal: Option<U128>) -> Promise;
// }
