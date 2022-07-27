use crate::*;

/// Internal methods implementation.
#[near_bindgen]
impl Contract {
    pub fn update_contract_state(&mut self, state: RunningState) -> String {
        self.is_owner();
        self.data_mut().state = state;
        format!("{} is {:#?}", env::current_account_id(), self.data().state)
    }

    pub fn update_exchange_contract(&mut self, contract_id: AccountId) {
        self.is_owner();
        self.data_mut().exchange_contract_id = contract_id;
    }

    pub fn update_farm_contract(&mut self, contract_id: AccountId) {
        self.is_owner();
        self.data_mut().farm_contract_id = contract_id;
    }

    pub fn update_treasure_contract(&mut self, contract_id: AccountId) {
        self.is_owner();
        self.data_mut().treasury.account_id = contract_id;
    }

    /// Returns allowed_accounts
    pub fn get_allowed_accounts(&self) -> Vec<AccountId> {
        self.is_owner();
        self.data().allowed_accounts.clone()
    }

    /// Returns all strategies without filtering
    pub fn get_strats_info(self) -> Vec<VersionedStrategy> {
        self.is_owner();

        let mut info: Vec<VersionedStrategy> = Vec::new();

        // TODO: should exist a `get_strategies` and upgrade everything at once if so?
        for (_, strat) in self.data().strategies.clone() {
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
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR1_TOKEN_NOT_REG);
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

    pub fn is_owner_or_guardians(&self) -> bool {
        env::predecessor_account_id() == self.data().owner_id
            || self
                .data()
                .guardians
                .contains(&env::predecessor_account_id())
    }

    /// Update slippage for given token_id
    pub fn update_strat_slippage(&mut self, token_id: String, new_slippage: u128) -> String {
        assert!(self.is_owner_or_guardians(), "ERR_");
        // TODO: what maximum slippage should be accepted?
        // Should not accept, say, 0 slippage
        let strat = self
            .data_mut()
            .strategies
            .get_mut(&token_id)
            .expect(ERR1_TOKEN_NOT_REG);

        let compounder = strat.get_mut();
        compounder.slippage = 100 - new_slippage;

        format!(
            "The current slippage for {} is {}",
            token_id, compounder.slippage
        )
    }
}
