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
    /// # Parameters example:
    ///   sender_id: account.testnet,
    ///   amount: U128(100000000),
    ///   msg: ""
    #[allow(unreachable_code)]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.assert_contract_running();

        let token_in = env::predecessor_account_id();
        let token_in_id = get_token_id(token_in.to_string());
        log!("token_id is: {}", token_in_id.to_string());

        // TODO: assert pembrock strat is running
        //self.assert_strategy_is_running(&seed_id);
        let strat_name: String = format!("pembrock@{}", token_in_id);

        let compounder = self.pemb_get_strat(&strat_name).pemb_get();

        // initiate stake process
        let amount_in_u128: u128 = amount.into();
        compounder.stake_on_pembrock(&sender_id, amount_in_u128, strat_name);

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
    /// # Parameters example:
    ///   token_id: :17
    ///   sender_id: account.testnet,
    ///   amount: U128(100000000),
    ///   msg: ""
    #[allow(unused)]
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let caller_id = env::predecessor_account_id();

        let seed_id: String = format!("{}@{}", caller_id, unwrap_token_id(&token_id));
        self.assert_strategy_is_running(&seed_id);

        let strat = self.get_strat(&seed_id);

        PromiseOrValue::Promise(strat.stake(token_id, seed_id, &sender_id, amount.0))
    }
}
