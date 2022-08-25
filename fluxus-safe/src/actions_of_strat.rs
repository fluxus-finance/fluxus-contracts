use crate::*;

#[near_bindgen]
impl Contract {
    // TODO: this method should register in the correct pool/farm
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
        self.is_owner();

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
        self.is_owner();
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

    pub fn harvest(&self, farm_id_str: String) -> Promise {
        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());

        let treasury = self.data().treasury.clone();
        let strat = self.get_strat(&seed_id);

        strat.harvest_proxy(farm_id_str, treasury)
    }

    // TODO: stable version
    pub fn delete_strategy_by_farm_id(&mut self, farm_id_str: String) {
        self.is_owner();
        let (_, token_id, _) = get_ids_from_farm(farm_id_str.clone());
        let strat = self.get_strat_mut(&token_id);
        let compounder = strat.get_compounder_mut();
        for (i, farm) in compounder.farms.iter().enumerate() {
            println!("{} - {}", farm_id_str, farm.id);
            if farm_id_str == farm.id {
                compounder.farms.remove(i);
                break;
            }
        }
    }
}
