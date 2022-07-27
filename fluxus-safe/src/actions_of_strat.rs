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
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm: String,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        self.is_owner();
        let seed_id: String = format!("{}@{}", self.data().exchange_contract_id, pool_id);
        let farm_id: String = format!("{}#{}", seed_id, farm);

        let treasury = self.data().treasury.clone();

        let token_id = self.wrap_mft_token_id(&pool_id.to_string());
        self.data_mut().token_ids.push(token_id.clone());

        let strat: VersionedStrategy = VersionedStrategy::AutoCompounder(AutoCompounder::new(
            strategy_fee,
            treasury,
            strat_creator,
            sentry_fee,
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            farm_id.clone(),
            pool_id,
            seed_id.clone(),
            seed_min_deposit,
        ));

        let fft_share_id = self.new_fft_share(seed_id.clone());
        if let Some(id) = fft_share_id {
            log!("Registering {} to {}", id, seed_id);
            //Registering id for the specific seed TODO: this makes no sense existing any longer
            self.data_mut()
                .fft_share_by_seed_id
                .insert(seed_id.clone(), id.clone());

            //Registering id in the users balance map
            let mut temp = HashMap::new();
            temp.insert("".to_string(), 0_u128);
            self.data_mut()
                .users_balance_by_fft_share
                .insert(id.clone(), temp);

            //Registering total_supply
            self.data_mut().total_supply_by_fft_share.insert(id, 0_u128);
            let mut new_set: HashSet<String> = HashSet::new();
            new_set.insert(farm_id.clone());
            self.data_mut().farms_by_seed_id.insert(seed_id,new_set.clone());
        }
        else {
            self.data_mut().farms_by_seed_id.get_mut(&seed_id).unwrap().insert(farm_id.clone());
        }

        self.data_mut().strategies.insert(token_id.clone(), strat.clone());
        self.data_mut().strats_by_farm.insert(farm_id,strat);

        format!("VersionedStrategy for {} created successfully", token_id)
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
    pub fn harvest(&mut self, farm_id: String) -> Promise {
        let strat = self.get_strat_by_farm(&farm_id).get();
        match strat.cycle_stage {
            AutoCompounderCycle::ClaimReward => self.claim_reward(farm_id),
            AutoCompounderCycle::Withdrawal => self.withdraw_of_reward(farm_id),
            AutoCompounderCycle::Swap => self.autocompounds_swap(farm_id),
            AutoCompounderCycle::Stake => self.autocompounds_liquidity_and_stake(farm_id),
        }
    }
}
