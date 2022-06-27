use crate::*;

#[near_bindgen]
impl Contract {
    /// Receives token address and pool id of the pair token-token_out
    /// Register the token in the exchange to be used by the contract
    /// Register the contract in the token to allow transfers
    pub fn register_token(&mut self, token: AccountId, pool_id: u64) -> Promise {
        self.is_owner();
        assert_eq!(
            self.token_to_pool.contains_key(&token),
            false,
            "TREASURER::ERR_TOKEN_ALREADY_EXIST"
        );

        ext_exchange::register_tokens(
            vec![token.clone()],
            self.exchange_contract_id.clone(),
            1,
            Gas(20_000_000_000_000),
        )
        .and(ext_input_token::storage_deposit(
            env::current_account_id(),
            false,
            token.clone(),
            10000000000000000000000,
            Gas(60_000_000_000_000),
        ))
        .then(ext_self::callback_register_token(
            token,
            pool_id,
            env::current_account_id(),
            0,
            Gas(80_000_000_000_000),
        ))
    }

    /// Callback to ensure that both register_tokens and storage_deposit were successful
    #[private]
    pub fn callback_register_token(
        &mut self,
        #[callback_result] register_result: Result<(), PromiseError>,
        #[callback_result] deposit_result: Result<StorageBalance, PromiseError>,
        token: AccountId,
        pool_id: u64,
    ) -> String {
        assert!(register_result.is_ok(), "TREASURER::COULD_NOT_REGISTER");
        assert!(deposit_result.is_ok(), "TREASURER::COULD_NOT_DEPOSIT");

        self.token_to_pool.insert(token.clone(), pool_id.clone());

        format!(
            "The token {} with pool {} was added successfully",
            token, pool_id
        )
    }

    /// Updates the pool related to the given token
    pub fn update_token_pool(&mut self, token: AccountId, pool_id: u64) -> String {
        self.is_owner();
        assert!(
            self.token_to_pool.contains_key(&token),
            "TREASURER::ERR_TOKEN_DOES_NOT_EXIST"
        );

        self.token_to_pool.insert(token.clone(), pool_id);

        format!(
            "The token {} with pool {} was updated successfully",
            token, pool_id
        )
    }

    pub fn get_registered_tokens(&self) -> HashMap<AccountId, u64> {
        self.token_to_pool.clone()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(to_account_id("auto_compounder.near"))
            .signer_account_id(to_account_id("auto_compounder.near"))
            .predecessor_account_id(to_account_id("auto_compounder.near"));
        builder
    }

    pub fn to_account_id(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn create_contract() -> Contract {
        let contract = Contract::new(
            to_account_id("auto_compounder.near"),
            "exchange.near".parse().unwrap(),
            "wnear".parse().unwrap(),
        );

        contract
    }

    #[test]
    #[should_panic]
    fn test_update_token_pool() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        let token = to_account_id("usn.near");
        let pool_id = 100u64;

        contract.token_to_pool.insert(token.clone(), pool_id);
        contract.update_token_pool(token, pool_id + 1);

        let token2 = to_account_id("dai.near");

        // should panic if trying to update a token that is not registered
        contract.update_token_pool(token2, pool_id);
    }

    #[test]
    fn test_get_token_to_pool() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        let token = to_account_id("usn.near");
        let pool_id = 100u64;

        contract.token_to_pool.insert(token.clone(), pool_id);

        let registered_tokens: HashMap<AccountId, u64> = contract.get_registered_tokens();
        assert_eq!(
            registered_tokens.len(),
            1,
            "ERR_COULD_NOT_GET_REGISTERED_TOKENS",
        );

        let pool = registered_tokens.get(&token).unwrap();
        assert_eq!(pool_id, *pool, "ERR_COULD_NOT_REGISTER_TOKENS");
    }
}
