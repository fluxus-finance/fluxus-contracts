use crate::*;

#[near_bindgen]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.assert_contract_running();
        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let registration_only = registration_only.unwrap_or(false);
        let min_balance = self.storage_balance_bounds().min.0;
        let already_registered = self.accounts.contains_key(&account_id);
        if amount < min_balance && !already_registered {
            env::panic_str("ERR_DEPOSIT_LESS_THAN_MIN_STORAGE");
        }
        if registration_only {
            // Registration only setups the account but doesn't leave space for tokens.
            if already_registered {
                log!("ERR_ACC_REGISTERED");
                if amount > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(amount);
                }
            } else {
                self.internal_register_account(&account_id, min_balance);
                let refund = amount - min_balance;
                if refund > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
            }
        } else {
            self.internal_register_account(&account_id, amount);
        }

        if already_registered {
            let amount_already_deposited = self
                .users_total_near_deposited
                .get_mut(&account_id.clone())
                .unwrap()
                .clone();

            self.users_total_near_deposited
                .insert(account_id.clone(), amount + amount_already_deposited);

            log!(
                "before + user_deposited_amount = {}",
                amount + amount_already_deposited
            );
        } else {
            self.users_total_near_deposited
                .insert(account_id.clone(), amount);
            log!("0 + amount = {}", amount);
        }
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }
    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        //assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        let amount = amount.unwrap_or(U128(0)).0;

        require!(
            self.accounts.contains_key(&account_id),
            "Account is not registered"
        );

        let amount_already_deposited = self
            .users_total_near_deposited
            .get_mut(&account_id.clone())
            .unwrap()
            .clone();

        require!(
            amount_already_deposited >= amount,
            "You do not have enough balance"
        );

        let available = u128::from(
            self.storage_balance_of(account_id.clone().try_into().unwrap())
                .unwrap()
                .available,
        );
        let percentage_gains: f64 =
            (available as f64 / amount_already_deposited as f64) * 100_f64 - 100_f64;
        log!(
            "available = {} -> Deposit before = {}, gains = {} = {}%",
            available,
            amount_already_deposited,
            available as i128 - amount_already_deposited as i128,
            percentage_gains
        );

        self.users_total_near_deposited
            .insert(account_id.clone(), amount_already_deposited - amount);
        let withdraw_amount = self.internal_storage_withdraw(&account_id, amount);
        Promise::new(account_id.clone()).transfer(withdraw_amount);
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        if let Some(account_deposit) = self.internal_get_account(&account_id) {
            // TODO: figure out force option logic.
            assert!(
                account_deposit.tokens.is_empty(),
                "ERR_STORAGE_UNREGISTER_TOKENS_NOT_EMPTY"
            );
            self.accounts.remove(&account_id);
            Promise::new(account_id.clone()).transfer(account_deposit.near_amount);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: Account::min_storage_usage().into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_get_account(&account_id)
            .map(|account| StorageBalance {
                total: U128(account.near_amount),
                available: U128(account.storage_available()),
            })
    }
}



mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .attached_deposit(1000000000000000000000000u128)
            .current_account_id(to_account_id("auto_compounder.near"))
            .signer_account_id(to_account_id("auto_compounder.near"))
            .predecessor_account_id(to_account_id("auto_compounder.near"));
        builder
    }

    fn get_context_yocto() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .attached_deposit(1u128)
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
    fn test_storage() {
        let context = get_context();
        testing_env!(context.build());
        let mut contract = create_contract();
        //Is it storing the amount?
        let what_was_deposited = StorageBalance {
            total: U128(1000000000000000000000000),
            available: U128(998980000000000000000000),
        };
        let what_is_deposited =
            contract.storage_deposit(Some(to_account_id("auto_compounder.near")), Some(false));

        //Is the contract storing the correct amount?
        assert_eq!(what_is_deposited.total, what_was_deposited.total);

        //Is the contract getting a part?
        assert_eq!(
            u128::from(what_is_deposited.available),
            u128::from(what_was_deposited.total) - 1020000000000000000000_u128
        );
    }

    #[test]
    fn test_unstorage() {
        let context = get_context();
        testing_env!(context.build());
        let mut contract = create_contract();
        let what_was_deposited = StorageBalance {
            total: U128(1000000000000000000000000),
            available: U128(998980000000000000000000),
        };
        let what_is_supposed_to_rest = StorageBalance {
            total: U128(1020000000000000000000),
            available: U128(0),
        };
        let what_is_deposited =
            contract.storage_deposit(Some(to_account_id("auto_compounder.near")), Some(false));

        let rest = contract.storage_withdraw(Some(what_was_deposited.available));

        //Is the contract doing withdraw correctly?
        assert_eq!(rest.total, what_is_supposed_to_rest.total);
        assert_eq!(rest.available, what_is_supposed_to_rest.available);
    }

    #[test]
    fn test_storage_balance_of() {
        let context = get_context();
        testing_env!(context.build());
        let mut contract = create_contract();
        //Is it storing the amount?
        let what_was_deposited = StorageBalance {
            total: U128(1000000000000000000000000),
            available: U128(998980000000000000000000),
        };

        let what_is_deposited =
            contract.storage_deposit(Some(to_account_id("auto_compounder.near")), Some(false));

        let balance = contract
            .storage_balance_of(to_account_id("auto_compounder.near"))
            .unwrap();

        let total = balance.total;
        let available = balance.available;

        //Is the contract storing the correct amount?
        assert_eq!(total, what_was_deposited.total);
        assert_eq!(available, what_was_deposited.available);
    }

    #[test]
    fn test_unregister() {
        let context = get_context();
        testing_env!(context.build());
        let mut contract = create_contract();
        let deposit =
            contract.storage_deposit(Some(to_account_id("auto_compounder.near")), Some(false));

        let context = get_context_yocto();
        testing_env!(context.build());
        let unregister = contract.storage_unregister(Some(true));
        assert_eq!(unregister, true);
    }
}
