use crate::*;

pub struct AutoCompounder {
    // Struct that maps addresses to its currents shares added plus the received
    // from the auto-compound strategy
    user_shares: HashMap<AccountId, u128>,

    // Keeps tracks of how much shares the contract gained from the auto-compound
    protocol_shares: u128,

    // State is used to update the contract to a Paused/Running state
    state: RunningState,

    // Used to keep track of the rewards received from the farm during auto-compound cycle
    last_reward_amount: u128,

    // Address of the first token used by pool
    token1_address: String,

    // Address of the token used by the pool
    token2_address: String,

    // Pool used to swap the reward received by the token used to add liquidity
    pool_id_token1_reward: u64,

    // Pool used to swap the reward received by the token used to add liquidity
    pool_id_token2_reward: u64,

    // Address of the reward token given by the farm
    reward_token: String,

    // Farm used to auto-compound
    farm: String,

    // Pool used to add liquidity and farming
    pool_id: u64,

    // Format expected by the farm to claim and withdraw rewards
    seed_id: String,

    // Min LP amount accepted by the farm for stake
    seed_min_deposit: U128,
}

/// Auto-compounder internal methods
impl AutoCompounder {
    pub(crate) fn new() -> Self {
        Self {}
    }

    #[private]
    pub fn increment_user_shares(&mut self, account_id: &AccountId, shares: Balance) {
        let user_lps = self.user_shares.get(account_id).unwrap_or(&0);

        if *user_lps > 0 {
            // TODO: improve log
            // log!("");
            let new_balance: u128 = *user_lps + shares;
            self.user_shares.insert(account_id.clone(), new_balance);
        } else {
            // TODO: improve log
            // log!("");
            self.user_shares.insert(account_id.clone(), shares);
        };
    }

    /// Update user balances based on the user's percentage in the contract.
    #[private]
    pub fn balance_update(&mut self, total: u128, shares_reward: String) {
        let shares_reward = shares_reward.parse::<u128>().unwrap();
        log!("new_shares_quantity is equal to {}", shares_reward);

        let mut shares_distributed: U256 = U256::from(0u128);

        for (account, val) in self.user_shares.clone() {
            let acc_percentage = U256::from(val) * U256::from(F) / U256::from(total);

            let casted_reward = U256::from(shares_reward) * acc_percentage;

            let earned_shares: U256 = casted_reward / U256::from(F);

            shares_distributed += earned_shares;

            let new_user_balance: u128 = (U256::from(val) + earned_shares).as_u128();

            self.user_shares.insert(account, new_user_balance);
        }

        let residue: u128 = shares_reward - shares_distributed.as_u128();
        log!("Shares residue: {}", residue);
    }
}
