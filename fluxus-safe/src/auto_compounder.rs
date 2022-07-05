use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SharesBalance {
    /// stores the amount given address deposited
    pub deposited: u128,
    /// stores the amount given address deposited plus the earned shares
    pub total: u128,
}

// #[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AutoCompounder {
    /// Fees struct to be distribute at each round of compound
    pub admin_fees: AdminFees,

    /// Struct that maps addresses to its currents shares added plus the received
    /// from the auto-compound strategy
    pub user_shares: HashMap<AccountId, SharesBalance>,

    /// Keeps tracks of how much shares the contract gained from the auto-compound
    pub protocol_shares: u128,

    /// State is used to update the contract to a Paused/Running state
    pub state: AutoCompounderState,

    /// Used to keep track of the current stage of the auto-compound cycle
    pub cycle_stage: AutoCompounderCycle,

    /// Slippage applied to swaps, range from 0 to 100.
    /// Defaults to 5%. The value will be computed as 100 - slippage
    pub slippage: u128,

    /// Used to keep track of the rewards received from the farm during auto-compound cycle
    pub last_reward_amount: u128,

    /// Used to keep track of the owned amount from fee of the token reward
    /// This will be used to store owned amount if ft_transfer to treasure fails
    pub last_fee_amount: u128,

    /// Address of the first token used by pool
    pub token1_address: AccountId,

    /// Address of the token used by the pool
    pub token2_address: AccountId,

    /// Pool used to swap the reward received by the token used to add liquidity
    pub pool_id_token1_reward: u64,

    /// Pool used to swap the reward received by the token used to add liquidity
    pub pool_id_token2_reward: u64,

    /// Address of the reward token given by the farm
    pub reward_token: AccountId,

    /// Store balance of available token1 and token2
    /// obs: would be better to have it in as a LookupMap, but Serialize and Clone is not available for it
    pub available_balance: Vec<Balance>,

    /// Farm used to auto-compound
    pub farm_id: String,

    /// Pool used to add liquidity and farming
    pub pool_id: u64,

    /// Min LP amount accepted by the farm for stake
    pub seed_min_deposit: U128,

    /// Format expected by the farm to claim and withdraw rewards
    pub seed_id: String,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum AutoCompounderState {
    Running,
    Ended,
    Cleared,
} //Should we really add paused state ? It would be best if our code is at such a synchronicity with ref-farm that we won't ever need to pause it.

