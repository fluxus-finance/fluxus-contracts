use crate::*;

#[near_bindgen]
impl Contract {
    /// Function that return the user`s near storage.
    /// WARN: DEPRECATED
    // pub fn get_user_storage_state(&self, account_id: AccountId) -> Option<RefStorageState> {
    //     self.is_caller(account_id.clone());
    //     let acc = self.internal_get_account(&account_id);
    //     if let Some(account) = acc {
    //         Some(RefStorageState {
    //             deposit: U128(account.near_amount),
    //             usage: U128(account.storage_usage()),
    //         })
    //     } else {
    //         None
    //     }
    // }

    /// Function to return the user's deposit in the auto_compounder contract.
    /// WARN: DEPRECATED
    // pub fn get_deposits(&self, account_id: AccountId) -> HashMap<AccountId, U128> {
    //     let wrapped_account = self.internal_get_account(&account_id);
    //     if let Some(account) = wrapped_account {
    //         account
    //             .get_tokens()
    //             .iter()
    //             .map(|token| (token.clone(), U128(account.get_balance(token).unwrap())))
    //             .collect()
    //     } else {
    //         HashMap::new()
    //     }
    // }

    /// Returns the state of the contract, such as Running, Paused
    pub fn get_contract_state(&self) -> String {
        format!("{} is {}", env::current_account_id(), self.data().state)
    }

    /// Return the whitelisted tokens.
    /// WARN: DEPRECATED
    // pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
    //     self.data().whitelisted_tokens.to_vec()
    // }

