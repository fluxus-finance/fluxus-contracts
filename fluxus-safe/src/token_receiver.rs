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
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.assert_contract_running();
        
        let token_in = env::predecessor_account_id();
        log!("token_id is: {}",token_in.clone().to_string());

        //self.internal_deposit(&sender_id, &token_in, amount.into());

    
        //self.assert_strategy_is_running(&seed_id);
        let strat_name: String = format!("pembrock@{}", token_in);//leo: Get the right token_in

        let compounder = self.get_strat(&strat_name).pemb_get();

        let amount_in_u128: u128 = amount.into();

        //Check: is the amount sent above or equal the minimum deposit?
        assert!(
            amount_in_u128 >= compounder.seed_min_deposit.into(),
            "ERR_BELOW_MIN_DEPOSIT"
        );

        // initiate stake process
        compounder.stake_on_pembrock();





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
        let caller_id = env::predecessor_account_id();

        let seed_id: String = format!("{}@{}", caller_id, unwrap_token_id(&token_id));
        self.assert_strategy_is_running(&seed_id);

        let compounder = self.get_strat(&seed_id).get();

        let amount_in_u128: u128 = amount.into();

        //Check: is the amount sent above or equal the minimum deposit?
        assert!(
            amount_in_u128 >= compounder.seed_min_deposit.into(),
            "ERR_BELOW_MIN_DEPOSIT"
        );

        // initiate stake process
        compounder.stake(token_id, seed_id, &sender_id, amount_in_u128);

        // TODO: remove this and return promise
        PromiseOrValue::Value(U128(0))
    }
}
