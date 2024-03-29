use crate::*;
//const pembrock_token = "token.pembrock.testnet";

#[near_bindgen]
impl Contract {
    // TODO: thi&s method should register in the correct pool/farm
    pub fn create_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        self.is_owner_or_guardians();

        let token_id = wrap_mft_token_id(&pool_id.to_string());

        let seed_id: String = format!("{}@{}", exchange_contract_id, pool_id);

        // TODO: update to seed
        return if self.data().strategies.contains_key(&seed_id) {
            format!("VersionedStrategy for {} already exist", token_id)
        } else {
            // OK
            let uxu_share_id = self.new_fft_share(seed_id.clone());

            let data_mut = self.data_mut();

            let strat: VersionedStrategy = VersionedStrategy::AutoCompounder(AutoCompounder::new(
                strategy_fee,
                strat_creator,
                sentry_fee,
                exchange_contract_id,
                farm_contract_id,
                token1_address,
                token2_address,
                pool_id,
                seed_id.clone(),
                seed_min_deposit,
            ));

            if let Some(share_id) = uxu_share_id {
                log!("Registering {} to {}", share_id, seed_id);
                //Registering id for the specific seed
                data_mut
                    .fft_share_by_seed_id
                    .insert(seed_id.clone(), share_id.clone());

                //Registering id in the users balance map
                let temp = LookupMap::new(StorageKey::Strategy {
                    fft_share_id: share_id.clone(),
                });

                data_mut
                    .users_balance_by_fft_share
                    .insert(&share_id.clone(), &temp);

                //Registering total_supply
                data_mut
                    .total_supply_by_fft_share
                    .insert(&share_id, &0_u128);
            }

            // TODO: update to seed id
            data_mut.strategies.insert(seed_id, strat);

            format!("VersionedStrategy for {} created successfully", token_id)
        };
    }

    pub fn add_farm_to_strategy(
        &mut self,
        seed_id: String,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
    ) -> String {
        self.is_owner_or_guardians();
        let compounder = self.get_strat_mut(&seed_id).get_compounder_mut();

        for farm in compounder.farms.clone() {
            if farm.id == farm_id {
                return format!("Farm with index {} for {} already exist", farm_id, seed_id);
            }
        }

        let farm_info: StratFarmInfo = StratFarmInfo {
            state: AutoCompounderState::Running,
            cycle_stage: AutoCompounderCycle::ClaimReward,
            slippage: 99u128,
            last_reward_amount: 0u128,
            last_fee_amount: 0u128,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            available_balance: vec![0u128, 0u128],
            id: farm_id.clone(),
        };

        compounder.farms.push(farm_info);

        format!(
            "Farm with index {} for {} created successfully",
            farm_id, seed_id
        )
    }

    pub fn create_stable_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        // TODO: is stable available on jumbo?

        self.is_owner_or_guardians();

        let token_id = wrap_mft_token_id(&pool_id.to_string());

        let seed_id: String = format!("{}@{}", exchange_contract_id, pool_id);

        // TODO: update to seed
        return if self.data().strategies.contains_key(&seed_id) {
            format!("VersionedStrategy for {} already exist", token_id)
        } else {
            let uxu_share_id = self.new_fft_share(seed_id.clone());

            let data_mut = self.data_mut();

            let strat: VersionedStrategy =
                VersionedStrategy::StableAutoCompounder(StableAutoCompounder::new(
                    strategy_fee,
                    strat_creator,
                    sentry_fee,
                    exchange_contract_id,
                    farm_contract_id,
                    pool_id,
                    seed_id.clone(),
                    seed_min_deposit,
                ));

            if let Some(share_id) = uxu_share_id {
                log!("Registering {} to {}", share_id, seed_id);
                //Registering id for the specific seed
                data_mut
                    .fft_share_by_seed_id
                    .insert(seed_id.clone(), share_id.clone());

                //Registering id in the users balance map
                let temp = LookupMap::new(StorageKey::Strategy {
                    fft_share_id: share_id.clone(),
                });

                data_mut
                    .users_balance_by_fft_share
                    .insert(&share_id.clone(), &temp);

                //Registering total_supply
                data_mut
                    .total_supply_by_fft_share
                    .insert(&share_id, &0_u128);
            }

            // TODO: update to seed id
            data_mut.strategies.insert(seed_id, strat);

            format!("VersionedStrategy for {} created successfully", token_id)
        };
    }

    pub fn add_farm_to_stable_strategy(
        &mut self,
        seed_id: String,
        token_address: AccountId,
        pool_id_token_reward: u64,
        token_position: u64,
        reward_token: AccountId,
        available_balance: Vec<Balance>,
        farm_id: String,
    ) -> String {
        self.is_owner_or_guardians();
        let stable_compounder = self.get_strat_mut(&seed_id).get_stable_compounder_mut();

        for farm in stable_compounder.farms.clone() {
            if farm.id == farm_id {
                return format!("Farm with index {} for {} already exist", farm_id, seed_id);
            }
        }

        let farm_info: StableStratFarmInfo = StableStratFarmInfo {
            state: AutoCompounderState::Running,
            cycle_stage: AutoCompounderCycle::ClaimReward,
            slippage: 99u128,
            last_reward_amount: 0u128,
            last_fee_amount: 0u128,
            token_address,
            pool_id_token_reward,
            token_position,
            reward_token,
            available_balance,
            id: farm_id.clone(),
        };

        stable_compounder.farms.push(farm_info);

        format!(
            "Farm with index {} for {} created successfully",
            farm_id, seed_id
        )
    }

    pub fn create_jumbo_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        self.is_owner_or_guardians();

        let token_id = wrap_mft_token_id(&pool_id.to_string());

        return if self.data().strategies.contains_key(&token_id) {
            format!("VersionedStrategy for {} already exist", token_id)
        } else {
            let seed_id: String = format!("{}@{}", exchange_contract_id, pool_id);
            let uxu_share_id = self.new_fft_share(seed_id.clone());

            let data_mut = self.data_mut();

            let strat: VersionedStrategy =
                VersionedStrategy::JumboAutoCompounder(JumboAutoCompounder::new(
                    strategy_fee,
                    strat_creator,
                    sentry_fee,
                    exchange_contract_id,
                    farm_contract_id,
                    token1_address,
                    token2_address,
                    pool_id,
                    seed_id.clone(),
                    seed_min_deposit,
                ));

            if let Some(share_id) = uxu_share_id {
                log!("Registering {} to {}", share_id, seed_id);
                //Registering id for the specific seed
                data_mut
                    .fft_share_by_seed_id
                    .insert(seed_id.clone(), share_id.clone());

                //Registering id in the users balance map
                let temp = LookupMap::new(StorageKey::Strategy {
                    fft_share_id: share_id.clone(),
                });

                data_mut
                    .users_balance_by_fft_share
                    .insert(&share_id.clone(), &temp);

                //Registering total_supply
                data_mut
                    .total_supply_by_fft_share
                    .insert(&share_id, &0_u128);
            }

            data_mut.strategies.insert(seed_id.clone(), strat);

            format!("VersionedStrategy for {} created successfully", seed_id)
        };
    }

    pub fn add_farm_to_jumbo_strategy(
        &mut self,
        seed_id: String,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
    ) -> String {
        self.is_owner_or_guardians();
        let compounder = self.get_strat_mut(&seed_id).get_jumbo_mut();

        for farm in compounder.farms.clone() {
            if farm.id == farm_id {
                return format!("Farm with index {} for {} already exist", farm_id, seed_id);
            }
        }

        let farm_info: JumboStratFarmInfo = JumboStratFarmInfo {
            state: JumboAutoCompounderState::Running,
            cycle_stage: JumboAutoCompounderCycle::ClaimReward,
            slippage: 99u128,
            last_reward_amount: 0u128,
            current_shares_to_stake: 0u128,
            last_fee_amount: 0u128,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            available_balance: vec![0u128, 0u128],
            id: farm_id.clone(),
        };

        compounder.farms.push(farm_info);

        format!(
            "Farm with index {} for {} created successfully",
            farm_id, seed_id
        )
    }

    #[private]
    fn new_fft_share(&mut self, seed_id: String) -> Option<String> {
        let already_has = self.data_mut().fft_share_by_seed_id.get(&seed_id).is_some();
        let fft_share_id;
        if already_has {
            fft_share_id = None
        } else {
            let num: u128 =
                u128::try_from(self.data_mut().fft_share_by_seed_id.keys().len()).unwrap() + 1_u128;
            fft_share_id = Some(format!("fft_share_{num}"));
            log!(
                "new fft_share created: {} for seed_id {}",
                fft_share_id.clone().unwrap(),
                seed_id
            );
        }

        fft_share_id
    }

    pub fn harvest(&mut self, farm_id_str: String, strat_name: String) -> PromiseOrValue<u128> {
        let treasury = self.data().treasury.clone();

        let strat = if !strat_name.is_empty() {
            self.pemb_get_strat_mut(&strat_name)
        } else {
            let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());
            self.get_strat_mut(&seed_id)
        };

        strat.harvest_proxy(farm_id_str, strat_name, treasury)
    }

    pub fn delete_strategy_by_farm_id(&mut self, farm_id_str: String) {
        self.is_owner_or_guardians();
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.clone());
        let strat = self.get_strat_mut(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                for (i, farm) in compounder.farms.iter().enumerate() {
                    println!("{} - {}", farm_id_str, farm.id);
                    if farm_id == farm.id {
                        compounder.farms.remove(i);
                        break;
                    }
                }
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                for (i, farm) in compounder.farms.iter().enumerate() {
                    println!("{} - {}", farm_id_str, farm.id);
                    if farm_id == farm.id {
                        compounder.farms.remove(i);
                        break;
                    }
                }
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                for (i, farm) in compounder.farms.iter().enumerate() {
                    println!("{} - {}", farm_id_str, farm.id);
                    if farm_id == farm.id {
                        compounder.farms.remove(i);
                        break;
                    }
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn delete_strategy_by_strat_name(&mut self, strat_name: String) {
        self.is_owner_or_guardians();
        self.data_mut().strategies.remove(&strat_name);
    }

    pub fn pembrock_create_strategy(
        &mut self,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        pembrock_contract_id: AccountId,
        pembrock_reward_id: AccountId,
        token_address: AccountId,
        pool_id: u64,
        reward_token: AccountId,
    ) -> String {
        self.is_owner();

        // pembrock@usdt
        let strat_name: String = format!("pembrock@{}", token_address);

        return if self.data().strategies.contains_key(&strat_name) {
            format!("VersionedStrategy for {} already exist", token_address)
        } else {
            let uxu_share_id = self.new_fft_share(strat_name.clone());

            let data_mut = self.data_mut();

            let strat: VersionedStrategy =
                VersionedStrategy::PembrockAutoCompounder(PembrockAutoCompounder::new(
                    strategy_fee,
                    strat_creator,
                    sentry_fee,
                    exchange_contract_id,
                    pembrock_contract_id,
                    pembrock_reward_id,
                    token_address.clone(),
                    pool_id,
                    reward_token,
                ));

            if let Some(share_id) = uxu_share_id {
                log!("Registering {} to {}", share_id, &strat_name);
                //Registering id for the specific seed
                data_mut
                    .fft_share_by_seed_id
                    .insert(strat_name.clone(), share_id.clone());

                //Registering id in the users balance map
                let temp = LookupMap::new(StorageKey::Strategy {
                    fft_share_id: share_id.clone(),
                });

                data_mut
                    .users_balance_by_fft_share
                    .insert(&share_id.clone(), &temp);

                //Registering total_supply
                data_mut
                    .total_supply_by_fft_share
                    .insert(&share_id, &0_u128);
            }

            data_mut
                .strategies
                .insert(strat_name.clone(), strat.clone());

            // let farm_info: PembStratFarmInfo = PembStratFarmInfo {
            //     state: PembAutoCompounderState::Running,
            //     cycle_stage: PembAutoCompounderCycle::ClaimReward,
            //     slippage: 99u128,
            //     last_reward_amount: 0u128,
            //     last_fee_amount: 0u128,
            //     pool_id_token1_reward: pool_id,
            //     // TODO: pass as parameter
            //     reward_token: "token.pembrock.testnet".parse().unwrap(),
            //     available_balance: vec![0u128, 0u128],
            // };

            // strat.pemb_get_mut().farms.push(farm_info);

            format!(
                "VersionedStrategy for {} created successfully",
                token_address
            )
        };
    }
}
