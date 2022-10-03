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

    pub(crate) fn increase_slippage(&mut self) {
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

    // Contract address of the exchange used
    pub exchange_contract_id: AccountId,

    // Contract address of the farm used
    pub farm_contract_id: AccountId,

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
    /// Initialize a new jumbo's compounder.
    /// # Parameters example:
    /// strategy_fee: 5,
    /// strat_creator: { "account_id": "creator_account.testnet", "fee_percentage": 5, "current_amount" : 0 },
    /// sentry_fee: 10,
    /// exchange_contract_id: exchange_contract.testnet,
    /// farm_contract_id: farm_contract.testnet,
    /// token1_address: token1.testnet,
    /// token2_address: token2.testnet,
    /// pool_id: 17,
    /// seed_id: exchange@pool_id,
    /// seed_min_deposit: U128(1000000)
    pub(crate) fn new(
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        Self {
            admin_fees: admin_fee,
            exchange_contract_id,
            farm_contract_id,
            token1_address,
            token2_address,
            pool_id,
            seed_min_deposit,
            seed_id,
            farms: Vec::new(),
            harvest_timestamp: 0u64,
        }
    }

    /// Split reward into fees and reward_remaining.
    /// # Parameters example:
    /// reward_amount: 100000000,
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

    /// Return a farm information.
    /// # Parameters example:
    /// farm_id: 1,
    pub(crate) fn get_farm_info(&self, farm_id: &str) -> StratFarmInfo {
        for farm in self.farms.iter() {
            if farm.id == farm_id {
                return farm.clone();
            }
        }

        panic!("{}", ERR44_FARM_INFO_DOES_NOT_EXIST)
    }

    /// Return a mutable farm information.
    /// # Parameters example:
    /// farm_id: 1,
    pub(crate) fn get_mut_farm_info(&mut self, farm_id: String) -> &mut StratFarmInfo {
        for farm in self.farms.iter_mut() {
            if farm.id == farm_id {
                return farm;
            }
        }

        panic!("{}", ERR44_FARM_INFO_DOES_NOT_EXIST)
    }

    /// Iterate through farms inside a compounder
    /// if `rewards_map` contains the same token than the strat, an reward > 0,
    /// then updates the strat to the next cycle, to avoid claiming the seed multiple times
    /// TODO: what if there are multiple farms with the same token_reward?
    pub(crate) fn update_strats_by_seed(&mut self, rewards_map: HashMap<String, U128>) {
        for farm in self.farms.iter_mut() {
            if let Some(reward_earned) = rewards_map.get(&farm.reward_token.to_string()) {
                if reward_earned.0 > 0 {
                    farm.last_reward_amount += reward_earned.0;
                }
            }
        }
    }

    /// Transfer the amount of the token to the exchange and stake it.
    /// # Parameters example:
    /// token_id: :1,
    /// seed_id: exchange@pool_id,
    /// account_id: account.testnet,
    /// shares: 1000000,
    pub(crate) fn stake(
        &self,
        token_id: String,
        seed_id: String,
        account_id: &AccountId,
        shares: u128,
    ) -> Promise {
        let exchange_contract_id = self.exchange_contract_id.clone();
        let farm_contract_id = self.farm_contract_id.clone();

        // decide which strategies
        ext_ref_exchange::mft_transfer_call(
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

    /// Get the pool shares and then call a function to unstake them.
    /// # Parameters example:
    /// token_id: 1,
    /// seed_id: exchange@pool_id,
    /// receiver_id: receiver_account.testnet,
    /// withdraw_amount: 1000000,
    /// user_fft_shares: 1000000
    pub(crate) fn unstake(
        &self,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise {
        // Unstake shares/lps
        ext_ref_exchange::get_pool_shares(
            self.pool_id,
            env::current_account_id(),
            self.exchange_contract_id.clone(),
            0,
            Gas(20_000_000_000_000),
        )
        .then(callback_ref_finance::callback_get_pool_shares(
            token_id,
            seed_id,
            receiver_id,
            withdraw_amount,
            user_fft_shares,
            env::current_account_id(),
            0,
            Gas(260_000_000_000_000),
        ))
    }

    /// Claim the rewards earned.
    /// # Parameters example:
    /// farm_id_str: exchange@pool_id#farm_id
    pub(crate) fn claim_reward(&self, farm_id_str: String) -> Promise {
        log!("claim_reward");
        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());

        ext_ref_farming::list_seed_farms(
            seed_id,
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(callback_ref_finance::callback_list_farms_by_seed(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    /// Function to withdraw the reward earned and already claimed.
    /// # Parameters example:
    /// farm_id_str: exchange@pool_id#farm_id
    /// treasury_current_amount: 1000000
    pub(crate) fn withdraw_of_reward(
        &self,
        farm_id_str: String,
        treasury_current_amount: u128,
    ) -> Promise {
        log!("withdraw_of_reward");

        let (_, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let farm_info = self.get_farm_info(&farm_id);

        // contract_id does not exist on sentries
        if !self
            .admin_fees
            .sentries
            .contains_key(&env::current_account_id())
        {
            let amount_to_withdraw = farm_info.last_reward_amount;
            ext_ref_farming::withdraw_reward(
                farm_info.reward_token,
                U128(amount_to_withdraw),
                "false".to_string(),
                self.farm_contract_id.clone(),
                0,
                Gas(180_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_withdraw(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(80_000_000_000_000),
            ))
        } else {
            // the withdraw succeeded but not the transfer
            ext_reward_token::ft_transfer_call(
                self.exchange_contract_id.clone(),
                U128(farm_info.last_reward_amount + treasury_current_amount), //Amount after withdraw the rewards
                "".to_string(),
                farm_info.reward_token,
                1,
                Gas(40_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_ft_transfer(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }
    }

    /// Transfer reward token to ref-exchange then swap the amount the contract has in the exchange
    /// # Parameters example:
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   treasure: { "account_id": "creator_account.testnet", "fee_percentage": 5, "current_amount" : 0 },
    pub(crate) fn autocompounds_swap(&self, farm_id_str: String, treasure: AccountFee) -> Promise {
        log!("autocompounds_swap");

        let treasury_acc: AccountId = treasure.account_id;
        let treasury_curr_amount: u128 = treasure.current_amount;

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        let farm_info = self.get_farm_info(&farm_id);

        let token1 = self.token1_address.clone();
        let token2 = self.token2_address.clone();
        let reward = farm_info.reward_token.clone();

        let mut common_token = 0;

        if token1 == reward {
            common_token = 1;
        } else if token2 == reward {
            common_token = 2;
        }

        let reward_amount = farm_info.last_reward_amount;

        // This works by increasing gradually the slippage allowed
        // It will be used only in the cases where the first swaps succeed but not the second
        if farm_info.available_balance[0] > 0 {
            common_token = 1;

            return self
                .get_tokens_return(
                    farm_id_str.clone(),
                    U128(farm_info.available_balance[0]),
                    U128(reward_amount),
                    common_token,
                )
                .then(callback_ref_finance::swap_to_auto(
                    farm_id_str,
                    U128(farm_info.available_balance[0]),
                    U128(reward_amount),
                    common_token,
                    env::current_account_id(),
                    0,
                    Gas(140_000_000_000_000),
                ));
        }

        let amount_in = U128(reward_amount / 2);

        if treasury_curr_amount > 0 {
            ext_ref_exchange::mft_transfer(
                farm_info.reward_token.to_string(),
                treasury_acc,
                U128(treasury_curr_amount),
                Some("".to_string()),
                self.exchange_contract_id.clone(),
                1,
                Gas(20_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_treasury_mft_transfer(
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ));
        }

        let strat_creator_curr_amount = self.admin_fees.strat_creator.current_amount;
        if strat_creator_curr_amount > 0 {
            ext_reward_token::ft_transfer(
                self.admin_fees.strat_creator.account_id.clone(),
                U128(strat_creator_curr_amount),
                Some("".to_string()),
                farm_info.reward_token,
                1,
                Gas(20_000_000_000_000),
            )
            .then(callback_ref_finance::callback_post_creator_ft_transfer(
                seed_id,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ));
        }

        self.get_tokens_return(farm_id_str.clone(), amount_in, amount_in, common_token)
            .then(callback_ref_finance::swap_to_auto(
                farm_id_str,
                amount_in,
                amount_in,
                common_token,
                env::current_account_id(),
                0,
                Gas(140_000_000_000_000),
            ))
    }

    /// Returns how many tokens will be received swapping given amount of token_in for token_out.
    /// # Parameters example:
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   amount_token_1: U128(10000000),
    ///   amount_token_2: U128(10000000),
    ///   common_token: 1
    pub(crate) fn get_tokens_return(
        &self,
        farm_id_str: String,
        amount_token_1: U128,
        amount_token_2: U128,
        common_token: u64,
    ) -> Promise {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let farm_info = self.get_farm_info(&farm_id);

        if common_token == 1 {
            // TODO: can be shortened by call_get_return
            ext_ref_exchange::get_return(
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                amount_token_2,
                self.token2_address.clone(),
                self.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .then(callback_ref_finance::callback_get_token_return(
                common_token,
                amount_token_1,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else if common_token == 2 {
            ext_ref_exchange::get_return(
                farm_info.pool_id_token1_reward,
                farm_info.reward_token,
                amount_token_1,
                self.token1_address.clone(),
                self.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .then(callback_ref_finance::callback_get_token_return(
                common_token,
                amount_token_2,
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        } else {
            ext_ref_exchange::get_return(
                farm_info.pool_id_token1_reward,
                farm_info.reward_token.clone(),
                amount_token_1,
                self.token1_address.clone(),
                self.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .and(ext_ref_exchange::get_return(
                farm_info.pool_id_token2_reward,
                farm_info.reward_token,
                amount_token_2,
                self.token2_address.clone(),
                self.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            ))
            .then(callback_ref_finance::callback_get_tokens_return(
                env::current_account_id(),
                0,
                Gas(10_000_000_000_000),
            ))
        }
    }

    //TODO: this function just call another one. Maybe we need to join both.
    /// Get amount of tokens available then stake it
    /// # Parameters example:
    /// farm_id_str: exchange@pool_id#farm_id
    pub(crate) fn autocompounds_liquidity_and_stake(&self, farm_id_str: String) -> Promise {
        log!("autocompounds_liquidity_and_stake");

        // send reward to contract caller
        self.send_reward_to_sentry(farm_id_str, env::predecessor_account_id())
    }
    pub(crate) fn send_reward_to_sentry(
        &self,
        farm_id_str: String,
        sentry_acc_id: AccountId,
    ) -> Promise {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let farm_info = self.get_farm_info(&farm_id);

        ext_reward_token::storage_balance_of(
            sentry_acc_id.clone(),
            farm_info.reward_token.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_ref_finance::callback_post_sentry(
            farm_id_str,
            sentry_acc_id,
            farm_info.reward_token,
            env::current_account_id(),
            0,
            Gas(240_000_000_000_000),
        ))
    }
}

// Versioned Farmer, used for lazy upgrade.
// Which means this structure would upgrade automatically when used.
// To achieve that, each time the new version comes in,
// each function of this enum should be carefully re-code!
// #[derive(BorshSerialize, BorshDeserialize)]
// pub enum VersionedCompounder {
//     V101(AutoCompounder),
// }

// impl VersionedCompounder {
//     #[allow(dead_code)]
//     pub(crate) fn new(
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
