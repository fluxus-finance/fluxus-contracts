use crate::*;

const MAX_SLIPPAGE_ALLOWED: u128 = 20;

#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum JumboAutoCompounderState {
    Running,
    Ended,
    Cleared,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JumboStratFarmInfo {
    /// State is used to update the contract to a Paused/Running state
    pub state: JumboAutoCompounderState,

    /// Used to keep track of the current stage of the auto-compound cycle
    pub cycle_stage: JumboAutoCompounderCycle,

    /// Slippage applied to swaps, range from 0 to 100.
    /// Defaults to 5%. The value will be computed as 100 - slippage
    pub slippage: u128,

    /// Used to keep track of the rewards received from the farm during auto-compound cycle
    pub last_reward_amount: u128,

    pub current_shares_to_stake: u128,

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

impl JumboStratFarmInfo {
    pub(crate) fn next_cycle(&mut self) {
        match self.cycle_stage {
            JumboAutoCompounderCycle::ClaimReward => {
                self.cycle_stage = JumboAutoCompounderCycle::Withdrawal
            }
            JumboAutoCompounderCycle::Withdrawal => {
                self.cycle_stage = JumboAutoCompounderCycle::SwapToken1
            }
            JumboAutoCompounderCycle::SwapToken1 => {
                self.cycle_stage = JumboAutoCompounderCycle::SwapToken2
            }
            JumboAutoCompounderCycle::SwapToken2 => {
                self.cycle_stage = JumboAutoCompounderCycle::Stake
            }
            JumboAutoCompounderCycle::Stake => {
                self.cycle_stage = JumboAutoCompounderCycle::ClaimReward
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
            self.state = JumboAutoCompounderState::Ended;
            log!("Slippage too high. State was updated to Ended");
        }
    }
}

// #[derive(BorshSerialize, BorshDeserialize)]
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JumboAutoCompounder {
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
    pub farms: Vec<JumboStratFarmInfo>,

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
pub enum JumboAutoCompounderCycle {
    ClaimReward,
    Withdrawal,
    SwapToken1,
    SwapToken2,
    Stake,
}

impl From<&JumboAutoCompounderCycle> for String {
    fn from(cycle: &JumboAutoCompounderCycle) -> Self {
        match *cycle {
            JumboAutoCompounderCycle::ClaimReward => String::from("Reward"),
            JumboAutoCompounderCycle::Withdrawal => String::from("Withdrawal"),
            JumboAutoCompounderCycle::SwapToken1 => String::from("SwapToken1"),
            JumboAutoCompounderCycle::SwapToken2 => String::from("SwapToken2"),
            JumboAutoCompounderCycle::Stake => String::from("Stake"),
        }
    }
}

/// Auto-compounder internal methods
impl JumboAutoCompounder {
    /// Initialize a new jumbo's compounder.
    /// Args:
    /// strategy_fee: 5,
    /// strat_creator: { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    /// sentry_fee: 10,
    /// exchange_contract_id: exchange_contract.testnet,
    /// farm_contract_id: farm_contract.testnet,
    /// token1_address: token2.testnet,
    /// token2_address: token1.testnet,
    /// pool_id: 17,
    /// seed_id: exchange@seed_id,
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
    /// Args:
    /// reward_amount: 100000000,
    pub(crate) fn compute_fees(&mut self, reward_amount: u128) -> (u128, u128, u128, u128) {
        // apply fees to reward amount
        let percent = Percentage::from(self.admin_fees.strategy_fee);
        let all_fees_amount = percent.apply_to(reward_amount);
        // let percent = Percentage::from(treasury_fee_percentage);
        // let protocol_amount = percent.apply_to(self.last_reward_amount);

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

    /// Return a jumbo's farm information.
    /// Args:
    /// farm_id: 1,
    pub fn get_jumbo_farm_info(&self, farm_id: &str) -> JumboStratFarmInfo {
        for farm in self.farms.iter() {
            if farm.id == farm_id {
                return farm.clone();
            }
        }

        panic!("Farm does not exist")
    }

    /// Return a jumbo's mutable farm information.
    /// Args:
    /// farm_id: 1,
    pub fn get_mut_jumbo_farm_info(&mut self, farm_id: String) -> &mut JumboStratFarmInfo {
        for farm in self.farms.iter_mut() {
            if farm.id == farm_id {
                return farm;
            }
        }

        panic!("Farm does not exist")
    }

    pub fn stake(
        &self,
        token_id: String,
        seed_id: String,
        account_id: &AccountId,
        shares: u128,
    ) -> Promise {
        // decide which strategies
        ext_jumbo_exchange::mft_transfer_call(
            token_id,
            self.farm_contract_id.clone(),
            U128(shares),
            None,
            "".to_string(),
            self.exchange_contract_id.clone(),
            1,
            Gas(80_000_000_000_000),
        )
        // substitute for a generic callback, with a match for next step
        .then(callback_jumbo_exchange::callback_jumbo_stake_result(
            seed_id,
            account_id.clone(),
            shares,
            env::current_account_id(),
            0,
            Gas(10_000_000_000_000),
        ))
    }

    /// Withdraw user lps and send it to the contract.
    pub fn unstake(
        &self,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise {
        // Unstake shares/lps
        ext_jumbo_exchange::get_pool_shares(
            self.pool_id,
            env::current_account_id(),
            self.exchange_contract_id.clone(),
            0,
            Gas(20_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_get_pool_shares(
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

    pub fn claim_reward(&mut self, farm_id_str: String) -> Promise {
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("claim_reward");

        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());

        ext_jumbo_farming::list_farms_by_seed(
            seed_id,
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_list_farms_by_seed(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }

    /// Step 2
    /// Function to claim the reward from the farm contract
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn withdraw_of_reward(
        &mut self,
        farm_id_str: String,
        treasury_current_amount: u128,
    ) -> Promise {
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("withdraw_of_reward");

        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let farm_info = self.get_jumbo_farm_info(&farm_id);

        // contract_id does not exist on sentries
        if !self
            .admin_fees
            .sentries
            .contains_key(&env::current_account_id())
        {
            let amount_to_withdraw = farm_info.last_reward_amount;
            ext_jumbo_farming::withdraw_reward(
                farm_info.reward_token,
                Some(U128(amount_to_withdraw)),
                self.farm_contract_id.clone(),
                1,
                Gas(100_000_000_000_000),
            )
            .then(callback_jumbo_exchange::callback_jumbo_post_withdraw(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(180_000_000_000_000),
            ))
        } else {
            // the withdraw succeeded but not the transfer
            ext_reward_token::ft_transfer_call(
                self.exchange_contract_id.clone(),
                U128(farm_info.last_reward_amount + treasury_current_amount), //Amount after withdraw the rewards
                "".to_string(),
                farm_info.reward_token,
                1,
                Gas(120_000_000_000_000),
            )
            .then(callback_jumbo_exchange::callback_jumbo_post_ft_transfer(
                farm_id_str,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }
    }

    /// Step 3
    /// Transfer reward token to ref-exchange then swap the amount the contract has in the exchange
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn autocompounds_swap(&mut self, farm_id_str: String, treasure: AccountFee) -> Promise {
        // TODO: take string as ref
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("autocompounds_swap");

        let treasury_acc: AccountId = treasure.account_id;
        let treasury_curr_amount: u128 = treasure.current_amount;

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.clone());
        let farm_info = self.get_jumbo_farm_info(&farm_id);

        let reward_amount = farm_info.last_reward_amount;

        let amount_in = U128(reward_amount / 2);

        if treasury_curr_amount > 0 {
            // 40 TGAS
            ext_jumbo_exchange::mft_transfer(
                farm_info.reward_token.to_string(),
                treasury_acc,
                U128(treasury_curr_amount),
                Some("".to_string()),
                self.exchange_contract_id.clone(),
                1,
                Gas(20_000_000_000_000),
            )
            .then(
                callback_jumbo_exchange::callback_jumbo_post_treasury_mft_transfer(
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            );
        }

        let strat_creator_curr_amount = self.admin_fees.strat_creator.current_amount;
        if strat_creator_curr_amount > 0 {
            // 40 TGAS
            ext_reward_token::ft_transfer(
                self.admin_fees.strat_creator.account_id.clone(),
                U128(strat_creator_curr_amount),
                Some("".to_string()),
                farm_info.reward_token.clone(),
                1,
                Gas(20_000_000_000_000),
            )
            .then(
                callback_jumbo_exchange::callback_jumbo_post_creator_ft_transfer(
                    seed_id,
                    env::current_account_id(),
                    0,
                    Gas(20_000_000_000_000),
                ),
            );
        }

        // 130 TGAS
        ext_jumbo_exchange::get_return(
            farm_info.pool_id_token1_reward,
            farm_info.reward_token,
            amount_in,
            self.token1_address.clone(),
            self.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_get_token1_return(
            farm_id_str,
            amount_in,
            env::current_account_id(),
            0,
            Gas(120_000_000_000_000),
        ))
    }

    /// Step 4
    /// Transfer reward token to ref-exchange then swap the amount the contract has in the exchange
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn autocompounds_swap_second_token(&mut self, farm_id_str: String) -> Promise {
        // TODO: take string as ref
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("autocompounds_swap_second_token");

        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.clone());
        let farm_info = self.get_jumbo_farm_info(&farm_id);

        let reward_amount_left = farm_info.last_reward_amount;

        // 130 TGAS
        ext_jumbo_exchange::get_return(
            farm_info.pool_id_token2_reward,
            farm_info.reward_token,
            U128(reward_amount_left),
            self.token2_address.clone(),
            self.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_get_token2_return(
            farm_id_str,
            U128(reward_amount_left),
            env::current_account_id(),
            0,
            Gas(120_000_000_000_000),
        ))
    }

    /// Step 5
    /// Get amount of tokens available then stake it
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id

    pub fn autocompounds_liquidity_and_stake(&mut self, farm_id_str: String) -> Promise {
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("autocompounds_liquidity_and_stake");

        // send reward to contract caller
        self.jumbo_send_reward_to_sentry(farm_id_str, env::predecessor_account_id())
    }

    pub fn jumbo_send_reward_to_sentry(
        &self,
        farm_id_str: String,
        sentry_acc_id: AccountId,
    ) -> Promise {
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let farm_info = self.get_jumbo_farm_info(&farm_id);

        ext_reward_token::storage_balance_of(
            sentry_acc_id.clone(),
            farm_info.reward_token.clone(),
            0,
            Gas(5_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_post_sentry(
            farm_id_str,
            sentry_acc_id,
            farm_info.reward_token,
            env::current_account_id(),
            0,
            Gas(280_000_000_000_000),
        ))
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_jumbo_stake_result(
        &mut self,
        #[callback_result] transfer_result: Result<U128, PromiseError>,
        seed_id: String,
        account_id: AccountId,
        shares: u128,
    ) -> String {
        if let Ok(amount) = transfer_result {
            assert_eq!(amount.0, 0, "ERR_STAKE_FAILED");
        } else {
            panic!("ERR_STAKE_FAILED");
        }
        //Total fft_share
        let total_fft = self.total_supply_by_pool_id(seed_id.clone());
        log!("total fft is = {}", total_fft);

        let fft_share_id = self.get_fft_share_id_from_seed(seed_id.clone());

        // let data_mut = self.data_mut();
        //Total seed_id
        let total_seed = self.seed_total_amount(&seed_id);

        let new_seed_amount = total_seed + shares;

        log!("seed {}", seed_id);

        log!("total seed: {} shares added: {}", total_seed, shares);

        log!("new seed amount: {}", new_seed_amount);

        let fft_share_amount = if total_fft == 0 {
            shares
        } else {
            (U256::from(shares) * U256::from(total_fft) / U256::from(total_seed)).as_u128()
        };

        log!(
            "{} {} will be minted for {}",
            fft_share_amount,
            fft_share_id,
            account_id.to_string()
        );
        self.mft_mint(fft_share_id, fft_share_amount, account_id.to_string());

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &new_seed_amount);

        format!(
            "The {} added {} to {}",
            account_id, fft_share_amount, seed_id
        )
    }

    #[private]
    pub fn callback_jumbo_get_pool_shares(
        &self,
        #[callback_result] shares_result: Result<U128, PromiseError>,
        token_id: String,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise {
        assert!(shares_result.is_ok(), "ERR");

        let compounder = self.get_strat(&seed_id).get_jumbo();

        let shares_on_exchange: u128 = shares_result.unwrap().into();

        if shares_on_exchange >= withdraw_amount {
            ext_jumbo_exchange::mft_transfer(
                token_id.clone(),
                receiver_id.clone(),
                U128(withdraw_amount),
                Some("".to_string()),
                compounder.exchange_contract_id.clone(),
                1,
                Gas(30_000_000_000_000),
            )
            .then(callback_jumbo_exchange::callback_jumbo_withdraw_shares(
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        } else {
            let amount = withdraw_amount - shares_on_exchange;

            // withdraw missing amount from farm
            ext_jumbo_farming::withdraw_seed(
                compounder.seed_id,
                U128(amount),
                compounder.farm_contract_id.clone(),
                1,
                Gas(180_000_000_000_000),
            )
            // transfer the total amount required
            .then(ext_jumbo_exchange::mft_transfer(
                token_id.clone(),
                receiver_id.clone(),
                U128(withdraw_amount),
                Some("".to_string()),
                compounder.exchange_contract_id.clone(),
                1,
                Gas(30_000_000_000_000),
            ))
            .then(callback_jumbo_exchange::callback_jumbo_withdraw_shares(
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
                env::current_account_id(),
                0,
                Gas(20_000_000_000_000),
            ))
        }
    }

    #[private]
    pub fn callback_jumbo_withdraw_shares(
        &mut self,
        #[callback_result] mft_transfer_result: Result<(), PromiseError>,
        seed_id: String,
        account_id: AccountId,
        amount: Balance,
        fft_shares: Balance,
    ) {
        match mft_transfer_result {
            Ok(_) => log!("Nice!"),
            Err(err) => {
                panic!("err")
            }
        }

        let data = self.data_mut();
        let total_seed = data.seed_id_amount.get(&seed_id).unwrap_or_default();

        self.data_mut()
            .seed_id_amount
            .insert(&seed_id, &(total_seed - amount));

        let fft_share_id = self
            .data()
            .fft_share_by_seed_id
            .get(&seed_id)
            .unwrap()
            .clone();

        self.mft_burn(fft_share_id, fft_shares, account_id.to_string());
    }
}
