use crate::*;

const MAX_SLIPPAGE_ALLOWED: u128 = 20;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StratFarmInfo {
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
    pub id: String,
}

impl StratFarmInfo {
    pub(crate) fn next_cycle(&mut self) {
        match self.cycle_stage {
            AutoCompounderCycle::ClaimReward => self.cycle_stage = AutoCompounderCycle::Withdrawal,
            AutoCompounderCycle::Withdrawal => self.cycle_stage = AutoCompounderCycle::Swap,
            AutoCompounderCycle::Swap => self.cycle_stage = AutoCompounderCycle::Stake,
            AutoCompounderCycle::Stake => self.cycle_stage = AutoCompounderCycle::ClaimReward,
        }
    }

    pub fn increase_slippage(&mut self) {
        if 100u128 - self.slippage < MAX_SLIPPAGE_ALLOWED {
            // increment slippage
            self.slippage -= 4;

            log!(
                "Slippage updated to {}. It will applied in the next call",
                self.slippage
            );
        } else {
            self.state = AutoCompounderState::Ended;
            log!("Slippage too high. State was updated to Ended");
        }
    }
}

// #[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AutoCompounder {
    /// Fees struct to be distribute at each round of compound
    pub admin_fees: AdminFees,

    /// Address of the first token used by pool
    pub token1_address: AccountId,

    /// Address of the token used by the pool
    pub token2_address: AccountId,

    /// Pool used to add liquidity and farming
    pub pool_id: u64,

    /// Min LP amount accepted by the farm for stake
    pub seed_min_deposit: U128,

    /// Format expected by the farm to claim and withdraw rewards
    pub seed_id: String,

    /// Store all farms that were used to compound by some token_id
    pub farms: Vec<StratFarmInfo>,

    /// Latest harvest timestamp
    pub harvest_timestamp: u64,
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
        strategy_fee: u128,
        treasury: AccountFee,
        strat_creator: AccountFee,
        sentry_fee: u128,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        Self {
            admin_fees: admin_fee,
            token1_address,
            token2_address,
            pool_id,
            seed_min_deposit,
            seed_id,
            farms: Vec::new(),
            harvest_timestamp: 0u64,
        }
    }

    pub(crate) fn compute_fees(&mut self, reward_amount: u128) -> (u128, u128, u128, u128) {
        // apply fees to reward amount
        let percent = Percentage::from(self.admin_fees.strategy_fee);
        let all_fees_amount = percent.apply_to(reward_amount);

        let percent = Percentage::from(self.admin_fees.sentries_fee);
        let sentry_amount = percent.apply_to(all_fees_amount);

        let percent = Percentage::from(self.admin_fees.strat_creator.fee_percentage);
        let strat_creator_amount = percent.apply_to(all_fees_amount);
        let treasury_amount = all_fees_amount - sentry_amount - strat_creator_amount;

        let remaining_amount =
            reward_amount - treasury_amount - sentry_amount - strat_creator_amount;

        (
            remaining_amount,
            treasury_amount,
            sentry_amount,
            strat_creator_amount,
        )
    }

    pub fn get_farm_info(&self, farm_id: &str) -> StratFarmInfo {
        for farm in self.farms.iter() {
            if farm.id == farm_id {
                return farm.clone();
            }
        }

        panic!("Farm does not exist")
    }

    pub fn get_mut_farm_info(&mut self, farm_id: String) -> &mut StratFarmInfo {
        for farm in self.farms.iter_mut() {
            if farm.id == farm_id {
                return farm;
            }
        }

        panic!("Farm does not exist")
    }

    /// Iterate through farms inside a compounder
    /// if `rewards_map` contains the same token than the strat, an reward > 0,
    /// then updates the strat to the next cycle, to avoid claiming the seed multiple times
    /// TODO: what if there are multiple farms with the same token_reward?
    pub fn update_strats_by_seed(&mut self, rewards_map: HashMap<String, U128>) {
        for farm in self.farms.iter_mut() {
            if let Some(reward_earned) = rewards_map.get(&farm.reward_token.to_string()) {
                if reward_earned.0 > 0 {
                    farm.last_reward_amount += reward_earned.0;
                }
            }
        }
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
    #[allow(dead_code)]
    pub fn new(
        strategy_fee: u128,
        treasury: AccountFee,
        strat_creator: AccountFee,
        sentry_fee: u128,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        VersionedCompounder::V101(AutoCompounder {
            admin_fees: admin_fee,
            token1_address,
            token2_address,
            pool_id,
            seed_min_deposit,
            seed_id,
            farms: Vec::new(),
            harvest_timestamp: 0u64,
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
