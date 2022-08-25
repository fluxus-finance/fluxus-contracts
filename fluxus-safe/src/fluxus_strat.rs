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

impl VersionedStrategy {
    pub fn stake(
        &self,
        token_id: String,
        seed_id: String,
        account_id: &AccountId,
        shares: u128,
    ) -> Promise {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => {
                compounder.stake(token_id, seed_id, account_id, shares)
            }
            VersionedStrategy::StableAutoCompounder(stable_compounder) => {
                stable_compounder.stake(token_id, seed_id, account_id, shares)
            }
        }
    }

    pub fn unstake(
        &self,
        seed_id: String,
        receiver_id: AccountId,
        withdraw_amount: u128,
        user_fft_shares: u128,
    ) -> Promise {
        log!(
            "{} is trying to withdrawal {}",
            receiver_id,
            withdraw_amount
        );
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder.unstake(
                wrap_mft_token_id(&compounder.pool_id.to_string()),
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
            ),
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder
                .unstake(
                    wrap_mft_token_id(&stable_compounder.pool_id.to_string()),
                    seed_id,
                    receiver_id,
                    withdraw_amount,
                    user_fft_shares,
                ),
        }
    }

    pub fn harvest_proxy(&mut self, farm_id_str: String, treasure: AccountFee) -> Promise {
        let (_, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        match self {
            VersionedStrategy::AutoCompounder(compounder) => {
                let farm_info = compounder.get_farm_info(&farm_id);

                assert_strategy_not_cleared(farm_info.state);

                match farm_info.cycle_stage {
                    AutoCompounderCycle::ClaimReward => compounder.claim_reward(farm_id_str),
                    AutoCompounderCycle::Withdrawal => {
                        compounder.withdraw_of_reward(farm_id_str, treasure.current_amount)
                    }
                    AutoCompounderCycle::Swap => {
                        compounder.autocompounds_swap(farm_id_str, treasure)
                    }
                    AutoCompounderCycle::Stake => {
                        compounder.autocompounds_liquidity_and_stake(farm_id_str)
                    }
                }
            }
            VersionedStrategy::StableAutoCompounder(stable_compounder) => {
                let farm_info = stable_compounder.get_farm_info(&farm_id);

                assert_strategy_not_cleared(farm_info.state);

                match farm_info.cycle_stage {
                    AutoCompounderCycle::ClaimReward => stable_compounder.claim_reward(farm_id_str),
                    AutoCompounderCycle::Withdrawal => {
                        stable_compounder.withdraw_of_reward(farm_id_str, treasure.current_amount)
                    }
                    AutoCompounderCycle::Swap => {
                        stable_compounder.autocompounds_swap(farm_id_str, treasure)
                    }
                    AutoCompounderCycle::Stake => {
                        stable_compounder.autocompounds_liquidity_and_stake(farm_id_str)
                    }
                }
            }
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
