use crate::*;

use crate::auto_compounder::AutoCompounder;
// use crate::sable
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

// pub(crate) type StratId = String;

/// Generic Strategy, providing wrapper around different implementations of strategies.
/// Allows to add new types of strategies just by adding extra item in the enum
/// without needing to migrate the storage.
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum VersionedStrategy {
    AutoCompounder(AutoCompounder),
    StableAutoCompounder(StableAutoCompounder),
}

impl VersionedStrategy {
    /// Returns Strategy kind.
    pub fn kind(&self) -> String {
        match self {
            VersionedStrategy::AutoCompounder(_) => "AUTO_COMPOUNDER".to_string(),
            VersionedStrategy::StableAutoCompounder(_) => "STABLE_AUTO_COMPOUNDER".to_string(),
        }
    }

    // TODO: impl
    // pub fn get_strategy_id(&self) -> StratId {
    //     match self {
    //         VersionedStrategy::AutoCompounder(strat) => strat.strategy_id.clone(),
    //     }
    // }

    /// update method in order to upgrade strategy
    #[allow(unreachable_patterns)]
    pub fn upgrade(&self) -> Self {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => {
                VersionedStrategy::AutoCompounder(compounder.clone())
            }
            VersionedStrategy::StableAutoCompounder(stable_compounder) => {
                VersionedStrategy::StableAutoCompounder(stable_compounder.clone())
            }
        }
    }

    /// update method in order to upgrade strategy
    #[allow(unreachable_patterns)]
    pub fn need_upgrade(&self) -> bool {
        match self {
            Self::AutoCompounder(_) => false,
            Self::StableAutoCompounder(_) => false,
        }
    }

    // // Return the farm or liquidity pool or token( other kinds of strategy) this strategy accepts
    // #[allow(unreachable_patterns)]
    // pub fn get_token_id(&self) -> String {
    //     match self {
    //         VersionedStrategy::AutoCompounder(strat) => strat.farm_id.clone(),
    //         _ => unimplemented!(),
    //     }
    // }

    #[allow(unreachable_patterns)]
    pub fn get_compounder(self) -> AutoCompounder {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn get_compounder_ref(&self) -> &AutoCompounder {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn get_compounder_mut(&mut self) -> &mut AutoCompounder {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn get_stable_compounder(self) -> StableAutoCompounder {
        match self {
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder,
            _ => unimplemented!(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn get_stable_compounder_ref(&self) -> &StableAutoCompounder {
        match self {
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder,
            _ => unimplemented!(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn get_stable_compounder_mut(&mut self) -> &mut StableAutoCompounder {
        match self {
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder,
            _ => unimplemented!(),
        }
    }
}

impl Contract {
    pub fn get_strat(&self, seed_id: &str) -> VersionedStrategy {
        let strat = self
            .data()
            .strategies
            .get(seed_id)
            .expect(ERR21_TOKEN_NOT_REG);

        if strat.need_upgrade() {
            strat.upgrade()
        } else {
            strat.clone()
        }
    }

    pub fn get_strat_mut(&mut self, seed_id: &str) -> &mut VersionedStrategy {
        let strat = self
            .data_mut()
            .strategies
            .get_mut(seed_id)
            .expect(ERR21_TOKEN_NOT_REG);

        if strat.need_upgrade() {
            strat.upgrade();
            strat
        } else {
            strat
        }
    }
}
