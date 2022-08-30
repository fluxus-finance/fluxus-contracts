use crate::*;

#[near_bindgen]
impl Contract {
    /// Function that return the user`s near storage.
    /// WARN: DEPRECATED
    pub fn get_user_storage_state(&self, account_id: AccountId) -> Option<RefStorageState> {
        self.is_caller(account_id.clone());
        let acc = self.internal_get_account(&account_id);
        if let Some(account) = acc {
            Some(RefStorageState {
                deposit: U128(account.near_amount),
                usage: U128(account.storage_usage()),
            })
        } else {
            None
        }
    }

    /// Function to return the user's deposit in the auto_compounder contract.
    /// WARN: DEPRECATED
    pub fn get_deposits(&self, account_id: AccountId) -> HashMap<AccountId, U128> {
        let wrapped_account = self.internal_get_account(&account_id);
        if let Some(account) = wrapped_account {
            account
                .get_tokens()
                .iter()
                .map(|token| (token.clone(), U128(account.get_balance(token).unwrap())))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Returns the state of the contract, such as Running, Paused
    pub fn get_contract_state(&self) -> String {
        format!("{} is {}", env::current_account_id(), self.data().state)
    }

    /// Return the whitelisted tokens.
    /// WARN: DEPRECATED
    pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
        self.data().whitelisted_tokens.to_vec()
    }

    /// Returns the minimum value accepted for given token_id
    pub fn get_seed_min_deposit(self, seed_id: String) -> U128 {
        // TODO: stable version
        let compounder = self.get_strat(&seed_id).get_compounder();
        compounder.seed_min_deposit
    }

    /// Returns the total amount of near that was deposited
    /// WARN: DEPRECATED
    pub fn user_total_near_deposited(&self, account_id: AccountId) -> Option<String> {
        self.data()
            .users_total_near_deposited
            .get(&account_id)
            .map(|x| x.to_string())
    }

    /// Returns all token ids
    // TODO: refactor, should be get seeds
    pub fn get_allowed_tokens(&self) -> Vec<String> {
        let mut seeds: Vec<String> = Vec::new();

        for (token_id, _) in self.data().strategies.iter() {
            seeds.push(token_id.clone());
        }

        seeds
    }

    pub fn get_running_farm_ids(&self) -> Vec<String> {
        let mut running_strategies: Vec<String> = Vec::new();

        // TODO: stable versions
        for (_, strat) in self.data().strategies.iter() {
            let compounder = strat.get_compounder_ref();
            for farm in compounder.farms.iter() {
                if farm.state == AutoCompounderState::Running {
                    let farm_id = format!("{}#{}", compounder.seed_id, farm.id);
                    running_strategies.push(farm_id);
                }
            }
        }

        running_strategies
    }

    /// Return all Strategies
    pub fn get_strategies(self) -> Vec<AutoCompounderInfo> {
        let mut info: Vec<AutoCompounderInfo> = Vec::new();

        // TODO: stable version
        for (seed_id, strat) in self.data().strategies.clone() {
            let compounder = strat.get_compounder();
            let mut seed_info = AutoCompounderInfo {
                seed_id,
                is_active: false,
                reward_tokens: vec![],
            };
            for farm_info in compounder.farms.iter() {
                if farm_info.state == AutoCompounderState::Running {
                    seed_info.is_active = true;
                }
                seed_info
                    .reward_tokens
                    .push(farm_info.reward_token.to_string());
            }

            info.push(seed_info)
        }

        info
    }

    pub fn get_strategies_info(&self) -> Vec<StratFarmInfo> {
        let mut info: Vec<StratFarmInfo> = Vec::new();
        // TODO: stable version
        for (_, strat) in self.data().strategies.iter() {
            for farm in strat.get_compounder_ref().farms.iter() {
                info.push(farm.clone());
            }
        }

        info
    }

    // TODO: refactor it
    // /// Running strategies to use in the bot
    // pub fn get_running_strategies(&self, farm_id_str: String) -> String {
    //     let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str);

    //     let strat = self.get_strat(token_id);
    //     let compounder = strat.get_compounder_ref();
    //     let farm_info = compounder.get_farm_info(&farm_id);

    //     farm_info.reward_token.into()
    // }

    pub fn get_strat_state(self, farm_id_str: String) -> AutoCompounderState {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        // TODO: stale version
        let compounder = self.get_strat(&seed_id).get_compounder();
        let farm_info = compounder.get_farm_info(&farm_id);

        farm_info.state
    }

    // /// Returns exchange and farm contracts
    // pub fn get_contract_info(self) -> SafeInfo {
    //     SafeInfo {
    //         exchange_address: self.exchange_acc(),
    //         farm_address: self.farm_acc(),
    //     }
    // }

    /// Only get guardians info
    pub fn get_guardians(&self) -> Vec<AccountId> {
        self.is_owner();
        self.data().guardians.to_vec()
    }

    /// TODO: refactor it
    // /// Returns current amount holden by the contract
    // pub fn get_contract_amount(self) -> U128 {
    //     let mut amount: u128 = 0;

    //     for (_, strat) in self.data().strategies.clone() {
    //         let compounder = strat.get_compounder();

    //         for (_, shares) in compounder.user_shares {
    //             amount += shares.total;
    //         }
    //     }
    //     U128(amount)
    // }

    /// TODO: refactor it
    ///Return the u128 number of strategies that we have for a specific seed_id.
    // pub fn number_of_strategies_by_seed(&self, seed_id: String) -> u128 {
    //     let num = self.data().compounders_by_seed_id.get(&seed_id);
    //     let mut result = 0_u128;
    //     if let Some(number) = num {
    //         result = (*number).len() as u128;
    //     }
    //     result
    // }

    /// Return the total number of strategies created, running or others
    pub fn number_of_strategies(&self) -> u128 {
        let mut count: u128 = 0;

        // TODO: stable versions
        for (_, strat) in self.data().strategies.iter() {
            let size = strat.get_compounder_ref().farms.len();
            count += size as u128;
        }

        count
    }

    pub fn check_fee_by_strategy(&self, seed_id: String) -> String {
        // TODO: stable version
        let compounder = self.get_strat(&seed_id).get_compounder();
        format!("{}%", compounder.admin_fees.strategy_fee)
    }

    pub fn is_strategy_active(&self, seed_id: String) -> bool {
        // TODO: stable version
        let compounder = self.get_strat(&seed_id).get_compounder();

        for farm in compounder.farms.iter() {
            if farm.state == AutoCompounderState::Running {
                return true;
            }
        }

        false
    }

    pub fn current_strat_step(&self, farm_id_str: String) -> String {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);
        // TODO: stable version
        let compounder = self.get_strat(&seed_id).get_compounder();
        let farm_info = compounder.get_farm_info(&farm_id);

        match farm_info.cycle_stage {
            AutoCompounderCycle::ClaimReward => "claim_reward".to_string(),
            AutoCompounderCycle::Withdrawal => "withdraw".to_string(),
            AutoCompounderCycle::Swap => "swap".to_string(),
            AutoCompounderCycle::Stake => "stake".to_string(),
        }
    }

    // TODO: refactor it
    // pub fn get_farm_ids_by_seed(&self, seed_id: String) -> Vec<String> {
    //     let mut strats: Vec<String> = vec![];

    //     let compounder = self.get_strat(&seed_id).get_compounder_ref().clone();

    //     for farm in compounder.farms.iter() {
    //         strats.push(format!("{}#{}", token_id, farm.id));
    //     }

    //     strats
    // }

    pub fn get_harvest_timestamp(&self, seed_id: String) -> String {
        // TODO: stable version
        let compounder = self.get_strat(&seed_id).get_compounder_ref().clone();
        compounder.harvest_timestamp.to_string()
    }

    pub fn get_strategy_kind(&self) -> String {
        match self.data().strategies.values().next() {
            Some(x) => x.kind(),
            None => "No strategies available".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AutoCompounderInfo {
    pub seed_id: String,
    pub is_active: bool,
    pub reward_tokens: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SafeInfo {
    pub exchange_address: AccountId,
    pub farm_address: AccountId,
}
