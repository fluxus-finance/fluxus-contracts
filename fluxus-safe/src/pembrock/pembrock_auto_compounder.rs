use crate::*;

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
    pub token_address: AccountId,

    /// State is used to update the contract to a Paused/Running state
    pub state: PembAutoCompounderState,

    /// Used to keep track of the current stage of the auto-compound cycle
    pub cycle_stage: PembAutoCompounderCycle,

    /// Slippage applied to swaps, range from 0 to 100.
    /// Defaults to 5%. The value will be computed as 100 - slippage
    pub slippage: u128,

    /// Used to keep track of the rewards received from the farm during auto-compound cycle
    pub last_reward_amount: u128,

    /// Fees earned by the DAO
    pub treasury: AccountFee,

    /// Fees earned by the strategy creator
    pub strat_creator_fee_amount: u128,

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

/// Pembrock Auto_compounder possibly states.
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

/// Cycles needed to the Pembrock Auto_compounder.
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
    /// Initialize a new jumbo's compounder.
    /// # Parameters example:
    /// strategy_fee: 5,
    /// strat_creator: { "account_id": "creator_account.testnet", "fee_percentage": 5, "current_amount" : 0 },
    /// sentry_fee: 10,
    /// exchange_contract_id: exchange_contract.testnet,
    /// pembrock_contract_id: pembrock_contract.testnet,
    /// pembrock_reward_id: reward_pembrock.testnet
    /// token1_address: token1.testnet,
    /// pool_id: 17,
    /// reward_token: reward_token.testnet
    pub(crate) fn new(
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        treasure_contract_id: AccountId,
        exchange_contract_id: AccountId,
        pembrock_contract_id: AccountId,
        pembrock_reward_id: AccountId,
        token_address: AccountId,
        pool_id: u64,
        reward_token: AccountId,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        let treasury: AccountFee = AccountFee {
            account_id: treasure_contract_id,
            fee_percentage: 10, //TODO: the treasury fee_percentage can be removed from here as the treasury contract will receive all the fees amount that won't be sent to strat_creator or sentry
            // The breakdown of amount for Stakers, operations and treasury will be dealt with inside the treasury contract
            current_amount: 0u128,
        };

        Self {
            admin_fees: admin_fee,
            exchange_contract_id,
            pembrock_contract_id,
            pembrock_reward_id,
            token_address,
            state: PembAutoCompounderState::Running,
            cycle_stage: PembAutoCompounderCycle::ClaimReward,
            slippage: 99u128,
            last_reward_amount: 0u128,
            treasury,
            strat_creator_fee_amount: 0u128,
            last_fee_amount: 0u128,
            pool_id_token1_reward: pool_id,
            reward_token,
            available_balance: 0u128,
            harvest_timestamp: 0u64,
            harvest_value_available_to_stake: 0u128,
        }
    }

    /// Split reward into fees and reward_remaining.
    /// # Parameters example:
    /// reward_amount: 10000000,
    pub(crate) fn compute_fees(&mut self, reward_amount: u128) -> (u128, u128, u128, u128) {
        // apply fees to reward amount
        let percent = Percentage::from(self.admin_fees.strategy_fee);
        let all_fees_amount = percent.apply_to(reward_amount);

        let percent = Percentage::from(self.admin_fees.sentries_fee);
        let sentry_amount = percent.apply_to(all_fees_amount);

        let percent = Percentage::from(self.admin_fees.strat_creator_fee);
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

    /// Stake the token in the pembrock contract.
    /// # Parameters example:
    /// account_id: account.testnet,
    /// shares: 10000000,
    /// strat_name: pembrock@token_name,
    pub fn stake_on_pembrock(
        &self,
        account_id: &AccountId,
        shares: u128,
        strat_name: String,
    ) -> Promise {
        let farm_contract_id = self.pembrock_contract_id.clone();
        let token_contract = self.token_address.clone();
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

    /// Claim the rewards earned in the pembrock contract.
    /// # Parameters example:
    /// strat_name: pembrock@token_name,
    pub fn claim_reward(&mut self, strat_name: String) -> Promise {
        ext_pembrock::claim(self.pembrock_reward_id.clone(), 1, Gas(100_000_000_000_000)).then(
            callback_pembrock::callback_pembrock_rewards(
                strat_name,
                env::current_account_id(),
                0,
                Gas(150_000_000_000_000),
            ),
        )
    }

    /// Get the token balance and call a function to swap the reward to the right token.
    /// # Parameters example:
    /// strat_name: pembrock@token_name,
    pub fn swap_and_lend(&self, strat_name: String) -> Promise {
        let sentry_acc_id = env::predecessor_account_id();

        ext_reward_token::storage_balance_of(
            sentry_acc_id.clone(),
            self.reward_token.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_pembrock::callback_pembrock_post_sentry(
            strat_name,
            sentry_acc_id,
            self.reward_token.clone(),
            env::current_account_id(),
            0,
            Gas(260_000_000_000_000),
        ))
    }

    /// Function that update the auto_compounder cycle.
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
