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
            farm_id,
            pool_id,
            seed_id.clone(),
            seed_min_deposit,
        ));

        let uxu_share_id = self.new_uxu_share(seed_id.clone());
        if let Some(id) = uxu_share_id {
            log!("Registering {} to {}", id, seed_id);
            //Registering id for the specific seed
            self.data_mut()
                .uxu_share_by_seed_id
                .insert(seed_id, id.clone());

            //Registering id in the users balance map
            let mut temp = HashMap::new();
            temp.insert("".to_string(), 0_u128);
            self.data_mut()
                .users_balance_by_uxu_share
                .insert(id.clone(), temp);

            //Registering total_supply
            self.data_mut().total_supply_by_uxu_share.insert(id, 0_u128);
        }

        self.data_mut().strategies.insert(token_id.clone(), strat);

        format!("VersionedStrategy for {} created successfully", token_id)
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
    pub fn harvest(&mut self, token_id: String) -> Promise {
        let strat = self.get_strat(&token_id).get();
        match strat.cycle_stage {
            AutoCompounderCycle::ClaimReward => self.claim_reward(token_id),
            AutoCompounderCycle::Withdrawal => self.withdraw_of_reward(token_id),
            AutoCompounderCycle::Swap => self.autocompounds_swap(token_id),
            AutoCompounderCycle::Stake => self.autocompounds_liquidity_and_stake(token_id),
        }
    }
}
