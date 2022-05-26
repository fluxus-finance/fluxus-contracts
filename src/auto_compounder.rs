use crate::*;

// #[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AutoCompounder {
    // Struct that maps addresses to its currents shares added plus the received
    // from the auto-compound strategy
    pub user_shares: HashMap<AccountId, u128>,

    // Keeps tracks of how much shares the contract gained from the auto-compound
    pub protocol_shares: u128,

    // State is used to update the contract to a Paused/Running state
    // state: RunningState,

    // Used to keep track of the rewards received from the farm during auto-compound cycle
    pub last_reward_amount: u128,

    // Address of the first token used by pool
    pub token1_address: AccountId,

    // Address of the token used by the pool
    pub token2_address: AccountId,

    // Pool used to swap the reward received by the token used to add liquidity
    pub pool_id_token1_reward: u64,

    // Pool used to swap the reward received by the token used to add liquidity
    pub pool_id_token2_reward: u64,

    // Address of the reward token given by the farm
    pub reward_token: AccountId,

    // Farm used to auto-compound
    pub farm: String,

    // Pool used to add liquidity and farming
    pub pool_id: u64,

    // Min LP amount accepted by the farm for stake
    pub seed_min_deposit: U128,

    // Format expected by the farm to claim and withdraw rewards
    pub seed_id: String,
}

/// Auto-compounder internal methods
impl AutoCompounder {
    pub(crate) fn new(
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm: String,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        Self {
            user_shares: HashMap::new(),
            protocol_shares: 0u128,
            last_reward_amount: 0u128,
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            farm,
            pool_id,
            seed_min_deposit,
            seed_id,
        }
    }

    /// Update user balances based on the user's percentage in the contract.
    pub(crate) fn balance_update(&mut self, total: u128, shares_reward: u128) {
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

    pub(crate) fn increment_user_shares(&mut self, account_id: &AccountId, shares: Balance) {
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
}

/// Versioned Farmer, used for lazy upgrade.
/// Which means this structure would upgrade automatically when used.
/// To achieve that, each time the new version comes in,
/// each function of this enum should be carefully re-code!
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedCompounder {
    V101(AutoCompounder),
}

impl VersionedCompounder {
    pub fn new(
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm: String,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        VersionedCompounder::V101(AutoCompounder {
            user_shares: HashMap::new(),
            protocol_shares: 0u128,
            last_reward_amount: 0u128,
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            farm,
            pool_id,
            seed_min_deposit,
            seed_id,
        })
    }
}
