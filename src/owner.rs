use crate::*;

/// Internal methods implementation.
#[near_bindgen]
impl Contract {
    pub fn update_contract_state(&mut self, state: RunningState) -> String {
        self.is_owner();
        self.state = state;
        format!("{} is {}", env::current_account_id(), self.state)
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

        for (token_id, strat) in self.strategies.clone() {
            info.push(strat);
        }

        info
    }
}