impl From<&AutoCompounderState> for String {
    fn from(status: &AutoCompounderState) -> Self {
        match *status {
            AutoCompounderState::Running => String::from("Running"),
            // Define how long the strategy should be on ended state, waiting for withdrawal
            AutoCompounderState::Ended => String::from("Ended"),
            // Latest state, after all withdraw was done
            AutoCompounderState::Cleared => String::from("Cleared"),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum AutoCompounderCycle {
    ClaimReward,
    Withdrawal,
    Swap,
    Stake,
}

impl From<&AutoCompounderCycle> for String {
    fn from(cycle: &AutoCompounderCycle) -> Self {
        match *cycle {
            AutoCompounderCycle::ClaimReward => String::from("Reward"),
            AutoCompounderCycle::Withdrawal => String::from("Withdrawal"),
            AutoCompounderCycle::Swap => String::from("Swap"),
            AutoCompounderCycle::Stake => String::from("Stake"),
        }
    }
}

/// Auto-compounder internal methods
impl AutoCompounder {
    pub(crate) fn new(
        treasury: AccountFee,
        strat_creator: AccountFee,
        sentry_fee: u128,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(treasury, strat_creator, sentry_fee);

        Self {
            admin_fees: admin_fee,
            user_shares: HashMap::new(),
            protocol_shares: 0u128,
            state: AutoCompounderState::Running,
            cycle_stage: AutoCompounderCycle::ClaimReward,
            slippage: 95u128,
            last_reward_amount: 0u128,
            last_fee_amount: 0u128,
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            available_balance: vec![0u128, 0u128],
            farm_id,
            pool_id,
            seed_min_deposit,
            seed_id,
        }
    }

    /// Update user balances based on the user's percentage in the contract.
    pub(crate) fn balance_update(&mut self, total: u128, shares_reward: u128) {
        log!("new_shares_quantity is equal to {}", shares_reward);

        let mut shares_distributed: U256 = U256::from(0u128);

        for (account, mut shares) in self.user_shares.clone() {
            let val = shares.total;
            let acc_percentage = U256::from(val) * U256::from(F) / U256::from(total);

            let casted_reward = U256::from(shares_reward) * acc_percentage;

            let earned_shares: U256 = casted_reward / U256::from(F);

            shares_distributed += earned_shares;

            let new_user_balance: u128 = (U256::from(val) + earned_shares).as_u128();

            shares.total = new_user_balance;

            self.user_shares.insert(account.clone(), shares);
        }

        let residue: u128 = shares_reward - shares_distributed.as_u128();
        log!("Shares residue: {}", residue);
    }

    pub(crate) fn increment_user_shares(&mut self, account_id: &AccountId, shares: Balance) {
        let initial_balance = &mut SharesBalance {
            deposited: 0u128,
            total: 0u128,
        };
        let mut user_shares = self
            .user_shares
            .get(account_id)
            .unwrap_or(initial_balance)
            .clone();

        let user_lps = user_shares.total;

        // TODO: is it possible to use get_mut on user_shares, to avoid using insert?

        if user_lps > 0 {
            user_shares.deposited += shares;
            user_shares.total += shares;
            self.user_shares.insert(account_id.clone(), user_shares);
        } else {
            user_shares.deposited = shares;
            user_shares.total = shares;
            self.user_shares.insert(account_id.clone(), user_shares);
        };
    }

    pub(crate) fn decrement_user_shares(&mut self, account_id: &AccountId, shares: Balance) {
        let mut user_shares = self.user_shares.get(account_id).unwrap().clone();
        let new_shares: u128 = user_shares.total - shares;
        log!(
            "{} had {} and now has {}",
            account_id,
            user_shares.total,
            new_shares
        );

        // The following code resets the initial deposit
        // The earned_shares will return how much the user has earned after the withdraw, not the deposit
        user_shares.deposited = new_shares;
        user_shares.total = new_shares;
        self.user_shares.insert(account_id.clone(), user_shares);
    }

    pub(crate) fn next_cycle(&mut self) {
        match self.cycle_stage {
            AutoCompounderCycle::ClaimReward => self.cycle_stage = AutoCompounderCycle::Withdrawal,
            AutoCompounderCycle::Withdrawal => self.cycle_stage = AutoCompounderCycle::Swap,
            AutoCompounderCycle::Swap => self.cycle_stage = AutoCompounderCycle::Stake,
            AutoCompounderCycle::Stake => self.cycle_stage = AutoCompounderCycle::ClaimReward,
        }
    }

    pub(crate) fn compute_fees(
        &self,
        reward_amount: u128,
        treasury_fee_percentage: u128,
    ) -> (u128, u128, u128, u128) {
        // apply fees to reward amount
        let percent = Percentage::from(treasury_fee_percentage);
        let protocol_amount = percent.apply_to(self.last_reward_amount);

        let percent = Percentage::from(self.admin_fees.sentries_fee);
        let sentry_amount = percent.apply_to(self.last_reward_amount);

        let percent = Percentage::from(self.admin_fees.strat_creator.fee_percentage);
        let strat_creator_amount = percent.apply_to(self.last_reward_amount);

        let remaining_amount =
            reward_amount - protocol_amount - sentry_amount - strat_creator_amount;

        (
            remaining_amount,
            protocol_amount,
            sentry_amount,
            strat_creator_amount,
        )
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
        treasury: AccountFee,
        strat_creator: AccountFee,
        sentry_fee: u128,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(treasury, strat_creator, sentry_fee);

        VersionedCompounder::V101(AutoCompounder {
            admin_fees: admin_fee,
            protocol_shares: 0u128,
            user_shares: HashMap::new(),
            state: AutoCompounderState::Running,
            cycle_stage: AutoCompounderCycle::ClaimReward,
            slippage: 95u128,
            last_reward_amount: 0u128,
            last_fee_amount: 0u128,
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            available_balance: vec![0u128, 0u128],
            farm_id,
            pool_id,
            seed_min_deposit,
            seed_id,
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::hash::Hash;

    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(to_account_id("auto_compounder.near"))
            .signer_account_id(to_account_id("auto_compounder.near"))
            .predecessor_account_id(to_account_id("auto_compounder.near"));
        builder
    }

    pub fn to_account_id(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn create_contract() -> Contract {
        let contract = Contract::new(
            to_account_id("auto_compounder.near"),
            String::from("eth.near").parse().unwrap(),
            String::from("dai.near").parse().unwrap(),
            String::from("treasure.near").parse().unwrap(),
        );

        contract
    }

    // fn create_strat(mut safe_contract: Contract) {}

    // #[test]
    // fn test_balance_update() {
    //     let context = get_context();
    //     testing_env!(context.build());

    //     let mut contract = create_contract();

    //     let near: u128 = 1_000_000_000_000_000_000_000_000; // 1 N

    //     let acc1 = to_account_id("alice.near");
    //     let shares1 = near.clone();

    //     let acc2 = to_account_id("bob.near");
    //     let shares2 = near.clone() * 3;

    //     let token1_address = String::from("eth.near").parse().unwrap();
    //     let token2_address = String::from("dai.near").parse().unwrap();
    //     let pool_id_token1_reward = 0;
    //     let pool_id_token2_reward = 1;
    //     let reward_token = String::from("usn.near").parse().unwrap();
    //     let farm = "0".to_string();
    //     let pool_id = 0;
    //     let seed_min_deposit = U128(10);

    //     contract.create_auto_compounder(
    //         token1_address,
    //         token2_address,
    //         pool_id_token1_reward,
    //         pool_id_token2_reward,
    //         reward_token,
    //         farm,
    //         pool_id,
    //         seed_min_deposit,
    //     );

    //     let token_id = String::from(":0");
    //     // add initial balance for accounts
    //     contract
    //         .strategies
    //         .get_mut(&token_id)
    //         .unwrap()
    //         .user_shares
    //         .insert(acc1.clone(), shares1);

    //     contract
    //         .strategies
    //         .get_mut(&token_id)
    //         .unwrap()
    //         .user_shares
    //         .insert(acc2.clone(), shares2);

    //     let total_shares: u128 = shares1 + shares2;

    //     // distribute shares between accounts
    //     contract
    //         .strategies
    //         .get_mut(&token_id)
    //         .unwrap()
    //         .balance_update(total_shares, near);

    //     // assert account 1 earned 25% from reward shares
    //     let acc1_updated_shares = contract
    //         .strategies
    //         .get(&token_id)
    //         .unwrap()
    //         .user_shares
    //         .get(&acc1)
    //         .unwrap();
    //     assert_eq!(
    //         *acc1_updated_shares, 1250000000000000000000000u128,
    //         "ERR_BALANCE_UPDATE"
    //     );

    //     // assert account 2 earned 75% from reward shares
    //     let acc2_updated_shares = contract
    //         .strategies
    //         .get(&token_id)
    //         .unwrap()
    //         .user_shares
    //         .get(&acc2)
    //         .unwrap();
    //     assert_eq!(
    //         *acc2_updated_shares, 3750000000000000000000000u128,
    //         "ERR_BALANCE_UPDATE"
    //     );
    // }
}
