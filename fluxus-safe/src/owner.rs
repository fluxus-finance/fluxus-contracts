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

    /// Args:
    ///   farm_id_str: exchange@pool_id#farm_id
    ///   state: Running, Ended, ...
    pub fn update_compounder_state(
        &mut self,
        farm_id_str: String,
        state: AutoCompounderState,
    ) -> String {
        self.is_owner();

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&token_id.to_string()).get_mut();
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
        assert!(self.is_owner_or_guardians(), "ERR_");
        // TODO: what maximum slippage should be accepted?
        // Should not accept, say, 0 slippage
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder_mut = self.get_strat_mut(&token_id.to_string()).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);
        farm_info_mut.slippage = 100 - new_slippage;

        format!(
            "The current slippage for {} is {}",
            token_id, farm_info_mut.slippage
        )
    }
}
