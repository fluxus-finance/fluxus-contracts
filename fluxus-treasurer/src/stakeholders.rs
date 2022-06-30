use crate::*;

#[near_bindgen]
impl Contract {
    /// Adds account_id and fee percentage to stakeholders_fees if it is not already present
    /// `fee` should be between 0 and 1, otherwise panics if total_fee exceeds 1
    pub fn add_stakeholder(&mut self, account_id: AccountId, fee: u128) -> String {
        self.is_owner();

        let mut total_fees: u128 = 0u128;

        for (acc_id, account_fee) in self.data().stakeholders_fees.iter() {
            assert!(
                *acc_id != account_id,
                "TREASURER::ERR_ADDRESS_ALREADY_EXIST"
            );
            total_fees += account_fee;
        }

        total_fees += fee;

        assert!(
            total_fees <= 100u128,
            "TREASURER::ERR_FEE_EXCEEDS_MAXIMUM_VALUE"
        );

        self.data_mut()
            .stakeholders_fees
            .insert(account_id.clone(), fee);
        self.data_mut()
            .stakeholders_amount_available
            .insert(account_id.clone(), 0u128);

        format!(
            "Account {} was added with {} proportion from value",
            account_id, fee
        )
    }

    /// Removes account from stakeholders_fee
    pub fn remove_stakeholder(&mut self, account_id: AccountId) {
        self.is_owner();
        self.data_mut().stakeholders_fees.remove(&account_id);
    }

    pub fn update_stakeholder_percentage(
        &mut self,
        account_id: AccountId,
        new_percentage: u128,
    ) -> String {
        self.is_owner();
        assert!(
            self.data().stakeholders_fees.contains_key(&account_id),
            "TREASURER::ERR_ACCOUNT_DOES_NOT_EXIST"
        );

        let mut total_fees = new_percentage;
        for (account, percentage) in self.data().stakeholders_fees.iter() {
            if account != &account_id {
                total_fees += percentage;
            }
        }

        assert!(
            total_fees <= 100u128,
            "TREASURER::ERR_FEE_EXCEEDS_MAXIMUM_VALUE"
        );

        self.data_mut()
            .stakeholders_fees
            .insert(account_id.clone(), new_percentage);

        format! { "The percentage for {} is now {}", account_id, new_percentage}
    }

    /// Returns stakeholders and associated fees
    pub fn get_stakeholders(&self) -> HashMap<AccountId, u128> {
        self.is_owner();
        self.data().stakeholders_fees.clone()
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
    fn test_add_stakeholders() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        let acc0: AccountId = to_account_id("fluxus0");
        let acc1: AccountId = to_account_id("fluxus1");

        let fee0: u128 = 40;
        let fee1: u128 = 50;

        contract.add_stakeholder(acc0.clone(), fee0);
        contract.add_stakeholder(acc1.clone(), fee1);

        let stakeholders_fees: HashMap<AccountId, u128> = contract.get_stakeholders();

        assert_eq!(stakeholders_fees.get(&acc0), Some(&fee0));
        assert_eq!(stakeholders_fees.get(&acc1), Some(&fee1));
    }

    #[test]
    fn test_remove_stakeholders() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        let acc0: AccountId = to_account_id("fluxus0");

        let fee0: u128 = 40;

        contract.add_stakeholder(acc0.clone(), fee0);
        let stakeholders_fees: HashMap<AccountId, u128> = contract.get_stakeholders();
        assert_eq!(stakeholders_fees.len(), 1);

        contract.remove_stakeholder(acc0);
        let stakeholders_fees: HashMap<AccountId, u128> = contract.get_stakeholders();
        assert_eq!(stakeholders_fees.len(), 0);
    }

    #[test]
    #[should_panic]
    fn test_fee_greater_than_one() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        let acc0: AccountId = to_account_id("fluxus0");
        let acc1: AccountId = to_account_id("fluxus1");

        let fee0: u128 = 50;
        let fee1: u128 = 60;

        contract.add_stakeholder(acc0.clone(), fee0);

        // panics because the fee will be above 1
        contract.add_stakeholder(acc1.clone(), fee1);
    }

    #[test]
    fn test_update_fee_percentage() {
        let context = get_context();
        testing_env!(context.build());

        let mut contract = create_contract();

        let acc0: AccountId = to_account_id("fluxus0");
        let acc1: AccountId = to_account_id("fluxus1");

        let fee0: u128 = 50;
        let fee1: u128 = 40;

        contract.add_stakeholder(acc0.clone(), fee0);
        contract.add_stakeholder(acc1.clone(), fee1);

        let stakeholders_fees: HashMap<AccountId, u128> = contract.get_stakeholders();

        assert_eq!(stakeholders_fees.get(&acc0), Some(&fee0));

        let new_fee_percentage = 60u128;

        contract.update_stakeholder_percentage(acc0.clone(), new_fee_percentage);

        let stakeholders_fees: HashMap<AccountId, u128> = contract.get_stakeholders();

        let mut total_fees = 0u128;
        for (_, perc) in stakeholders_fees.iter() {
            total_fees += perc;
        }

        assert_eq!(total_fees, 100u128, "ERR_WRONG_FEE_TOTAL_AMOUNT");

        assert_eq!(
            stakeholders_fees.get(&acc0),
            Some(&new_fee_percentage),
            "ERR_UPDATE_PERCENTAGE"
        );
    }
}
