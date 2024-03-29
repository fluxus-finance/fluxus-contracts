use crate::*;

/// Internal methods implementation.
#[near_bindgen]
impl Contract {
    pub fn update_contract_state(&mut self, state: RunningState) -> String {
        self.is_owner();
        self.data_mut().state = state;
        format!("{} is {:#?}", env::current_account_id(), self.data().state)
    }

    pub fn update_treasure_contract(&mut self, contract_id: AccountId) {
        self.is_owner();
        self.data_mut().treasury.account_id = contract_id;
    }

    /// Returns allowed_accounts
    pub fn get_allowed_accounts(&self) -> Vec<AccountId> {
        self.is_owner_or_guardians();
        self.data().allowed_accounts.clone()
    }

    /// Returns all strategies without filtering
    pub fn get_strats_info(self) -> Vec<VersionedStrategy> {
        self.is_owner_or_guardians();

        let mut info: Vec<VersionedStrategy> = Vec::new();

        // TODO: should exist a `get_strategies` and upgrade everything at once if so?
        for (_, strat) in self.data().strategies.clone() {
            info.push(strat);
        }

        info
    }

    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   state: Running, Ended, ...
    pub fn update_compounder_state(
        &mut self,
        farm_id_str: String,
        state: AutoCompounderState,
    ) -> String {
        self.is_owner_or_guardians();

        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);
        // TODO: stable version
        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        if farm_info_mut.state != state {
            farm_info_mut.state = state;
        }

        format!("The current state is {:#?}", farm_info_mut.state)
    }

    /// Extend guardians. Only can be called by owner.
    #[payable]
    pub fn extend_guardians(&mut self, guardians: Vec<AccountId>) {
        assert_one_yocto();
        self.is_owner();
        for guardian in guardians {
            self.data_mut().guardians.insert(&guardian);
        }
    }

    /// Remove guardians. Only can be called by owner.
    #[payable]
    pub fn remove_guardians(&mut self, guardians: Vec<AccountId>) {
        assert_one_yocto();
        self.is_owner();
        for guardian in guardians {
            self.data_mut().guardians.remove(&guardian);
        }
    }

    #[private]
    pub fn is_owner_or_guardians(&self) -> bool {
        env::predecessor_account_id() == self.data().owner_id
            || self
                .data()
                .guardians
                .contains(&env::predecessor_account_id())
    }

    /// Update slippage for given token_id
    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   new_slippage: value between 80-100
    pub fn update_strat_slippage(&mut self, farm_id_str: String, new_slippage: u128) -> String {
        assert!(self.is_owner_or_guardians(), "ERR: not allowed");
        // TODO: what maximum slippage should be accepted?
        // Should not accept, say, 0 slippage
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        // TODO: stable versions
        let compounder_mut = self.get_strat_mut(&seed_id).get_compounder_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);
        farm_info_mut.slippage = 100 - new_slippage;

        format!(
            "The current slippage for {} is {}",
            seed_id, farm_info_mut.slippage
        )
    }

    /// Adds account_id to allowed_accounts if it is not already present
    pub fn add_allowed_account(&mut self, account_id: AccountId) {
        self.is_owner();

        require!(
            !self.data().allowed_accounts.contains(&account_id),
            "ERR_ACCOUNT_ALREADY_EXIST"
        );
        self.data_mut().allowed_accounts.push(account_id)
    }

    /// Removes account_id from allowed_accounts
    pub fn remove_allowed_account(&mut self, account_id: AccountId) {
        self.is_owner();

        let accounts = self.data().allowed_accounts.clone();

        // https://stackoverflow.com/a/44012406
        self.data_mut().allowed_accounts.swap_remove(
            accounts
                .iter()
                .position(|x| *x == account_id)
                .expect("ERR_ACCOUNT_DOES_NOT_EXIST"),
        );
    }

    /// Checks if predecessor_account_id is either the contract or the owner of the contract
    #[private]
    pub(crate) fn is_owner(&self) {
        let (caller_acc_id, contract_id) = get_predecessor_and_current_account();
        require!(
            caller_acc_id == contract_id || caller_acc_id == self.data().owner_id,
            "ERR_NOT_ALLOWED"
        );
    }

    /// Checks if account_id is either the caller account or the contract
    #[private]
    pub(crate) fn is_caller(&self, account_id: AccountId) {
        let (caller_acc_id, contract_id) = get_predecessor_and_current_account();
        assert!(
            (caller_acc_id == account_id) || (caller_acc_id == contract_id),
            "ERR_NOT_ALLOWED"
        );
    }

    /// Checks if the caller account is in allowed_accounts
    #[private]
    pub(crate) fn is_allowed_account(&self) {
        let caller_acc_id: &AccountId = &env::predecessor_account_id();

        let mut is_allowed: bool = false;

        for account in &self.data().allowed_accounts {
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
            self.data().owner_id,
            "ERR_NOT_ALLOWED"
        );
        for token in tokens {
            self.data_mut().whitelisted_tokens.insert(&token);
        }
    }

    pub fn contract_version(&self) -> String {
        String::from(env!("CARGO_PKG_VERSION"))
    }

    // TODO: REMOVE
    #[private]
    pub fn check_promise(&self) -> bool {
        match env::promise_results_count() {
            0 => true,
            1 => match env::promise_result(0) {
                PromiseResult::Successful(_) => {
                    env::log_str("Check_promise successful");
                    true
                }
                PromiseResult::Failed => env::panic_str("ERR_CALL_FAILED"),
                _ => false,
            },
            _ => false,
        }
    }
}

