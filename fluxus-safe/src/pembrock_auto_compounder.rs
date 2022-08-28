use crate::*;
use substring::Substring;

const MAX_SLIPPAGE_ALLOWED: u128 = 20;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PembStratFarmInfo {
    /// State is used to update the contract to a Paused/Running state
    pub state: PembAutoCompounderState,

    /// Used to keep track of the current stage of the auto-compound cycle
    pub cycle_stage: PembAutoCompounderCycle,

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

    /// Address of the reward token given by the farm
    pub reward_token: AccountId,

    /// Store balance of available token1 and token2
    /// obs: would be better to have it in as a LookupMap, but Serialize and Clone is not available for it
    pub available_balance: Vec<Balance>,
}

impl PembStratFarmInfo {
    pub(crate) fn next_cycle(&mut self) {
        match self.cycle_stage {
            PembAutoCompounderCycle::ClaimReward => {
                self.cycle_stage = PembAutoCompounderCycle::Withdrawal
            }
            PembAutoCompounderCycle::Withdrawal => self.cycle_stage = PembAutoCompounderCycle::Swap,
            PembAutoCompounderCycle::Swap => self.cycle_stage = PembAutoCompounderCycle::Stake,
            PembAutoCompounderCycle::Stake => {
                self.cycle_stage = PembAutoCompounderCycle::ClaimReward
            }
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
            self.state = PembAutoCompounderState::Ended;
            log!("Slippage too high. State was updated to Ended");
        }
    }
}

// #[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PembrockAutoCompounder {
    /// Fees struct to be distribute at each round of compound
    pub admin_fees: AdminFees,

    // Contract address of the exchange used
    pub exchange_contract_id: AccountId,

    // Contract address of the farm used
    pub farm_contract_id: AccountId,

    /// Address of the first token used by pool
    pub token1_address: AccountId,

    /// Pool used to add liquidity and farming
    pub token_name: String,

    /// Min LP amount accepted by the farm for stake
    pub seed_min_deposit: U128,

    /// Store all farms that were used to compound by some token_id
    pub farms: Vec<PembStratFarmInfo>,

    /// Latest harvest timestamp
    pub harvest_timestamp: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum PembAutoCompounderState {
    Running,
    Ended,
    Cleared,
} //Should we really add paused state ? It would be best if our code is at such a synchronicity with ref-farm that we won't ever need to pause it.

impl From<&PembAutoCompounderState> for String {
    fn from(status: &PembAutoCompounderState) -> Self {
        match *status {
            PembAutoCompounderState::Running => String::from("Running"),
            // Define how long the strategy should be on ended state, waiting for withdrawal
            PembAutoCompounderState::Ended => String::from("Ended"),
            // Latest state, after all withdraw was done
            PembAutoCompounderState::Cleared => String::from("Cleared"),
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum PembAutoCompounderCycle {
    ClaimReward,
    Withdrawal,
    Swap,
    Stake,
}

impl From<&PembAutoCompounderCycle> for String {
    fn from(cycle: &PembAutoCompounderCycle) -> Self {
        match *cycle {
            PembAutoCompounderCycle::ClaimReward => String::from("Reward"),
            PembAutoCompounderCycle::Withdrawal => String::from("Withdrawal"),
            PembAutoCompounderCycle::Swap => String::from("Swap"),
            PembAutoCompounderCycle::Stake => String::from("Stake"),
        }
    }
}

/// Auto-compounder internal methods
impl PembrockAutoCompounder {
    pub(crate) fn new(
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token1_address: AccountId,
        token_name: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        Self {
            admin_fees: admin_fee,
            exchange_contract_id,
            farm_contract_id,
            token1_address,
            seed_min_deposit,
            token_name,
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

    pub fn stake_on_pembrock(
        &self,
        account_id: &AccountId,
        shares: u128,
        strat_name: String,
    ) -> Promise {
        let farm_contract_id = self.farm_contract_id.clone();
        let token_contract = self.token1_address.clone();
        log!(
            "Farm _contract_id is: {} current account is {} ",
            farm_contract_id,
            env::current_account_id()
        );

        log!("Inside stake_on_pembrock");

        ext_pembrock::ft_transfer_call(
            farm_contract_id,
            U128(shares),
            "deposit".to_string(),
            token_contract,
            1,
            Gas(80_000_000_000_000),
        )
        // substitute for a generic callback, with a match for next step
        .then(callback_pembrock::callback_pembrock_stake_result(
            strat_name,
            account_id.clone(),
            shares,
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    // pub fn internal_stake_resolver(
    //     &self,
    //     exchange_account_id: SupportedExchanges,
    //     token_id: String,
    //     account_id: &AccountId,
    //     shares: u128,
    // ) {
    //     match exchange_account_id {
    //         SupportedExchanges::RefFinance => self.stake_on_ref_finance(),
    //         SupportedExchanges::Jumbo => self.stake_on_jumbo(),
    //         SupportedExchanges::Pembrock => self.stake_on_pembrock(),

    //     }
    // }

    pub fn stake(
        &self,
        token_id: String,
        seed_id: String,
        account_id: &AccountId,
        shares: u128,
    ) -> Promise {
        let exchange_contract_id = self.exchange_contract_id.clone();
        let farm_contract_id = self.farm_contract_id.clone();

        // decide which strategies
        ext_exchange::mft_transfer_call(
            farm_contract_id,
            token_id,
            U128(shares),
            "\"Free\"".to_string(),
            exchange_contract_id,
            1,
            Gas(80_000_000_000_000),
        )
        // substitute for a generic callback, with a match for next step
        .then(callback_ref_finance::callback_stake_result(
            seed_id,
            account_id.clone(),
            shares,
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }
}

pub enum SupportedExchanges {
    RefFinance,
    Jumbo,
    Pembrock,
}

/// Versioned Farmer, used for lazy upgrade.
/// Which means this structure would upgrade automatically when used.
/// To achieve that, each time the new version comes in,
/// each function of this enum should be carefully re-code!
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedCompounder {
    V101(AutoCompounder),
}

// impl VersionedCompounder {
//     #[allow(dead_code)]
//     pub fn new(
//         strategy_fee: u128,
//         treasury: AccountFee,
//         strat_creator: AccountFee,
//         sentry_fee: u128,
//         exchange_contract_id: AccountId,
//         farm_contract_id: AccountId,
//         token1_address: AccountId,
//         token2_address: AccountId,
//         pool_id: u64,
//         seed_id: String,
//         seed_min_deposit: U128,
//     ) -> Self {
//         let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

//         VersionedCompounder::V101(AutoCompounder {
//             admin_fees: admin_fee,
//             exchange_contract_id,
//             farm_contract_id,
//             token1_address,
//             token2_address,
//             pool_id,
//             seed_min_deposit,
//             seed_id,
//             farms: Vec::new(),
//             harvest_timestamp: 0u64,
//         })
//     }
// }
