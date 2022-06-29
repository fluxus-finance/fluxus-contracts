use crate::*;

#[ext_contract(ext_share_token_receiver)]
pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}
#[ext_contract(ext_self)]
trait MFTTokenResolver {
    fn mft_resolve_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}


pub const NO_DEPOSIT: u128 = 0;
pub const GAS_FOR_RESOLVE_TRANSFER: u128 = 20_000_000_000_000_u128;
pub const GAS_FOR_FT_TRANSFER_CALL: u128 = 25_000_000_000_000_u128 + GAS_FOR_RESOLVE_TRANSFER;

#[near_bindgen]
impl Contract {
    
    pub fn users_share_amount(&mut self, seed_id: String, user: String) -> u128 {
        let mut temp = HashMap::new();
        temp.insert(user.clone(), 0_u128);
        let sla = (*self.data().user_shares_by_seed_id.get(&seed_id).unwrap_or(& temp)).get(&user).unwrap_or(&0_u128)
        ;
        *sla
    }

    pub fn total_supply_amount(&mut self, seed_id: String) -> u128 {
        let result: u128 = *self.data_mut().total_supply_by_seed_id.get(&seed_id).unwrap_or(&0_u128);
        result
    }

    pub fn mint_function(&mut self, seed_id: String, balance: u128, user: String) -> u128{

        //Add balance to the user for this seed
        let old_amount: u128 = self.users_share_amount(seed_id.clone(), user.clone());
        let new_balance = old_amount+balance;
        let mut hash_temp = HashMap::new();
        hash_temp.insert(user, new_balance);
        self.data_mut().user_shares_by_seed_id.insert(seed_id.clone(), hash_temp );

        //Add balance to the total supply
        let old_total = self.total_supply_amount(seed_id.clone());
        self.data_mut().total_supply_by_seed_id.insert(seed_id, old_total + balance);

        //Returning the new balance
        new_balance
    }

    pub fn burn_function(&mut self, seed_id: String, balance: u128, user: String) -> u128{
        //Sub balance to the user for this seed
        let old_amount: u128 = self.users_share_amount(seed_id.clone(), user.clone());

        assert!(old_amount >= balance);

        let new_balance = old_amount - balance;
        let mut hash_temp = HashMap::new();
        hash_temp.insert(user, new_balance);
        self.data_mut().user_shares_by_seed_id.insert(seed_id.clone(), hash_temp );

        //Sub balance to the total supply
        let old_total = self.total_supply_amount(seed_id.clone());
        self.data_mut().total_supply_by_seed_id.insert(seed_id, old_total - balance);

        //Returning the new balance
        new_balance
    }

    
    //Functions for transference
        
    /// Transfer one of internal tokens: LP or balances.
    /// The token_id is the seed_id
    #[payable]
    pub fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: String,
        amount: U128,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        self.assert_contract_running();
        self.internal_mft_transfer(
            token_id,
            env::predecessor_account_id().to_string(),
            receiver_id,
            amount.0,
            memo,
        );
    }/* */

    fn internal_mft_transfer(
        &mut self,
        token_id: String,
        sender_id: String,
        receiver_id: String,
        amount: u128,
        memo: Option<String>,
    ) {
        // [AUDIT_07]
        assert_ne!(sender_id, receiver_id, "{}", ERR33_TRANSFER_TO_SELF);
            
        //let  strat = self.data_mut().strategies.get(&token_id).expect("ERR_NO_STRATEGY");
        self.share_transfer(token_id.clone(), sender_id.clone(), receiver_id.clone(), amount.clone());



        //self.data_mut().strategies.insert(token_id, *strat);
        log!(
            "Transfer shares {} pool: {} from {} to {}",
            token_id,
            amount,
            sender_id,
            receiver_id
        );
        
            
        
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
    }

    pub fn share_transfer(&mut self, seed_id: String, sender_id: String, receiver_id: String, amount: u128) {

        let old_amount: u128 = self.users_share_amount(seed_id.clone(), sender_id.clone());
        assert!(old_amount>=amount);

        let new_balance = old_amount - amount;
        let mut hash_temp = HashMap::new();
        hash_temp.insert(sender_id, new_balance);
        self.data_mut().user_shares_by_seed_id.insert(seed_id.clone(), hash_temp );
        

        let old_amount: u128 = self.users_share_amount(seed_id.clone(), receiver_id.clone());
        let new_balance = old_amount + amount;
        let mut hash_temp = HashMap::new();
        hash_temp.insert(receiver_id, new_balance);
        self.data_mut().user_shares_by_seed_id.insert(seed_id.clone(), hash_temp );
    }

    /* 
    #[payable]
    pub fn mft_transfer_call(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        self.assert_contract_running();
        let sender_id = env::predecessor_account_id();
        self.internal_mft_transfer(
            token_id.clone(),
            sender_id.to_string(),
            receiver_id.clone().to_string(),
            amount.0,
            memo,
        );
        ext_share_token_receiver::mft_on_transfer(
            token_id.clone(),
            sender_id.clone(),
            amount,
            msg,
            receiver_id.clone(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::mft_resolve_transfer(
            token_id,
            sender_id,
            receiver_id.into(),
            amount,
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }*/

    /// Returns how much was refunded back to the sender.
    /// If sender removed account in the meantime, the tokens are sent to the owner account.
    /// Tokens are never burnt.
    #[private]
    pub fn mft_resolve_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: &AccountId,
        amount: U128,
    ) -> U128 {
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount.0, unused_amount.0)
                } else {
                    amount.0
                }
            }
            PromiseResult::Failed => amount.0,
        };/*
        if unused_amount > 0 {
            
            let receiver_balance = self.internal_mft_balance(token_id.clone(), &receiver_id);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                // If sender's account was deleted, we assume that they have also withdrew all the liquidity from pools.
                // Funds are sent to the owner account.
                let refund_to = if self.accounts.get(&sender_id).is_some() {
                    sender_id
                } else {
                    self.owner_id.clone()
                };
                self.internal_mft_transfer(token_id, &receiver_id, &refund_to, refund_amount, None);
            } 
        }*/
        U128(unused_amount)
    }


}

