use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{ext_contract, PromiseOrValue};

use crate::*;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    /// Alternative to deposit + execute actions call.
    Execute {
        referral_id: Option<AccountId>,
        // List of sequential actions.
        //actions: Vec<Action>,
    },
}

#[ext_contract(ext_self)]
pub trait RefExchange {
    fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );
    fn metadata(&mut self);
}

#[ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_multi_fungible_token)]
pub trait MultiFungibleToken {
    fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is either "" for deposit or `TokenReceiverMessage`.
    #[allow(unreachable_code)]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        _msg: String,
    ) -> PromiseOrValue<U128> {
        self.assert_contract_running();
        ext_self::metadata(
            env::current_account_id(),
            0,                      // yocto NEAR to attach
            Gas(5_000_000_000_000), // gas to attach
        );
        let token_in = env::predecessor_account_id();
        self.internal_deposit(&sender_id, &token_in, amount.into());
        PromiseOrValue::Value(U128(0))
    }
}

pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

/// seed token deposit
#[near_bindgen]
impl MFTTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    #[allow(unused)]
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.assert_strategy_running(token_id.clone());

        //Check: Is the token_id the vault's pool_id? If is not, send it back
        let strat = self.get_strat(&token_id);

        let compounder = strat.get();

        let amount_in_u128: u128 = amount.into();

        //Check: is the amount sent above or equal the minimum deposit?
        assert!(
            amount_in_u128 >= compounder.seed_min_deposit.into(),
            "ERR_BELOW_MIN_DEPOSIT"
        );

        // initiate stake process
        self.stake(token_id, &sender_id, amount_in_u128);

        PromiseOrValue::Value(U128(0))
    }
}