// #[cfg(all(test, not(target_arch = "wasm32")))]
// mod tests {
//     use super::*;
//     use near_sdk::test_utils::VMContextBuilder;
//     use near_sdk::testing_env;

//     fn get_context() -> VMContextBuilder {
//         let mut builder = VMContextBuilder::new();
//         builder
//             .current_account_id(to_account_id("auto_compounder.near"))
//             .signer_account_id(to_account_id("auto_compounder.near"))
//             .predecessor_account_id(to_account_id("auto_compounder.near"));
//         builder
//     }

//     pub fn to_account_id(value: &str) -> AccountId {
//         value.parse().unwrap()
//     }

//     fn create_contract() -> Contract {
//         let contract = Contract::new(
//             to_account_id("auto_compounder.near"),
//             0u128,
//             String::from("eth.near"),
//             String::from("dai.near"),
//             0,
//             1,
//             String::from("usn.near"),
//             String::from(""),
//             String::from(""),
//             0,
//             0,
//             U128(100),
//         );

//         contract
//     }

//     #[test]
//     fn test_pool_to_token_id() {
//         let context = get_context();
//         testing_env!(context.build());

//         let contract = create_contract();

//         assert_eq!(
//             contract.wrap_mft_token_id(String::from("100")),
//             String::from(":100")
//         );
//         assert_ne!(
//             contract.wrap_mft_token_id(String::from("100")),
//             String::from("100")
//         );
//     }

//     #[test]
//     fn test_update_minimum_deposit() {
//         let context = get_context();
//         testing_env!(context.build());

//         let mut contract = create_contract();

//         assert_eq!(contract.get_seed_min_deposit(), U128(100));

//         contract.update_seed_min_deposit(U128(1000));
//         assert_eq!(contract.get_seed_min_deposit(), U128(1000));
//     }

//     #[test]
//     fn test_whitelisted_tokens() {
//         let context = get_context();
//         testing_env!(context.build());

//         let mut contract = create_contract();

//         assert_eq!(contract.get_whitelisted_tokens(), vec![]);

//         contract.extend_whitelisted_tokens(vec![to_account_id("usn.near")]);
//         assert_eq!(
//             contract.get_whitelisted_tokens(),
//             vec![to_account_id("usn.near")]
//         );
//     }

//     #[test]
//     #[should_panic]
//     fn test_allowed_accounts() {
//         let context = get_context();
//         testing_env!(context.build());

//         let mut contract = create_contract();

//         assert_eq!(
//             contract.get_allowed_accounts(),
//             vec![to_account_id("auto_compounder.near")]
//         );

//         contract.add_allowed_account(to_account_id("manager.near"));
//         assert_eq!(
//             contract.get_allowed_accounts(),
//             vec![
//                 to_account_id("auto_compounder.near"),
//                 to_account_id("manager.near")
//             ]
//         );

//         contract.remove_allowed_account(to_account_id("auto_compounder.near"));
//         assert_eq!(
//             contract.get_allowed_accounts(),
//             vec![to_account_id("manager.near")]
//         );

//         // should panic because there is no auto_compounder.near in the vector after it was removed
//         contract.remove_allowed_account(to_account_id("auto_compounder.near"));
//     }

//     #[test]
//     fn test_callers_checks() {
//         let mut context = get_context();
//         testing_env!(context.build());

//         let mut contract = create_contract();

//         // both contract and owner (caller) have permissions
//         contract.is_allowed_account();
//         contract.is_owner();
//         contract.is_caller(to_account_id("auto_compounder.near"));

//         // update caller to a different value
//         testing_env!(context
//             .predecessor_account_id(to_account_id("fluxus.near"))
//             .build());

//         // https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
//         // should panic because the caller is not present in allowed_accounts
//         let result = std::panic::catch_unwind(|| contract.is_allowed_account());
//         assert!(result.is_err());

//         // should panic because the caller is not the contract or the owner of the contract
//         let result = std::panic::catch_unwind(|| contract.is_owner());
//         assert!(result.is_err());

//         // should panic because the caller is not the contract or the account being consulted
//         let result = std::panic::catch_unwind(|| {
//             contract.is_caller(to_account_id("fluxus_finance.near"))
//         });
//         assert!(result.is_err());
//     }
// }
