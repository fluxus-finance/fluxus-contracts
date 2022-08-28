use crate::*;

const MAX_SLIPPAGE_ALLOWED: u128 = 20;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StableStratFarmInfo {
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

    // TODO: need a variable to define which position the add_stable_liquidity will go
    // can have [usdt, usdc] [usdt, usdc, dai] but only one position will be used
    /// Address of the token used by pool
    pub token_address: AccountId,

    /// Pool used to swap the reward received by the token used to add liquidity
    pub pool_id_token_reward: u64,

    /// Vector position of the token used to add liquidity to the pool
    /// [token1, token2, token_to_add] -> indexes: [0, 1, 2] -> use 2
    pub token_position: u64,

    /// Address of the reward token given by the farm
    pub reward_token: AccountId,

    /// Store balance of available of tokens
    /// obs: would be better to have it in as a LookupMap, but Serialize and Clone is not available for it
    pub available_balance: Vec<Balance>,

    /// Farm used to auto-compound
    pub id: String,
}

impl StableStratFarmInfo {
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
pub struct StableAutoCompounder {
    /// Fees struct to be distribute at each round of compound
    pub admin_fees: AdminFees,

    // Contract address of the exchange used
    pub exchange_contract_id: AccountId,

    // Contract address of the farm used
    pub farm_contract_id: AccountId,

    /// Pool used to add liquidity and farming
    pub pool_id: u64,

    /// Min LP amount accepted by the farm for stake
    pub seed_min_deposit: U128,

    /// Format expected by the farm to claim and withdraw rewards
    pub seed_id: String,

    /// Store all farms that were used to compound by some token_id
    pub farms: Vec<StableStratFarmInfo>,

    /// Latest harvest timestamp
    pub harvest_timestamp: u64,
}

/// Auto-compounder internal methods
impl StableAutoCompounder {
    pub(crate) fn new(
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        pool_id: u64,
        seed_id: String,
        seed_min_deposit: U128,
    ) -> Self {
        let admin_fee = AdminFees::new(strat_creator, sentry_fee, strategy_fee);

        Self {
            admin_fees: admin_fee,
            exchange_contract_id,
            farm_contract_id,
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

    pub fn get_farm_info(&self, farm_id: &str) -> StableStratFarmInfo {
        for farm in self.farms.iter() {
            if farm.id == farm_id {
                return farm.clone();
            }
        }

        panic!("Farm does not exist")
    }

    pub fn get_mut_farm_info(&mut self, farm_id: &String) -> &mut StableStratFarmInfo {
        for farm in self.farms.iter_mut() {
            if farm.id == *farm_id {
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

    pub fn stake_on_ref_finance(&self) {}
    pub fn stake_on_jumbo(&self) {}

    pub fn internal_stake_resolver(
        &self,
        exchange_account_id: SupportedExchanges,
        token_id: String,
        account_id: &AccountId,
        shares: u128,
    ) {
        match exchange_account_id {
            SupportedExchanges::RefFinance => self.stake_on_ref_finance(),
            SupportedExchanges::Jumbo => self.stake_on_jumbo(),
        }
    }

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

    pub fn unstake(
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
        .then(
            callback_stable_ref_finance::stable_callback_get_pool_shares(
                token_id,
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
                env::current_account_id(),
                0,
                Gas(260_000_000_000_000),
            ),
        )
    }

    /// Step 1
    /// Function to claim the reward from the farm contract
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn claim_reward(&self, farm_id_str: String) -> Promise {
        log!("claim_reward");
        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());

        ext_ref_farming::list_seed_farms(
            seed_id,
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(
            callback_stable_ref_finance::stable_callback_list_farms_by_seed(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(100_000_000_000_000),
            ),
        )
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn withdraw_of_reward(
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
            .then(callback_stable_ref_finance::stable_callback_post_withdraw(
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
            .then(
                callback_stable_ref_finance::stable_callback_post_ft_transfer(
                    farm_id_str,
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            )
        }
    }

    /// Step 3
    /// Transfer reward token to ref-exchange then swap the amount the contract has in the exchange
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn autocompounds_swap(
        &mut self,
        farm_id_str: String,
        treasure: AccountFee,
    ) -> PromiseOrValue<u128> {
        log!("autocompounds_swap");

        let treasury_acc: AccountId = treasure.account_id;
        let treasury_curr_amount: u128 = treasure.current_amount;

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());

        let exchange_id = self.exchange_contract_id.clone();
        let strat_creator_curr_amount = self.admin_fees.strat_creator.current_amount;
        let strat_creator_account_id = self.admin_fees.strat_creator.account_id.clone();

        let farm_info_mut = self.get_mut_farm_info(&farm_id);
        let token_id = farm_info_mut.token_address.clone();

        let reward_amount = farm_info_mut.last_reward_amount;

        if treasury_curr_amount > 0 {
            ext_ref_exchange::mft_transfer(
                farm_info_mut.reward_token.to_string(),
                treasury_acc,
                U128(treasury_curr_amount),
                Some("".to_string()),
                exchange_id,
                1,
                Gas(20_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_post_treasury_mft_transfer(
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            );
        }

        if strat_creator_curr_amount > 0 {
            ext_reward_token::ft_transfer(
                strat_creator_account_id,
                U128(strat_creator_curr_amount),
                Some("".to_string()),
                farm_info_mut.reward_token.clone(),
                1,
                Gas(20_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_post_creator_ft_transfer(
                    seed_id,
                    env::current_account_id(),
                    0,
                    Gas(10_000_000_000_000),
                ),
            );
        }

        if token_id == farm_info_mut.reward_token {
            // No need to swap tokens

            // no more rewards to spend
            farm_info_mut.last_reward_amount = 0;

            farm_info_mut.available_balance[farm_info_mut.token_position as usize] = reward_amount;

            farm_info_mut.next_cycle();
            return PromiseOrValue::Value(0u128);
        }

        PromiseOrValue::Promise(
            ext_ref_exchange::get_return(
                farm_info_mut.pool_id_token_reward,
                farm_info_mut.reward_token.clone(),
                U128(reward_amount),
                token_id,
                self.exchange_contract_id.clone(),
                0,
                Gas(10_000_000_000_000),
            )
            .then(
                callback_stable_ref_finance::stable_callback_get_token_return(
                    farm_id_str,
                    U128(reward_amount),
                    env::current_account_id(),
                    0,
                    Gas(120_000_000_000_000),
                ),
            ),
        )
    }

    pub fn autocompounds_liquidity_and_stake(&self, farm_id_str: String) -> Promise {
        log!("autocompounds_liquidity_and_stake");

        // send reward to contract caller
        self.send_reward_to_sentry(farm_id_str, env::predecessor_account_id())
    }

    pub fn send_reward_to_sentry(&self, farm_id_str: String, sentry_acc_id: AccountId) -> Promise {
        let (_, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let farm_info = self.get_farm_info(&farm_id);

        ext_reward_token::storage_balance_of(
            sentry_acc_id.clone(),
            farm_info.reward_token.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_stable_ref_finance::stable_callback_post_sentry(
            farm_id_str,
            sentry_acc_id,
            farm_info.reward_token,
            env::current_account_id(),
            0,
            Gas(250_000_000_000_000),
        ))
    }
}

pub enum SupportedExchanges {
    RefFinance,
    Jumbo,
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
