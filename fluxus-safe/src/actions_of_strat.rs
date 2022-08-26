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
        let compounder = self.get_strat_mut(&seed_id).get_mut();

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

    pub fn harvest(&mut self, farm_id_str: String) -> Promise {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let strat = self.get_strat(&seed_id);
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);
        match farm_info.cycle_stage {
            AutoCompounderCycle::ClaimReward => self.claim_reward(farm_id_str),
            AutoCompounderCycle::Withdrawal => self.withdraw_of_reward(farm_id_str),
            AutoCompounderCycle::Swap => self.autocompounds_swap(farm_id_str),
            AutoCompounderCycle::Stake => self.autocompounds_liquidity_and_stake(farm_id_str),
        }
    }

    pub fn delete_strategy_by_farm_id(&mut self, farm_id_str: String) {
        self.is_owner();
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.clone());
        let strat = self.get_strat_mut(&token_id);
        let compounder = strat.get_mut();
        for (i, farm) in compounder.farms.iter().enumerate() {
            println!("{} - {}", farm_id_str, farm.id);
            if farm_id_str == farm.id {
                compounder.farms.remove(i);
                break;
            }
        }
    }



    pub fn pembrock_create_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token_name: String,
        token1_address: AccountId,
        seed_min_deposit: U128,
        pool_id: u64
    ) -> String {
        self.is_owner();

        //let token_id = wrap_mft_token_id(&pool_id.to_string());

        let strat_name: String = format!("pembrock@{}", token_name);

        return if self.data().strategies.contains_key(&strat_name) {
            format!("VersionedStrategy for {} already exist", token_name)
        } else {
            let uxu_share_id = self.new_fft_share(strat_name.clone());

            let data_mut = self.data_mut();

            let mut strat: VersionedStrategy = VersionedStrategy::PembrockAutoCompounder(PembrockAutoCompounder::new(
                strategy_fee,
                strat_creator,
                sentry_fee,
                exchange_contract_id,
                farm_contract_id,
                token1_address,
                token_name.clone(),
                seed_min_deposit,
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


            data_mut.strategies.insert(strat_name.clone(), strat.clone());

    
            let farm_info: PembStratFarmInfo = PembStratFarmInfo {
                state: PembAutoCompounderState::Running,
                cycle_stage: PembAutoCompounderCycle::ClaimReward,
                slippage: 99u128,
                last_reward_amount: 0u128,
                last_fee_amount: 0u128,
                pool_id_token1_reward:pool_id,
                reward_token: "token.pembrock.testnet".parse().unwrap(),
                available_balance: vec![0u128, 0u128],
            };
    
            strat.pemb_get_mut().farms.push(farm_info);




            format!("VersionedStrategy for {} created successfully", token_name)
        };
    }


}