    /// Returns the minimum value accepted for given token_id
    /// # Parameters example:
    ///   seed_id: exchange@pool_id
    pub fn get_seed_min_deposit(self, seed_id: String) -> U128 {
        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => compounder.seed_min_deposit,
            VersionedStrategy::StableAutoCompounder(compounder) => compounder.seed_min_deposit,
            VersionedStrategy::JumboAutoCompounder(compounder) => compounder.seed_min_deposit,
            VersionedStrategy::PembrockAutoCompounder(_) => U128(0),
        }
    }

    /// Returns the total amount of near that was deposited
    /// WARN: DEPRECATED
    // pub fn user_total_near_deposited(&self, account_id: AccountId) -> Option<String> {
    //     self.data()
    //         .users_total_near_deposited
    //         .get(&account_id)
    //         .map(|x| x.to_string())
    // }

    /// Returns all seeds ids/strat_names
    // TODO: refactor, should be get seeds
    pub fn get_allowed_tokens(&self) -> Vec<String> {
        let mut seeds: Vec<String> = Vec::new();

        for (seed_id, _) in self.data().strategies.iter() {
            seeds.push(seed_id.clone());
        }

        seeds
    }

    // TODO
    // pub fn get_running_farm_ids(&self) -> Vec<String> {
    //     let mut running_strategies: Vec<String> = Vec::new();

    //     // TODO: stable versions
    //     for (_, strat) in self.data().strategies.iter() {
    //         let compounder = strat.get_compounder_ref();
    //         for farm in compounder.farms.iter() {
    //             if farm.state == AutoCompounderState::Running {
    //                 let farm_id = format!("{}#{}", compounder.seed_id, farm.id);
    //                 running_strategies.push(farm_id);
    //             }
    //         }
    //     }

    //     running_strategies
    // }

    /// Return all Strategies
    pub fn get_strategies(self) -> Vec<AutoCompounderInfo> {
        let mut info: Vec<AutoCompounderInfo> = Vec::new();

        // TODO: stable version
        for (seed_id, strat) in self.data().strategies.clone() {
            match strat {
                VersionedStrategy::AutoCompounder(compounder) => {
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
                VersionedStrategy::StableAutoCompounder(compounder) => {
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
                VersionedStrategy::JumboAutoCompounder(compounder) => {
                    let mut seed_info = AutoCompounderInfo {
                        seed_id,
                        is_active: false,
                        reward_tokens: vec![],
                    };
                    for farm_info in compounder.farms.iter() {
                        if farm_info.state == JumboAutoCompounderState::Running {
                            seed_info.is_active = true;
                        }
                        seed_info
                            .reward_tokens
                            .push(farm_info.reward_token.to_string());
                    }

                    info.push(seed_info)
                }
                VersionedStrategy::PembrockAutoCompounder(compounder) => {
                    let mut seed_info = AutoCompounderInfo {
                        seed_id,
                        is_active: false,
                        reward_tokens: vec![],
                    };

                    if compounder.state == PembAutoCompounderState::Running {
                        seed_info.is_active = true;
                    }
                    seed_info
                        .reward_tokens
                        .push(compounder.reward_token.to_string());

                    info.push(seed_info)
                }
            };
        }

        info
    }

    /// Return the registered strategies for ref finance.
    pub fn get_strategies_info_for_ref_finance(&self) -> Vec<StratFarmInfo> {
        let mut info: Vec<StratFarmInfo> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"AUTO_COMPOUNDER" {
                let compounder = strat.get_compounder_ref();
                for farm in compounder.farms.iter() {
                    info.push(farm.clone())
                }
            }
        }
        info
    }

    /// Return the registered stable strategies for ref finance.
    pub fn get_strategies_info_for_stable_ref_finance(&self) -> Vec<StableStratFarmInfo> {
        let mut info: Vec<StableStratFarmInfo> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"STABLE_AUTO_COMPOUNDER" {
                let compounder = strat.get_stable_compounder_ref();
                for farm in compounder.farms.iter() {
                    info.push(farm.clone())
                }
            }
        }

        info
    }

    /// Return the registered strategies for Jumbo.
    pub fn get_strategies_info_for_jumbo(&self) -> Vec<JumboStratFarmInfo> {
        let mut info: Vec<JumboStratFarmInfo> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"JUMBO_AUTO_COMPOUNDER" {
                for farm in strat.get_jumbo_ref().farms.iter() {
                    info.push(farm.clone());
                }
            }
        }

        info
    }

    /// Return the registered strategies for Pembrock.
    pub fn get_strategies_info_for_pembrock(&self) -> Vec<PembrockAutoCompounder> {
        let mut info: Vec<PembrockAutoCompounder> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"PEMBROCK_AUTO_COMPOUNDER" {
                info.push(strat.get_pemb_ref().clone());
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

    /// Return some ref finance strategy structure.
    /// # Parameter example:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn get_strategy_for_ref_finance(self, farm_id_str: String) -> AutoCompounderState {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                let farm_info = compounder.get_farm_info(&farm_id);

                farm_info.state
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                let farm_info = compounder.get_farm_info(&farm_id);

                farm_info.state
            }
            _ => unimplemented!(),
        }
    }

    /// Return some Jumbo strategy structure.
    /// # Parameter example:
    ///   farm_id_str: exchange@pool_id#farm_id
    pub fn get_strategy_for_jumbo(self, farm_id_str: String) -> JumboAutoCompounderState {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder = self.get_strat(&seed_id).get_jumbo();
        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

        farm_info.state
    }

    /// Return some Pembrock strategy structure.
    /// # Parameter example:
    ///   farm_id_str: pembrock@token_name
    pub fn get_strategy_for_pembrock(self, strat_name: String) -> PembAutoCompounderState {
        let compounder = self.get_strat(&strat_name).get_pemb();

        compounder.state
    }

    /// Only get guardians info
    pub fn get_guardians(&self) -> Vec<AccountId> {
        self.data().guardians.to_vec()
    }

    /// Returns current amount holden by the contract
    pub fn get_contract_amount(self) -> U128 {
        let mut amount: u128 = 0;

        for (seed_id, _) in self.data().strategies.iter() {
            amount += self.seed_total_amount(seed_id);
        }
        U128(amount)
    }

    ///Return the u128 number of strategies that we have for a specific seed_id.
    /// # Parameter example:
    ///   seed_id: exchange@pool_id
    pub fn number_of_strategies_by_seed(&self, seed_id: String) -> String {
        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => compounder.farms.len().to_string(),
            VersionedStrategy::StableAutoCompounder(compounder) => {
                compounder.farms.len().to_string()
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                compounder.farms.len().to_string()
            }
            VersionedStrategy::PembrockAutoCompounder(_) => 1.to_string(),
        }
    }

    /// Return the total number of strategies created, running or others
    pub fn number_of_strategies(&self) -> U128 {
        let mut count: u128 = 0;

        for (_, strat) in self.data().strategies.iter() {
            count += match strat {
                VersionedStrategy::AutoCompounder(compounder) => compounder.farms.len() as u128,
                VersionedStrategy::StableAutoCompounder(compounder) => {
                    compounder.farms.len() as u128
                }
                VersionedStrategy::JumboAutoCompounder(compounder) => {
                    compounder.farms.len() as u128
                }
                VersionedStrategy::PembrockAutoCompounder(_) => 1,
            }
        }

        U128(count)
    }

    /// Return the fee for some specific strategy.
    /// # Parameter example:
    ///   seed_id: exchange@pool_id
    pub fn check_fee_by_strategy(&self, seed_id: String) -> String {
        // let compounder = self.get_strat(&seed_id).get_compounder();

        let strat = self.get_strat(&seed_id);

        let fee = match strat {
            VersionedStrategy::AutoCompounder(compounder) => compounder.admin_fees.strategy_fee,
            VersionedStrategy::StableAutoCompounder(compounder) => {
                compounder.admin_fees.strategy_fee
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                compounder.admin_fees.strategy_fee
            }
            VersionedStrategy::PembrockAutoCompounder(compounder) => {
                compounder.admin_fees.strategy_fee
            }
        };

        format!("{}%", fee)
    }

    /// Return true if the strategy is active.
    /// # Parameter example:
    ///   seed_id: exchange@pool_id
    pub fn is_strategy_active(&self, seed_id: String) -> bool {
        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                for farm in compounder.farms.iter() {
                    if farm.state == AutoCompounderState::Running {
                        return true;
                    }
                }
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                for farm in compounder.farms.iter() {
                    if farm.state == AutoCompounderState::Running {
                        return true;
                    }
                }
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                for farm in compounder.farms.iter() {
                    if farm.state == JumboAutoCompounderState::Running {
                        return true;
                    }
                }
            }
            VersionedStrategy::PembrockAutoCompounder(compounder) => {
                if compounder.state == PembAutoCompounderState::Running {
                    return true;
                }
            }
        }

        false
    }

    /// Return the current harvest step for some strategy.
    /// # Parameter example:
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   strat_name: "" or pembrock@token_name
    pub fn current_strat_step(&self, farm_id_str: String, strat_name: String) -> String {
        match strat_name.is_empty() {
            false => String::from(&self.get_strat(&strat_name).get_pemb().cycle_stage),
            true => {
                let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);
                let strat = self.get_strat(&seed_id);

                match strat {
                    VersionedStrategy::AutoCompounder(compounder) => {
                        let farm_info = compounder.get_farm_info(&farm_id);

                        String::from(&farm_info.cycle_stage)
                    }
                    VersionedStrategy::StableAutoCompounder(compounder) => {
                        let farm_info = compounder.get_farm_info(&farm_id);

                        String::from(&farm_info.cycle_stage)
                    }
                    VersionedStrategy::JumboAutoCompounder(compounder) => {
                        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

                        String::from(&farm_info.cycle_stage)
                    }
                    _ => unimplemented!(),
                }
            }
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

    /// Return the harvest time_stamp for some strategy.
    /// # Parameter example:
    ///   seed_id: exchange@pool_id
    pub fn get_harvest_timestamp(&self, seed_id: String) -> String {
        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
            VersionedStrategy::PembrockAutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
        }
    }

    /// Return the strategy kind.
    /// # Parameter example:
    ///   seed_id: exchange@pool_id
    pub fn get_strategy_kind(&self, seed_id: String) -> String {
        self.get_strat(&seed_id).kind()
    }

    pub fn get_harvest_info(&self) -> Vec<StrategyInfo> {
        let mut strat_farms: Vec<StrategyInfo> = Vec::new();

        for (seed_id, strat) in self.data().strategies.iter() {
            match strat {
                VersionedStrategy::AutoCompounder(compounder) => {
                    for strat_farm in compounder.farms.iter() {
                        strat_farms.push(StrategyInfo {
                            strat_kind: strat.kind(),
                            seed_id: Some(seed_id.to_string()),
                            strat_name: None,
                            farm_id_str: Some(format!("{}#{}", seed_id, strat_farm.id)),
                            is_active: strat_farm.state == AutoCompounderState::Running,
                            reward_tokens: strat_farm.reward_token.to_string(),
                            fees: Some(compounder.admin_fees.clone()),
                        })
                    }
                }
                VersionedStrategy::StableAutoCompounder(stable) => {
                    for strat_farm in stable.farms.iter() {
                        strat_farms.push(StrategyInfo {
                            strat_kind: strat.kind(),
                            seed_id: Some(seed_id.to_string()),
                            strat_name: None,
                            farm_id_str: Some(format!("{}#{}", seed_id, strat_farm.id)),
                            is_active: strat_farm.state == AutoCompounderState::Running,
                            reward_tokens: strat_farm.reward_token.to_string(),
                            fees: Some(stable.admin_fees.clone()),
                        })
                    }
                }
                VersionedStrategy::JumboAutoCompounder(jumbo) => {
                    for strat_farm in jumbo.farms.iter() {
                        strat_farms.push(StrategyInfo {
                            strat_kind: strat.kind(),
                            seed_id: Some(seed_id.to_string()),
                            strat_name: None,
                            farm_id_str: Some(format!("{}#{}", seed_id, strat_farm.id)),
                            is_active: strat_farm.state == JumboAutoCompounderState::Running,
                            reward_tokens: strat_farm.reward_token.to_string(),
                            fees: Some(jumbo.admin_fees.clone()),
                        })
                    }
                }
                VersionedStrategy::PembrockAutoCompounder(pembrock) => {
                    strat_farms.push(StrategyInfo {
                        strat_kind: strat.kind(),
                        seed_id: None,
                        strat_name: Some(seed_id.to_string()),
                        farm_id_str: None,
                        is_active: pembrock.state == PembAutoCompounderState::Running,
                        reward_tokens: pembrock.reward_token.to_string(),
                        fees: Some(pembrock.admin_fees.clone()),
                    })
                }
            }
        }

        strat_farms
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StrategyInfo {
    pub strat_kind: String,
    pub seed_id: Option<String>,
    pub strat_name: Option<String>,
    pub farm_id_str: Option<String>,
    pub is_active: bool,
    pub reward_tokens: String,
    pub fees: Option<AdminFees>,
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
