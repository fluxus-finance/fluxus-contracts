use crate::*;

/// Internal methods implementation.
#[near_bindgen]
impl Contract {
    pub fn update_contract_state(&mut self, state: RunningState) -> String {
        self.is_owner();
        self.state = state;
        format!("{} is {:#?}", env::current_account_id(), self.state)
    }

    pub fn update_exchange_contract(&mut self, contract_id: AccountId) {
        self.is_owner();
        self.exchange_contract_id = contract_id;
    }

    pub fn update_farm_contract(&mut self, contract_id: AccountId) {
        self.is_owner();
        self.farm_contract_id = contract_id;
    }

    /// Returns allowed_accounts
    pub fn get_allowed_accounts(&self) -> Vec<AccountId> {
        self.is_owner();
        self.allowed_accounts.clone()
    }

    /// Returns all strategies without filtering
    pub fn get_strats_info(self) -> Vec<Strategy> {
        self.is_owner();

        let mut info: Vec<Strategy> = Vec::new();

        for (_, strat) in self.strategies {
            info.push(strat);
        }

        info
    }

    pub fn update_compounder_state(
        &mut self,
        token_id: String,
        state: AutoCompounderState,
    ) -> String {
        self.is_owner();

        let strat = self
            .strategies
            .get_mut(&token_id)
            .expect(ERR21_TOKEN_NOT_REG);
        let compounder = strat.get_mut();

        if compounder.state != state {
            compounder.state = state;
        }

        format!("The current state is {:#?}", compounder.state)
    }

    /// Extend guardians. Only can be called by owner.
    #[payable]
    pub fn extend_guardians(&mut self, guardians: Vec<AccountId>) {
        assert_one_yocto();
        self.is_owner();
        for guardian in guardians {
            self.guardians.insert(&guardian);
        }
    }

    /// Remove guardians. Only can be called by owner.
    #[payable]
    pub fn remove_guardians(&mut self, guardians: Vec<AccountId>) {
        assert_one_yocto();
        self.is_owner();
        for guardian in guardians {
            self.guardians.remove(&guardian);
        }
    }

    pub fn is_owner_or_guardians(&self) -> bool {
        env::predecessor_account_id() == self.owner_id
            || self.guardians.contains(&env::predecessor_account_id())
    }
}
