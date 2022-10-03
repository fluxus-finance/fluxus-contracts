use crate::auto_compounder::AutoCompounder;
use crate::jumbo_auto_compounder::JumboAutoCompounder;
use crate::pembrock_auto_compounder::PembrockAutoCompounder;
use crate::*;

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
#[allow(clippy::enum_variant_names)]
pub enum VersionedStrategy {
    AutoCompounder(AutoCompounder),
    StableAutoCompounder(StableAutoCompounder),
    PembrockAutoCompounder(PembrockAutoCompounder),
    JumboAutoCompounder(JumboAutoCompounder),
}

impl VersionedStrategy {
    /// Returns Strategy kind.
    pub fn kind(&self) -> String {
        match self {
            VersionedStrategy::AutoCompounder(_) => "REF_REGULAR".to_string(),
            VersionedStrategy::StableAutoCompounder(_) => "REF_STABLE".to_string(),
            VersionedStrategy::JumboAutoCompounder(_) => "JUMBO_REGULAR".to_string(),
            VersionedStrategy::PembrockAutoCompounder(_) => "PEMBROCK".to_string(),
        }
    }

    // TODO: impl
    // pub fn get_strategy_id(&self) -> StratId {
    //     match self {
    //         VersionedStrategy::AutoCompounder(strat) => strat.strategy_id.clone(),
    //     }
    // }

