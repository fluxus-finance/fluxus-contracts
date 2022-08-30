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
    pub available_balance: Balance,
}

impl PembStratFarmInfo {
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

    /// Contract address of the exchange used to swap
    pub exchange_contract_id: AccountId,

    /// Contract address of pembrock
    pub pembrock_contract_id: AccountId,

    /// Contract of the reward token middleware
    pub pembrock_reward_id: AccountId,

    /// Address of the first token used by pool
    pub token1_address: AccountId,

    /// Pool used to add liquidity and farming
    pub token_name: String,

    /// Min LP amount accepted by the farm for stake
    // pub seed_min_deposit: U128,

    /// Store all farms that were used to compound by some token_id
    // pub farms: Vec<PembStratFarmInfo>,

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
    pub available_balance: Balance,

    /// Latest harvest timestamp
    pub harvest_timestamp: u64,

    pub harvest_value_available_to_stake: u128,
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
    SwapAndLend,
}

impl From<&PembAutoCompounderCycle> for String {
    fn from(cycle: &PembAutoCompounderCycle) -> Self {
        match *cycle {
            PembAutoCompounderCycle::ClaimReward => String::from("Reward"),
            PembAutoCompounderCycle::SwapAndLend => String::from("SwapAndLend"),
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
        pembrock_contract_id: AccountId,
        pembrock_reward_id: AccountId,
        token1_address: AccountId,
        token_name: String,
        pool_id: u64,
        reward_token: AccountId,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        Self {
            admin_fees: admin_fee,
            exchange_contract_id,
            pembrock_contract_id,
            pembrock_reward_id,
            token1_address,
            token_name,
            state: PembAutoCompounderState::Running,
            cycle_stage: PembAutoCompounderCycle::ClaimReward,
            slippage: 99u128,
            last_reward_amount: 0u128,
            last_fee_amount: 0u128,
            pool_id_token1_reward: pool_id,
            // TODO: pass as parameter
            reward_token,
            available_balance: 0u128,
            harvest_timestamp: 0u64,
            harvest_value_available_to_stake: 0u128,
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

    pub fn stake_on_pembrock(
        &self,
        account_id: &AccountId,
        shares: u128,
        strat_name: String,
    ) -> Promise {
        let farm_contract_id = self.pembrock_contract_id.clone();
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

    pub fn claim_reward(&mut self, strat_name: String) -> Promise {
        ext_pembrock::claim(self.pembrock_reward_id.clone(), 1, Gas(100_000_000_000_000)).then(
            callback_pembrock::callback_pembrock_rewards(
                strat_name,
                env::current_account_id(),
                0,
                Gas(120_000_000_000_000),
            ),
        )
    }

    pub fn swap_and_lend(&mut self, strat_name: String) -> Promise {
        ext_ref_exchange::get_return(
            self.pool_id_token1_reward,
            self.reward_token.clone(),
            U128(self.last_reward_amount),
            self.token1_address.clone(),
            self.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_pembrock::callback_pembrock_swap(
            strat_name,
            env::current_account_id(),
            0,
            Gas(250_000_000_000_000),
        ))
    }

    pub(crate) fn next_cycle(&mut self) {
        match self.cycle_stage {
            PembAutoCompounderCycle::ClaimReward => {
                self.cycle_stage = PembAutoCompounderCycle::SwapAndLend
            }
            PembAutoCompounderCycle::SwapAndLend => {
                self.cycle_stage = PembAutoCompounderCycle::ClaimReward
            }
        }
    }
}
