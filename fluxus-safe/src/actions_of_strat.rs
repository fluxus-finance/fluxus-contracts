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
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        self.is_owner();

        let token_id = self.wrap_mft_token_id(&pool_id.to_string());

        return if self.data().strategies.contains_key(&token_id) {
            format!("VersionedStrategy for {} already exist", token_id)
        } else {
            let seed_id: String = format!("{}@{}", self.data().exchange_contract_id, pool_id);
            let uxu_share_id = self.new_uxu_share(seed_id.clone());

            let data_mut = self.data_mut();
            let treasury = data_mut.treasury.clone();
            data_mut.token_ids.push(token_id.clone());

            let strat: VersionedStrategy = VersionedStrategy::AutoCompounder(AutoCompounder::new(
                strategy_fee,
                treasury,
                strat_creator,
                sentry_fee,
                token1_address,
                token2_address,
                pool_id,
                seed_id.clone(),
                seed_min_deposit,
            ));

            if let Some(id) = uxu_share_id {
                log!("Registering {} to {}", id, seed_id);
                //Registering id for the specific seed
                data_mut.uxu_share_by_seed_id.insert(seed_id, id.clone());

                //Registering id in the users balance map
                let mut temp = HashMap::new();
                temp.insert("".to_string(), 0_u128);
                data_mut.users_balance_by_uxu_share.insert(id.clone(), temp);

                //Registering total_supply
                data_mut.total_supply_by_uxu_share.insert(id, 0_u128);
            }

            data_mut.strategies.insert(token_id.clone(), strat);

            format!("VersionedStrategy for {} created successfully", token_id)
        };
    }

    pub fn add_farm_to_strategy(
        &mut self,
        pool_id: u64,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
    ) -> String {
        self.is_owner();
        let token_id = self.wrap_mft_token_id(&pool_id.to_string());

        // let data_mut = self.data_mut();
        let compounder = self.get_strat_mut(&token_id).get_mut();

        for farm in compounder.farms.clone() {
            if farm.id == farm_id {
                return format!("Farm with index {} for {} already exist", farm_id, token_id);
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
            farm_id, token_id
        )
    }

    fn new_uxu_share(&mut self, seed_id: String) -> Option<String> {
        let already_has = self.data_mut().uxu_share_by_seed_id.get(&seed_id).is_some();
        let uxu_share_id;
        if already_has {
            uxu_share_id = None
        } else {
            let num: u128 =
                u128::try_from(self.data_mut().uxu_share_by_seed_id.keys().len()).unwrap() + 1_u128;
            uxu_share_id = Some(format!("uxu_share_{num}"));
            log!(
                "new uxu_share created: {} for seed_id {}",
                uxu_share_id.clone().unwrap(),
                seed_id
            );
        }

        uxu_share_id
    }
    pub fn harvest(&mut self, farm_id_str: String) -> Promise {
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        log!("{} {}", token_id, farm_id);
        let strat = self.get_strat(token_id);
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);
        match farm_info.cycle_stage {
            AutoCompounderCycle::ClaimReward => self.claim_reward(farm_id_str),
            AutoCompounderCycle::Withdrawal => self.withdraw_of_reward(farm_id_str),
            AutoCompounderCycle::Swap => self.autocompounds_swap(farm_id_str),
            AutoCompounderCycle::Stake => self.autocompounds_liquidity_and_stake(farm_id_str),
        }
    }
}