    /// Update method in order to upgrade strategy
    #[allow(unreachable_patterns)]
    pub fn upgrade(&self) -> Self {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => {
                VersionedStrategy::AutoCompounder(compounder.clone())
            }
            VersionedStrategy::StableAutoCompounder(stable_compounder) => {
                VersionedStrategy::StableAutoCompounder(stable_compounder.clone())
            }

            VersionedStrategy::PembrockAutoCompounder(compounder) => {
                VersionedStrategy::PembrockAutoCompounder(compounder.clone())
            }
            VersionedStrategy::JumboAutoCompounder(jumbo_compounder) => {
                VersionedStrategy::JumboAutoCompounder(jumbo_compounder.clone())
            }
        }
    }

    /// Return true if it is necessary to update the compounder.
    #[allow(unreachable_patterns)]
    pub fn need_upgrade(&self) -> bool {
        match self {
            Self::AutoCompounder(_) => false,
            Self::StableAutoCompounder(_) => false,
            Self::PembrockAutoCompounder(_) => false,
            Self::JumboAutoCompounder(_) => false,
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

    /// Return the compounder structure.
    #[allow(unreachable_patterns)]
    pub fn get_compounder(self) -> AutoCompounder {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the compounder structure as a reference.
    #[allow(unreachable_patterns)]
    pub fn get_compounder_ref(&self) -> &AutoCompounder {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the compounder structure as a mutable reference.
    #[allow(unreachable_patterns)]
    pub fn get_compounder_mut(&mut self) -> &mut AutoCompounder {
        match self {
            VersionedStrategy::AutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Stable_compounder structure.
    #[allow(unreachable_patterns)]
    pub fn get_stable_compounder(self) -> StableAutoCompounder {
        match self {
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Stable_compounder structure as a reference.
    #[allow(unreachable_patterns)]
    pub fn get_stable_compounder_ref(&self) -> &StableAutoCompounder {
        match self {
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Stable_compounder structure as a mutable reference.
    #[allow(unreachable_patterns)]
    pub fn get_stable_compounder_mut(&mut self) -> &mut StableAutoCompounder {
        match self {
            VersionedStrategy::StableAutoCompounder(stable_compounder) => stable_compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Jumbo_compounder structure.
    #[allow(unreachable_patterns)]
    pub fn get_jumbo(self) -> JumboAutoCompounder {
        match self {
            VersionedStrategy::JumboAutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Jumbo_compounder structure as a reference.
    #[allow(unreachable_patterns)]
    pub fn get_jumbo_ref(&self) -> &JumboAutoCompounder {
        match self {
            VersionedStrategy::JumboAutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Jumbo_compounder structure as a mutable reference.
    #[allow(unreachable_patterns)]
    pub fn get_jumbo_mut(&mut self) -> &mut JumboAutoCompounder {
        match self {
            VersionedStrategy::JumboAutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }
}

impl VersionedStrategy {

    /// Call the stake function for an auto_compounder.
    /// # Parameters example: 
    /// token_id: :1,
    /// seed_id: exchange@seed_id,
    ///  account_id: account.testnet,
    ///  shares: 100000000,
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
            VersionedStrategy::PembrockAutoCompounder(pemb_compounder) => {
                // pemb_compounder.stake_on_pembrock(account_id, shares, account_id, shares)
                unimplemented!()
            }
            VersionedStrategy::JumboAutoCompounder(jumbo_compounder) => {
                jumbo_compounder.stake(token_id, seed_id, account_id, shares)
            }
        }
    }

    /// Call the unstake function for an auto_compounder.
    /// # Parameters example: 
    ///  seed_id: exchange@seed_id,
    ///  receiver_id: account.testnet,
    ///  withdraw_amount: 100000000,
    ///  user_fft_shares: 100000000,
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
            VersionedStrategy::PembrockAutoCompounder(stable_compounder) => unimplemented!(),
            VersionedStrategy::JumboAutoCompounder(jumbo_compounder) => jumbo_compounder.unstake(
                wrap_mft_token_id(&jumbo_compounder.pool_id.to_string()),
                seed_id,
                receiver_id,
                withdraw_amount,
                user_fft_shares,
            ),
        }
    }

    /// Call the harvest function for some compounder based on it`s type (stable, jumbo...).
    /// # Parameters example: 
    ///  farm_id_str: exchange_contract.testnet@pool_id#farm_id,
    ///  strat_name: pembrock@token_name,
    ///  treasure: { account_id: treasure.testnet, "fee_percentage": 5, "current_amount" : 0 },
    pub fn harvest_proxy(
        &mut self,
        farm_id_str: String,
        strat_name: String,
        treasure: AccountFee,
    ) -> PromiseOrValue<u128> {
        let mut farm_id: String = "".to_string();
        if farm_id_str != *"" {
            (_, _, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        }
        match self {
            VersionedStrategy::AutoCompounder(compounder) => {
                let farm_info = compounder.get_farm_info(&farm_id);

                assert_strategy_not_cleared(farm_info.state);

                match farm_info.cycle_stage {
                    AutoCompounderCycle::ClaimReward => {
                        PromiseOrValue::Promise(compounder.claim_reward(farm_id_str))
                    }
                    AutoCompounderCycle::Withdrawal => PromiseOrValue::Promise(
                        compounder.withdraw_of_reward(farm_id_str, treasure.current_amount),
                    ),
                    AutoCompounderCycle::Swap => PromiseOrValue::Promise(
                        compounder.autocompounds_swap(farm_id_str, treasure),
                    ),
                    AutoCompounderCycle::Stake => PromiseOrValue::Promise(
                        compounder.autocompounds_liquidity_and_stake(farm_id_str),
                    ),
                }
            }
            VersionedStrategy::StableAutoCompounder(stable_compounder) => {
                let farm_info = stable_compounder.get_farm_info(&farm_id);

                assert_strategy_not_cleared(farm_info.state);

                match farm_info.cycle_stage {
                    AutoCompounderCycle::ClaimReward => {
                        PromiseOrValue::Promise(stable_compounder.claim_reward(farm_id_str))
                    }
                    AutoCompounderCycle::Withdrawal => PromiseOrValue::Promise(
                        stable_compounder.withdraw_of_reward(farm_id_str, treasure.current_amount),
                    ),
                    AutoCompounderCycle::Swap => {
                        stable_compounder.autocompounds_swap(farm_id_str, treasure)
                    }
                    AutoCompounderCycle::Stake => PromiseOrValue::Promise(
                        stable_compounder.autocompounds_liquidity_and_stake(farm_id_str),
                    ),
                }
            }
            VersionedStrategy::JumboAutoCompounder(jumbo_compounder) => {
                let farm_info = jumbo_compounder.get_jumbo_farm_info(&farm_id);
                match farm_info.cycle_stage {
                    JumboAutoCompounderCycle::ClaimReward => {
                        PromiseOrValue::Promise(jumbo_compounder.claim_reward(farm_id_str))
                    }
                    JumboAutoCompounderCycle::Withdrawal => PromiseOrValue::Promise(
                        jumbo_compounder.withdraw_of_reward(farm_id_str, treasure.current_amount),
                    ),
                    JumboAutoCompounderCycle::SwapToken1 => PromiseOrValue::Promise(
                        jumbo_compounder.autocompounds_swap(farm_id_str, treasure),
                    ),
                    JumboAutoCompounderCycle::SwapToken2 => PromiseOrValue::Promise(
                        jumbo_compounder.autocompounds_swap_second_token(farm_id_str),
                    ),
                    JumboAutoCompounderCycle::Stake => PromiseOrValue::Promise(
                        jumbo_compounder.autocompounds_liquidity_and_stake(farm_id_str),
                    ),
                }
            }
            VersionedStrategy::PembrockAutoCompounder(pemb_compounder) => {
                match pemb_compounder.cycle_stage {
                    PembAutoCompounderCycle::ClaimReward => {
                        PromiseOrValue::Promise(pemb_compounder.claim_reward(strat_name))
                    }
                    PembAutoCompounderCycle::SwapAndLend => {
                        PromiseOrValue::Promise(pemb_compounder.swap_and_lend(strat_name))
                    }
                }
            }
        }
    }

    /// Return the Pembrock compounder structure.
    #[allow(unreachable_patterns)]
    pub fn pemb_get(self) -> PembrockAutoCompounder {
        match self {
            VersionedStrategy::PembrockAutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Pembrock compounder structure as a reference.
    #[allow(unreachable_patterns)]
    pub fn pemb_get_ref(&self) -> &PembrockAutoCompounder {
        match self {
            VersionedStrategy::PembrockAutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }

    /// Return the Pembrock compounder structure as a mutable reference.
    #[allow(unreachable_patterns)]
    pub fn pemb_get_mut(&mut self) -> &mut PembrockAutoCompounder {
        match self {
            VersionedStrategy::PembrockAutoCompounder(compounder) => compounder,
            _ => unimplemented!(),
        }
    }
}

impl Contract {
       
    /// Return the VersionedStrategy structure.
    /// # Parameters example: 
    ///  seed_id: exchange@seed_id,
    pub fn get_strat(&self, seed_id: &str) -> VersionedStrategy {
        let strat = self
            .data()
            .strategies
            .get(seed_id)
            .expect(ERR42_TOKEN_NOT_REG);

        if strat.need_upgrade() {
            strat.upgrade()
        } else {
            strat.clone()
        }
    }

    /// Return the VersionedStrategy structure as a mutable reference.
    /// # Parameters example: 
    ///  seed_id: exchange@seed_id,
    pub fn get_strat_mut(&mut self, seed_id: &str) -> &mut VersionedStrategy {
        let strat = self
            .data_mut()
            .strategies
            .get_mut(seed_id)
            .expect(ERR42_TOKEN_NOT_REG);

        if strat.need_upgrade() {
            strat.upgrade();
            strat
        } else {
            strat
        }
    }

    /// Return the Pembrock VersionedStrategy structure.
    /// # Parameters example: 
    ///  seed_id: exchange@seed_id,
    pub fn pemb_get_strat(&self, seed_id: &str) -> VersionedStrategy {
        let strat = self
            .data()
            .strategies
            .get(seed_id)
            .expect(ERR42_TOKEN_NOT_REG);

        if strat.need_upgrade() {
            strat.upgrade()
        } else {
            strat.clone()
        }
    }
    
    /// Return the Pembrock VersionedStrategy structure as a mutable reference.
    /// # Parameters example: 
    ///  seed_id: exchange@seed_id,
    pub fn pemb_get_strat_mut(&mut self, seed_id: &str) -> &mut VersionedStrategy {
        let strat = self
            .data_mut()
            .strategies
            .get_mut(seed_id)
            .expect(ERR42_TOKEN_NOT_REG);

        if strat.need_upgrade() {
            strat.upgrade();
            strat
        } else {
            strat
        }
    }
}
