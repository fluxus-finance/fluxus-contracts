use crate::*;

#[near_bindgen]
impl Contract {
    /// Adds account_id to allowed_accounts if it is not already present
    pub fn add_allowed_account(&mut self, account_id: AccountId) {
        self.check_permission();

        require!(
            !self.allowed_accounts.contains(&account_id),
            "ERR_ACCOUNT_ALREADY_EXIST"
        );
        self.allowed_accounts.push(account_id)
    }

    /// Removes account_id from allowed_accounts
    pub fn remove_allowed_account(&mut self, account_id: AccountId) {
        self.check_permission();

        // https://stackoverflow.com/a/44012406
        self.allowed_accounts.swap_remove(
            self.allowed_accounts
                .iter()
                .position(|x| *x == account_id)
                .expect("ERR_ACCOUNT_DOES_NOT_EXIST"),
        );
    }

    /// Returns allowed_accounts
    pub fn get_allowed_accounts(&self) -> Vec<AccountId> {
        self.check_permission();
        self.allowed_accounts.clone()
    }

    /// Checks if predecessor_account_id is either the contract or the owner of the contract
    /// TODO: rename method to is_owner()
    #[private]
    pub(crate) fn check_permission(&self) {
        let (caller_acc_id, contract_id) = self.get_predecessor_and_current_account();
        require!(
            caller_acc_id == contract_id || caller_acc_id == self.owner_id,
            "ERR_NOT_ALLOWED"
        );
    }

    /// Checks if account_id is either the caller account or the contract
    /// TODO: rename method to is_caller()
    #[private]
    pub(crate) fn check_caller(&self, account_id: AccountId) {
        let (caller_acc_id, contract_id) = self.get_predecessor_and_current_account();
        assert!(
            (caller_acc_id == account_id) || (caller_acc_id == contract_id),
            "ERR_NOT_ALLOWED"
        );
    }

    /// Checks if the caller account is in allowed_accounts
    /// TODO: rename method to is_allowed_account()
    #[private]
    pub(crate) fn check_autocompounds_caller(&self) {
        let contract = env::current_account_id();

        let caller_acc_id: &AccountId = &env::predecessor_account_id();

        let mut is_allowed: bool = false;

        for account in &self.allowed_accounts {
            if caller_acc_id == account {
                is_allowed = true;
                break;
            }
        }

        assert!(is_allowed, "ERR_NOT_ALLOWED");
    }

    /// Extend the whitelist of tokens.
    #[payable]
    pub fn extend_whitelisted_tokens(&mut self, tokens: Vec<AccountId>) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "ERR_NOT_ALLOWED"
        );
        for token in tokens {
            self.whitelisted_tokens.insert(&token);
        }
    }

    /// Return the whitelisted tokens.
    pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
        self.whitelisted_tokens.to_vec()
    }

    #[private]
    pub fn get_predecessor_and_current_account(&self) -> (AccountId, AccountId) {
        (env::predecessor_account_id(), env::current_account_id())
    }

    pub fn contract_version(&self) -> String {
        String::from(env!("CARGO_PKG_VERSION"))
    }

    #[private]
    /// wrap token_id into correct format in MFT standard
    pub fn wrap_mft_token_id(&self, token_id: String) -> String {
        format!(":{}", token_id)
    }

    pub fn update_seed_min_deposit(&mut self, min_deposit: U128) -> U128 {
        self.check_permission();
        self.seed_min_deposit = min_deposit;
        self.seed_min_deposit
    }

    pub fn get_seed_min_deposit(&self) -> U128 {
        self.seed_min_deposit
    }

    #[private]
    pub fn check_promise(&self) -> bool {
        match env::promise_results_count() {
            0 => {
                return true;
            }
            1 => {
                match env::promise_result(0) {
                    PromiseResult::Successful(_) => {
                        env::log_str("Check_promise successful");
                        return true;
                    }
                    _ => return false,
                };
            }
            _ => false,
        }
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
            0u128,
            String::from("eth.near"),
            String::from("dai.near"),
            0,
            1,
            String::from("usn.near"),
            String::from(""),
            String::from(""),
            0,
            0,
            U128(100),
        );

        contract
    }

    #[test]
    fn test_pool_to_token_id() {
        let context = get_context();
        testing_env!(context.build());

        let contract = create_contract();

        assert_eq!(
            contract.wrap_mft_token_id(String::from("100")),
            String::from(":100")
        );
        assert_ne!(
            contract.wrap_mft_token_id(String::from("100")),
            String::from("100")
        );
    }

    #[test]
    fn test_update_minimum_deposit() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        assert_eq!(contract.get_seed_min_deposit(), U128(100));

        contract.update_seed_min_deposit(U128(1000));
        assert_eq!(contract.get_seed_min_deposit(), U128(1000));
    }

    #[test]
    fn test_whitelisted_tokens() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        assert_eq!(contract.get_whitelisted_tokens(), vec![]);

        contract.extend_whitelisted_tokens(vec![to_account_id("usn.near")]);
        assert_eq!(
            contract.get_whitelisted_tokens(),
            vec![to_account_id("usn.near")]
        );
    }

    #[test]
    #[should_panic]
    fn test_allowed_accounts() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        assert_eq!(
            contract.get_allowed_accounts(),
            vec![to_account_id("auto_compounder.near")]
        );

        contract.add_allowed_account(to_account_id("manager.near"));
        assert_eq!(
            contract.get_allowed_accounts(),
            vec![
                to_account_id("auto_compounder.near"),
                to_account_id("manager.near")
            ]
        );

        contract.remove_allowed_account(to_account_id("auto_compounder.near"));
        assert_eq!(
            contract.get_allowed_accounts(),
            vec![to_account_id("manager.near")]
        );

        // should panic because there is no auto_compounder.near in the vector after it was removed
        contract.remove_allowed_account(to_account_id("auto_compounder.near"));
    }

    #[test]
    fn test_callers_checks() {
        let mut context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        // both contract and owner (caller) have permissions
        contract.check_autocompounds_caller();
        contract.check_permission();
        contract.check_caller(to_account_id("auto_compounder.near"));

        // update caller to a different value
        testing_env!(context
            .predecessor_account_id(to_account_id("fluxus.near"))
            .build());

        // https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
        // should panic because the caller is not present in allowed_accounts
        let result = std::panic::catch_unwind(|| contract.check_autocompounds_caller());
        assert!(result.is_err());

        // should panic because the caller is not the contract or the owner of the contract
        let result = std::panic::catch_unwind(|| contract.check_permission());
        assert!(result.is_err());

        // should panic because the caller is not the contract or the account being consulted
        let result = std::panic::catch_unwind(|| {
            contract.check_caller(to_account_id("fluxus_finance.near"))
        });
        assert!(result.is_err());
    }
}
