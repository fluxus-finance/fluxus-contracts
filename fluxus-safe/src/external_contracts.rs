use crate::*;

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct RefStorageState {
    pub deposit: U128,
    pub usage: U128,
}

/// Single swap action.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
    /// Token to swap from.
    pub token_in: AccountId,
    /// Amount to exchange.
    /// If amount_in is None, it will take amount_out from previous step.
    /// Will fail if amount_in is None on the first step.
    pub amount_in: Option<U128>,
    /// Token to swap into.
    pub token_out: AccountId,
    /// Required minimum amount of token_out.
    pub min_amount_out: U128,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FarmInfo {
    pub farm_id: FarmId,
    pub farm_kind: String,
    pub farm_status: String,
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: u32,
    pub reward_per_session: U128,
    pub session_interval: u32,

    pub total_reward: U128,
    pub cur_round: u32,
    pub last_round: u32,
    pub claimed_reward: U128,
    pub unclaimed_reward: U128,
    pub beneficiary_reward: U128,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TermsBoost {
    pub reward_token: String,
    pub start_at: u128,
    pub daily_reward: U128,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FarmInfoBoost {
    pub farm_id: FarmId,
    pub terms: TermsBoost,
    pub total_reward: U128,
    pub distributed_at: String,
    pub distributed_reward: U128,
    pub claimed_reward: U128,
    pub amount_of_beneficiary: U128,
    pub status: String,
}

type SeedId = String;
type FarmId = String;

// Farm functions that we need to call inside the auto_compounder.
#[ext_contract(ext_ref_farming)]
pub trait Farming {
    fn mft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        msg: String,
    );
    fn claim_reward_by_seed(&mut self, seed_id: String);
    fn claim_reward_by_farm(&mut self, farm_id: String);
    fn withdraw_seed(&mut self, seed_id: String, amount: U128, msg: String);
    /// Boost contract
    fn unlock_and_withdraw_seed(
        &mut self,
        seed_id: String,
        unlock_amount: U128,
        withdraw_amount: U128,
    ) -> bool;
    fn withdraw_reward(&mut self, token_id: AccountId, amount: U128, unregister: String)
        -> Promise;
    fn get_reward(&mut self, account_id: AccountId, token_id: AccountId) -> U128;
    fn get_unclaimed_reward(&mut self, account_id: AccountId, farm_id: String) -> U128;
    fn get_unclaimed_rewards(
        &mut self,
        farmer_id: AccountId,
        seed_id: String,
    ) -> HashMap<String, U128>;
    fn list_user_seeds(&self, account_id: AccountId) -> HashMap<SeedId, U128>;
    fn list_farms_by_seed(&self, seed_id: SeedId) -> Vec<FarmInfo>;
    fn list_seed_farms(&self, seed_id: SeedId) -> Vec<FarmInfoBoost>;
}

// Ref exchange functions that we need to call inside the auto_compounder.
#[ext_contract(ext_ref_exchange)]
pub trait RefExchange {
    fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );
    fn get_pool_shares(&mut self, pool_id: u64, account_id: AccountId) -> U128;
    fn metadata(&mut self);
    fn storage_deposit(&mut self, account_id: AccountId);
    fn get_deposits(&mut self, account_id: AccountId);
    fn get_return(
        &self,
        pool_id: u64,
        token_in: AccountId,
        amount_in: U128,
        token_out: AccountId,
    ) -> U128;
    fn add_liquidity(
        &mut self,
        pool_id: u64,
        amounts: Vec<U128>,
        min_amounts: Option<Vec<U128>>,
    ) -> U128;
    fn add_stable_liquidity(&mut self, pool_id: u64, amounts: Vec<U128>, min_shares: U128) -> U128;
    fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<AccountId>) -> U128;
    fn mft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        amount: U128,
        msg: String,
    );
    fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
    fn mft_balance_of(&self, token_id: String, account_id: AccountId) -> U128;
    fn remove_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>);
    fn withdraw(&mut self, token_id: String, amount: U128, unregister: Option<bool>);
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
    fn mft_register(
        token_id: String,
        contract_id: AccountId,
    );
}

// Wrap.testnet functions that we need to call inside the auto_compounder.
#[ext_contract(ext_wrap)]
pub trait Wrapnear {
    fn storage_deposit(&mut self);
    fn near_deposit(&mut self);
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: String, msg: String);
    fn near_withdraw(&mut self, amount: U128);
}

#[ext_contract(ext_reward_token)]
pub trait ExtRewardToken {
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance>;
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PembrockAccount {
    lend_shares: HashMap<AccountId, U128>,
    debt_shares: HashMap<AccountId, U128>,
    total_rewards: U128,
    storage_stake: U128,
}

// dev-v1.slovko.testnet get_account '{"account_id":"mesto-pem.testnet"}'
#[ext_contract(ext_pembrock)]
pub trait ExtPembrock {
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;

    fn withdraw(&mut self, token_id: AccountId, amount: U128) -> PromiseOrValue<U128>;
    fn get_account(&self, account_id: AccountId) -> PembrockAccount;
    fn claim(&self) -> U128;

}

// reward-v1.slovko.testnet get_claimed_rewards
#[ext_contract(ext_pembrock_reward)]
pub trait ExtPembrockReward {
    fn get_claimed_rewards(&self, account_id: AccountId) -> U128;
}

#[ext_contract(ext_jumbo_farming)]
pub trait JumboFarming {
    fn claim_reward_by_farm(&mut self, farm_id: String);
    fn withdraw_seed(&mut self, seed_id: String, amount: U128);
    fn withdraw_reward(&mut self, token_id: AccountId, amount: Option<U128>);
    fn get_unclaimed_reward(&mut self, account_id: AccountId, farm_id: String) -> U128;
    fn list_farms_by_seed(&self, seed_id: SeedId) -> Vec<FarmInfo>;
}

// Ref exchange functions that we need to call inside the auto_compounder.
#[ext_contract(ext_jumbo_exchange)]
pub trait JumboExchange {
    fn exchange_callback_jumbo_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );
    fn get_pool_shares(&mut self, pool_id: u64, account_id: AccountId) -> U128;
    fn metadata(&mut self);
    fn storage_deposit(&mut self, account_id: AccountId);
    fn get_return(
        &self,
        pool_id: u64,
        token_in: AccountId,
        amount_in: U128,
        token_out: AccountId,
    ) -> U128;
    fn add_liquidity(
        &mut self,
        pool_id: u64,
        amounts: Vec<U128>,
        min_amounts: Option<Vec<U128>>,
    ) -> U128;
    fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<AccountId>) -> U128;
    fn mft_transfer_call(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;
    fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
    fn mft_balance_of(&self, token_id: String, account_id: AccountId) -> U128;
    fn remove_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>);
    fn withdraw(&mut self, token_id: AccountId, amount: U128, unregister: Option<bool>);
}
