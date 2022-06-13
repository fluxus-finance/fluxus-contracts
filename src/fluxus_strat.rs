use crate::auto_compounder::AutoCompounder;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

pub(crate) type StratId = String;

/// Generic Strategy, providing wrapper around different implementations of strategies.
/// Allows to add new types of strategies just by adding extra item in the enum
/// without needing to migrate the storage.
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Strategy {
    AutoCompounder(AutoCompounder),
}

impl Strategy {
    /// Returns Strategy kind.
    pub fn kind(&self) -> String {
        match self {
            Strategy::AutoCompounder(_) => "AUTO_COMPOUNDER".to_string(),
        }
    }

    // TODO: impl
    // pub fn strategy_cycle(&mut self) {
    //     match self {
    //         Strategy::AutoCompounder(strat) => {
    //             strat.strategy_cycle();
    //         }
    //     }
    // }

    // TODO: impl
    // pub fn get_strategy_id(&self) -> StratId {
    //     match self {
    //         Strategy::AutoCompounder(strat) => strat.strategy_id.clone(),
    //     }
    // }

    // Return the farm or liquidity pool or token( other kinds of strategy) this strategy accepts
    pub fn get_token_id(&self) -> String {
        match self {
            Strategy::AutoCompounder(strat) => strat.farm_id.clone(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get(self) -> AutoCompounder {
        match self {
            // VersionedFarmer::V101(farmer) => farmer,
            // AutoCompounder => ,
            Strategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref(&self) -> &AutoCompounder {
        match self {
            Strategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_mut(&mut self) -> &mut AutoCompounder {
        match self {
            // VersionedFarmer::V101(farmer) => farmer,
            // AutoCompounder => ,
            Strategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }
}
