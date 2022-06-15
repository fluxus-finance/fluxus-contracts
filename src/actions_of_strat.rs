use crate::*;

#[near_bindgen]
impl Contract {
    // TODO: this method should register in the correct pool/farm
    pub fn create_strategy(
        &mut self,
        _strategy: String,
        protocol_fee: u128,
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
        let seed_id: String = format!("{}@{}", self.exchange_contract_id, pool_id);
        let farm_id: String = format!("{}#{}", seed_id, farm);

        let token_id = self.wrap_mft_token_id(&pool_id.to_string());
        self.token_ids.push(token_id.clone());

        let strat: Strategy = Strategy::AutoCompounder(AutoCompounder::new(
            protocol_fee,
            token1_address,
            token2_address,
            pool_id_token1_reward,
            pool_id_token2_reward,
            reward_token,
            farm_id,
            pool_id,
            seed_id,
            seed_min_deposit,
        ));

        self.strategies.insert(token_id.clone(), strat);

        format!("Strategy for {} created successfully", token_id)
    }
}
