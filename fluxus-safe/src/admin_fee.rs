use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId};
use std::collections::HashMap;

const MAX_STRAT_CREATOR_FEE: u128 = 20;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountFee {
    /// address id
    pub account_id: AccountId,
    /// fee percentage
    pub fee_percentage: u128,
    /// current amount earned, stored to be used if tx fails
    pub current_amount: u128,
}

impl AccountFee {
    pub fn new(acc_id: AccountId, fee: u128) -> Self {
        assert!(
            (0..MAX_STRAT_CREATOR_FEE + 1).contains(&fee),
            "ERR_FEE_NOT_VALID"
        );

        AccountFee {
            account_id: acc_id,
            fee_percentage: fee,
            current_amount: 0u128,
        }
    }
}

const MAX_CONTRIBUTOR_FEE: u128 = 20;
const MAX_PROTOCOL_FEE: u128 = 20;

/// Maintain information about fees.
/// Maps receiver address to percentage
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AdminFees {
    /// Protocol Total fees of the running strategy
    pub strategy_fee: u128,
    /// Fees earned by the creator of the running strategy
    pub strat_creator: AccountFee,
    /// Fee percentage earned by sentries
    pub sentries_fee: u128,
    /// Fees earned by users that interact with the harvest method - TODO: is this really needed seems a bit of storage waste
    pub sentries: HashMap<AccountId, u128>,
}

impl AdminFees {
    pub fn new(strat_creator: AccountFee, sentries_fee: u128, strategy_fee: u128) -> Self {
        assert!(
            strat_creator.fee_percentage + sentries_fee <= MAX_CONTRIBUTOR_FEE,
            "ERR: fees too high"
        );
        assert!(strategy_fee <= MAX_PROTOCOL_FEE, "ERR: fees too high");
        AdminFees {
            strategy_fee,
            strat_creator,
            sentries_fee,
            sentries: HashMap::new(),
        }
    }
}
