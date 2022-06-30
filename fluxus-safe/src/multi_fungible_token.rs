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
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(20_000_000_000_000);
pub const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas(45_000_000_000_000);

#[near_bindgen]
impl Contract {
    
    ///Return the u128 amount of an user for an specific uxu_share (ref lp token).
    pub fn users_share_amount(&mut self, uxu_share: String, user: String) -> u128 {
        let mut temp = HashMap::new();
        temp.insert(user.clone(), 0_u128);
        let sla = (*self.data().users_balance_by_uxu_share.get(&uxu_share).unwrap_or(& temp)).get(&user).unwrap_or(&0_u128)
        ;
        *sla
    }

    ///Register a seed into the users_balance_by_uxu_share
    pub fn register_seed(&mut self, uxu_share: String) {
        let mut temp = HashMap::new();
        temp.insert("".to_string(), 0_u128);
        self.data_mut().users_balance_by_uxu_share.insert(uxu_share, temp);
        
    }

    ///Return users_balance of a specific uxu_share
    #[inline]
    pub fn users_share_map_by_uxu_share(&self, uxu_share: String)  -> HashMap<String, u128> {
        let temp = self.data().users_balance_by_uxu_share.get(&uxu_share).unwrap();
        self.do_clone(temp)
    }

    ///Clone a HashMap<String, u128>
    pub fn do_clone(&self, data: &HashMap<String,u128>) -> HashMap<String, u128> {
        data.clone()
    } 

    ///Return the total_supply of an specific uxu_share (ref lp token). 
    pub fn total_supply_amount(&mut self, uxu_share: String) -> u128 {
        let result: u128 = *self.data_mut().total_supply_by_uxu_share.get(&uxu_share).unwrap_or(&0_u128);
        result
    }

    ///Assigns a uxu_share value to an user for a specific uxu_share (ref lp token)
    /// and increment the total_supply of this seed's uxu_share.
    /// It returns the user's new balance.
    pub fn mft_mint(&mut self, uxu_share: String, balance: u128, user: String) -> u128{

        //Add balance to the user for this seed
        let old_amount: u128 = self.users_share_amount(uxu_share.clone(), user.clone());
        let new_balance = old_amount+balance;
        log!("{} + {} = new_balance {}", old_amount, balance, new_balance);
        let mut hash_temp = self.users_share_map_by_uxu_share(uxu_share.clone());

        hash_temp.insert(user, new_balance);
        self.data_mut().users_balance_by_uxu_share.insert(uxu_share.clone(), hash_temp);

        //Add balance to the total supply
        let old_total = self.total_supply_amount(uxu_share.clone());
        self.data_mut().total_supply_by_uxu_share.insert(uxu_share,  old_total + balance);

        //Returning the new balance
        new_balance
    }

    ///Burn uxu_share value for an user in a specific uxu_share (ref lp token)
    /// and decrement the total_supply of this seed's uxu_share.
    /// It returns the user's new balance.
    pub fn mft_burn(&mut self, uxu_share: String, balance: u128, user: String) -> u128{
        //Sub balance to the user for this seed
        let old_amount: u128 = self.users_share_amount(uxu_share.clone(), user.clone());
        assert!(old_amount >= balance);
        let new_balance = old_amount - balance;
        log!("{} - {} = new_balance {}", old_amount, balance, new_balance);
        let mut hash_temp = self.users_share_map_by_uxu_share(uxu_share.clone());
        hash_temp.insert(user, new_balance);
        self.data_mut().users_balance_by_uxu_share.insert(uxu_share.clone(), hash_temp );

        //Sub balance to the total supply
        let old_total = self.total_supply_amount(uxu_share.clone());
        self.data_mut().total_supply_by_uxu_share.insert(uxu_share, old_total - balance);

        //Returning the new balance
        new_balance
    }
        
    /// Transfer uxu_shares internally (user for user).
    /// Token_id is a specific uxu_share.
    #[payable]
    pub fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: String,
        amount: U128,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        log!("{}",env::predecessor_account_id().to_string());
        self.assert_contract_running();
        self.internal_mft_transfer(
            token_id,
            env::predecessor_account_id().to_string(),
            receiver_id,
            amount.0,
            memo,
        );
    }

    fn internal_mft_transfer(
        &mut self,
        token_id: String,
        sender_id: String,
        receiver_id: String,
        amount: u128,
        memo: Option<String>,
    ) {

        assert_ne!(sender_id, receiver_id, "{}", ERR33_TRANSFER_TO_SELF);
        self.share_transfer(token_id.clone(), sender_id.clone(), receiver_id.clone(), amount);

        log!(
            "Transfer shares {}: {} from {} to {}",
            token_id,
            amount,
            sender_id,
            receiver_id
        );
        
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
    }

    pub fn share_transfer(&mut self, uxu_share: String, sender_id: String, receiver_id: String, amount: u128) {

        log!("{} and {}", sender_id, uxu_share);
        let old_amount: u128 = self.users_share_amount(uxu_share.clone(), sender_id.clone());
        log!("{} > = {}", old_amount, amount);
        assert!(old_amount>=amount);
        log!("{} - {}", old_amount, amount);
        let new_balance = old_amount - amount;
        log!("{} + {} = new_balance {}", old_amount, amount, new_balance);

        let mut hash_temp = self.users_share_map_by_uxu_share(uxu_share.clone());
        hash_temp.insert(sender_id, new_balance);
        self.data_mut().users_balance_by_uxu_share.insert(uxu_share.clone(), hash_temp );

        
        let old_amount: u128 = self.users_share_amount(uxu_share.clone(), receiver_id.clone());
        let new_balance = old_amount + amount;
        log!("{} + {} = new_balance {}", old_amount, amount, new_balance);
        let mut hash_temp = self.users_share_map_by_uxu_share(uxu_share.clone());
        hash_temp.insert(receiver_id, new_balance);
        self.data_mut().users_balance_by_uxu_share.insert(uxu_share, hash_temp );
    }

    ///Transfer uxu_shares internally (account to account),
    /// call mft_on_transfer in the receiver contract and 
    /// refound something if it is necessary.
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
            receiver_id.to_string(),
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
            receiver_id,
            amount,
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    } 

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
        };
        if unused_amount > 0 {
            
            let receiver_balance = self.users_share_amount(token_id.clone(), (*receiver_id).to_string());
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                // If sender's account was deleted, we assume that they have also withdrew all the liquidity from pools.
                // Funds are sent to the owner account.
                let refund_to = if self.data().accounts.get(&sender_id).is_some() {
                    sender_id
                } else {
                    self.data().owner_id.clone()
                };
                self.internal_mft_transfer(token_id, (*receiver_id).to_string(), refund_to.to_string(), refund_amount, None);
            } 
        }
        U128(unused_amount)
    }


}

