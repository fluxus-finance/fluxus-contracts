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
    //Return the FFT token to a seed_id TODO: enhance name of fft tokens they should be named fft_seed_{seed_id}
    pub fn fft_token_seed_id(&self, seed_id: String) -> String {
        let data = self.data();
        let fft_name: String = if let Some(fft_resp) = data.fft_share_by_seed_id.get(&seed_id) {
            fft_resp.to_owned()
        } else {
            env::panic_str("E1: seed_id doesn't exist");
        };
        fft_name
    }
    ///Return the u128 amount of an user for an specific fft_share (ref lp token).
    pub fn users_fft_share_amount(&self, fft_share: String, user: String) -> u128 {
        let map = self.data().users_balance_by_fft_share.get(&fft_share);
        if let Some(shares) = map {
            if let Some(user_balance) = shares.get(&user) {
                return user_balance;
            }
        }

        0
    }
    /// Return the u128 amount a user has in seed_id.
    pub fn user_share_seed_id(&self, seed_id: String, user: String) -> u128 {
        let data = self.data();
        let fft_name: String = if let Some(fft_resp) = data.fft_share_by_seed_id.get(&seed_id) {
            fft_resp.to_owned()
        } else {
            env::panic_str("E1: seed_id doesn't exist");
        };

        let user_fft_shares = self.users_fft_share_amount(fft_name.clone(), user);
        let total_fft = self.total_supply_amount(fft_name);
        let total_seed = *self.data().seed_id_amount.get(&seed_id).unwrap_or(&0_u128);
        log!(
            "user_fft {} total fft {} total seed {}",
            user_fft_shares,
            total_fft,
            total_seed
        );
        if total_fft == 0_u128 || total_seed == 0 || user_fft_shares == 0 {
            0
        } else {
            (U256::from(user_fft_shares) * U256::from(total_seed) / U256::from(total_fft)).as_u128()
        }
    }

    ///Register a seed into the users_balance_by_fft_share
    pub fn register_seed(&mut self, fft_share: String) {
        let mut temp = LookupMap::new(b"a");
        temp.insert(&"".to_string(), &0_u128);
        self.data_mut()
            .users_balance_by_fft_share
            .insert(&fft_share, &temp);
    }

    pub fn seed_total_amount(&self, token_id: String) -> u128 {
        let mut id = token_id;
        id.remove(0).to_string();
        let seed_id: String = format!("{}@{}", self.data().exchange_contract_id, id);

        let temp = self.data().seed_id_amount.get(&seed_id).unwrap();
        *temp
    }

    ///Return users_balance of a specific fft_share
    // #[inline]
    // pub fn users_share_map_by_fft_share(&self, fft_share: String, account_id: String) -> u128 {
    //     let temp = self
    //         .data()
    //         .users_balance_by_fft_share
    //         .get(&fft_share)
    //         .unwrap();
    //     temp.get(&account_id).unwrap()
    // }

    ///Return the total_supply of an specific fft_share (ref lp token).
    pub fn total_supply_amount(&self, fft_share: String) -> u128 {
        let result = self.data().total_supply_by_fft_share.get(&fft_share);
        if let Some(res) = result {
            *res
        } else {
            0u128
        }
    }

    ///Return the total_supply of an specific fft_share (ref lp token).
    pub fn total_supply_by_pool_id(&mut self, token_id: String) -> u128 {
        let seed_id: String = format!("{}@{}", self.data_mut().exchange_contract_id, token_id);
        log!("Total supply of: {}", seed_id);
        let fft_share_id = self
            .data_mut()
            .fft_share_by_seed_id
            .get(&seed_id)
            .unwrap()
            .clone();

        let result = self.data_mut().total_supply_by_fft_share.get(&fft_share_id);
        if let Some(res) = result {
            *res
        } else {
            0u128
        }
    }
    pub fn convert_pool_id_in_fft_share(&mut self, token_id: String) -> String {
        let seed_id: String = format!("{}@{}", self.data_mut().exchange_contract_id, token_id);

        let fft_share_id = self
            .data_mut()
            .fft_share_by_seed_id
            .get(&seed_id)
            .unwrap()
            .clone();
        log!("fft id is: {}", fft_share_id);
        fft_share_id
    }

    ///Assigns a fft_share value to an user for a specific fft_share (ref lp token)
    /// and increment the total_supply of this seed's fft_share.
    /// It returns the user's new balance.
    pub fn mft_mint(&mut self, fft_share: String, balance: u128, user: String) -> u128 {
        //Add balance to the user for this seed
        let old_amount: u128 = self.users_fft_share_amount(fft_share.clone(), user.clone());

        let new_balance = old_amount + balance;
        log!("{} + {} = new_balance {}", old_amount, balance, new_balance);

        let temp = self.data().users_balance_by_fft_share.get(&fft_share);

        let mut map_temp = if temp.is_some() {
            temp.unwrap()
        } else {
            LookupMap::new(b"a")
        };
        map_temp.insert(&user, &new_balance);

        self.data_mut()
            .users_balance_by_fft_share
            .insert(&fft_share, &map_temp);

        //Add balance to the total supply
        let old_total = self.total_supply_amount(fft_share.clone());
        self.data_mut()
            .total_supply_by_fft_share
            .insert(fft_share, (old_total + balance));

        //Returning the new balance
        new_balance
    }

    ///Burn fft_share value for an user in a specific fft_share (ref lp token)
    /// and decrement the total_supply of this seed's fft_share.
    /// It returns the user's new balance.
    pub fn mft_burn(&mut self, fft_share: String, balance: u128, user: String) -> u128 {
        //Sub balance to the user for this seed
        let old_amount: u128 = self.users_fft_share_amount(fft_share.clone(), user.clone());
        assert!(old_amount >= balance);
        let new_balance = old_amount - balance;
        log!("{} - {} = new_balance {}", old_amount, balance, new_balance);

        let temp = self.data().users_balance_by_fft_share.get(&fft_share);

        let mut map_temp = if temp.is_some() {
            temp.unwrap()
        } else {
            LookupMap::new(b"a")
        };

        map_temp.insert(&user, &new_balance);

        self.data_mut()
            .users_balance_by_fft_share
            .insert(&fft_share, &map_temp);

        //Sub balance to the total supply
        let old_total = self.total_supply_amount(fft_share.clone());
        self.data_mut()
            .total_supply_by_fft_share
            .insert(fft_share, (old_total - balance));

        //Returning the new balance
        new_balance
    }

    /// Transfer fft_shares internally (user for user).
    /// Token_id is a specific fft_share.
    #[payable]
    pub fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: String,
        amount: U128,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        log!("{}", env::predecessor_account_id().to_string());
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
        self.share_transfer(
            token_id.clone(),
            sender_id.clone(),
            receiver_id.clone(),
            amount,
        );

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

    pub fn share_transfer(
        &mut self,
        fft_share: String,
        sender_id: String,
        receiver_id: String,
        amount: u128,
    ) {
        log!("{} and {}", sender_id, fft_share);
        let old_amount: u128 = self.users_fft_share_amount(fft_share.clone(), sender_id.clone());
        log!("{} > = {}", old_amount, amount);
        assert!(old_amount >= amount);
        log!("{} - {}", old_amount, amount);
        let new_balance = old_amount - amount;
        log!("{} + {} = new_balance {}", old_amount, amount, new_balance);

        // let mut hash_temp = self.users_share_map_by_fft_share(fft_share.clone());
        // hash_temp.insert(&sender_id, &new_balance);

        let temp = self.data().users_balance_by_fft_share.get(&fft_share);

        let mut map_temp = if temp.is_some() {
            temp.unwrap()
        } else {
            LookupMap::new(b"a")
        };
        map_temp.insert(&sender_id, &new_balance);

        self.data_mut()
            .users_balance_by_fft_share
            .insert(&fft_share, &map_temp);

        let old_amount: u128 = self.users_fft_share_amount(fft_share.clone(), receiver_id.clone());
        let new_balance = old_amount + amount;
        log!("{} + {} = new_balance {}", old_amount, amount, new_balance);

        // let mut hash_temp = self.users_share_map_by_fft_share(fft_share.clone());
        // hash_temp.insert(&receiver_id, &new_balance);

        let temp = self.data().users_balance_by_fft_share.get(&fft_share);

        let mut map_temp = if temp.is_some() {
            temp.unwrap()
        } else {
            LookupMap::new(b"a")
        };
        map_temp.insert(&receiver_id, &new_balance);

        self.data_mut()
            .users_balance_by_fft_share
            .insert(&fft_share, &map_temp);
    }

    ///Transfer fft_shares internally (account to account),
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
            let receiver_balance =
                self.users_fft_share_amount(token_id.clone(), (*receiver_id).to_string());
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                // If sender's account was deleted, we assume that they have also withdrew all the liquidity from pools.
                // Funds are sent to the owner account.
                let refund_to = if self.data().accounts.get(&sender_id).is_some() {
                    sender_id
                } else {
                    self.data().owner_id.clone()
                };
                self.internal_mft_transfer(
                    token_id,
                    (*receiver_id).to_string(),
                    refund_to.to_string(),
                    refund_amount,
                    None,
                );
            }
        }
        U128(unused_amount)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context() -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(to_account_id("auto_compounder.near"))
            .signer_account_id(to_account_id("auto_compounder.near"))
            .predecessor_account_id(to_account_id("auto_compounder.near"))
            .attached_deposit(1);
        builder
    }

    pub fn to_account_id(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn create_account() -> Account {
        let account_struct = Account::new(&to_account_id("fluxus.near"));
        account_struct
    }

    #[test]
    fn test_mint() {
        let context = get_context();
        testing_env!(context.build());

        let mut account = create_account();
        let mut contract = Contract::new(
            "auto_compounder.near".parse().unwrap(),
            "ref-finance-101.testnet".parse().unwrap(),
            "farm101.fluxusfi.testnet".parse().unwrap(),
            "dev-1656420526638-61041719201929".parse().unwrap(),
        );

        //Registering seed
        contract.register_seed("fft_share_1".to_string());

        //Minting fft_share
        let mut deposit =
            contract.mft_mint("fft_share_1".to_string(), 10_u128, "user1".to_string());
        assert_eq!(deposit, 10_u128);

        //Checking balance
        let mut balance =
            contract.users_fft_share_amount("fft_share_1".to_string(), "user1".to_string());
        assert_eq!(balance, 10_u128);

        //Minting more fft_share
        deposit = contract.mft_mint("fft_share_1".to_string(), 10_u128, "user1".to_string());
        assert_eq!(deposit, 20_u128);
    }

    #[test]
    fn test_burn() {
        let context = get_context();
        testing_env!(context.build());

        let mut account = create_account();
        let mut contract = Contract::new(
            "auto_compounder.near".parse().unwrap(),
            "ref-finance-101.testnet".parse().unwrap(),
            "farm101.fluxusfi.testnet".parse().unwrap(),
            "dev-1656420526638-61041719201929".parse().unwrap(),
        );

        //Seed register
        contract.register_seed("fft_share_1".to_string());

        //Minting fft_share
        let mut deposit =
            contract.mft_mint("fft_share_1".to_string(), 10_u128, "user1".to_string());
        assert_eq!(deposit, 10_u128);

        //burning fft_share
        let mut balance = contract.mft_burn("fft_share_1".to_string(), 2_u128, "user1".to_string());
        assert_eq!(balance, 8_u128);

        //Checking total supply
        balance = contract.total_supply_amount("fft_share_1".to_string());
        assert_eq!(balance, 8_u128);
    }

    #[test]
    fn test_transfer() {
        let context = get_context();
        testing_env!(context.build());
        let mut account = create_account();
        let mut contract = Contract::new(
            "auto_compounder.near".parse().unwrap(),
            "ref-finance-101.testnet".parse().unwrap(),
            "farm101.fluxusfi.testnet".parse().unwrap(),
            "dev-1656420526638-61041719201929".parse().unwrap(),
        );

        //Seed register
        contract.register_seed("fft_share_1".to_string());

        //Minting balance for users
        let mut balance_user1 = contract.mft_mint(
            "fft_share_1".to_string(),
            10_u128,
            "auto_compounder.near".to_string(),
        );
        assert_eq!(balance_user1, 10_u128);
        let mut balance_user2 =
            contract.mft_mint("fft_share_1".to_string(), 10_u128, "user2".to_string());
        assert_eq!(balance_user2, 10_u128);
        let mut balance_user3 =
            contract.mft_mint("fft_share_1".to_string(), 999_u128, "user3".to_string());
        assert_eq!(balance_user3, 999_u128);

        //Checking total supply
        let total_supply = contract.total_supply_amount("fft_share_1".to_string());
        assert_eq!(total_supply, 1019_u128);

        //Transferring fft_shares
        contract.mft_transfer(
            "fft_share_1".to_string(),
            "user2".to_string(),
            U128::from(5_u128),
            None,
        );
        balance_user1 = contract.users_fft_share_amount(
            "fft_share_1".to_string(),
            "auto_compounder.near".to_string(),
        );
        assert_eq!(balance_user1, 5_u128);
        balance_user2 =
            contract.users_fft_share_amount("fft_share_1".to_string(), "user2".to_string());
        assert_eq!(balance_user2, 15_u128);

        //Transferring fft_shares
        contract.mft_transfer(
            "fft_share_1".to_string(),
            "user3".to_string(),
            U128::from(5_u128),
            None,
        );
        balance_user1 = contract.users_fft_share_amount(
            "fft_share_1".to_string(),
            "auto_compounder.near".to_string(),
        );
        assert_eq!(balance_user1, 0_u128);
        balance_user3 =
            contract.users_fft_share_amount("fft_share_1".to_string(), "user3".to_string());
        assert_eq!(balance_user3, 1004_u128);
    }
}
