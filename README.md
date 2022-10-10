# Fluxus Contracts

This repository contains the Fluxus contracts and scripts to deploy, run and test the project. In this README it will be possible to see and understand some of the crucial methods and functions in the code.


&nbsp;
## Contracts summary
| Contract  | Description |
| - | - 
| [Fluxus safe](fluxus-safe/src/lib.rs) | Fluxus safe contract - Main contract that manages users, deposits, fft_shares and strategies. The safe also have different folders for each project integrated in the fluxus (Pembrock and Ref-finance for example). |
| [Fluxus treasure](fluxus-treasurer/src/lib.rs) | Fluxus treasure contract - Responsible for receiving any token, swapping to WNEAR and distributing to the correct addresses. |



&nbsp;
## Dependencies

1 - Installing near-cli. If you need more help, there is an excellent guide at [Near Docs](https://docs.near.org/tools/near-cli)
```
curl -sL https://deb.nodesource.com/setup_18.x | sudo -E bash -  
sudo apt install build-essential nodejs
PATH="$PATH"
```
2 - Installing rust. This was based on [Rust official docs](https://doc.rust-lang.org/book/ch01-01-installation.html)
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3 - Setting up target
```
rustup target add wasm32-unknown-unknown
```
4 - Creating near testnet account - To be able to run the scripts and call the function in contracts,
 it is necessary to have a near account. To create it, go to:

https://wallet.testnet.near.org/create

5 - Logging in the the near account with the terminal
```
near login
```

6 - Setting up the .env file in the treasurer contract: Create a .env file inside the fluxus-treasurer's 
script folder, and add this lines as it is described on [.env.example file](https://github.com/fluxus-finance/fluxus-contracts/blob/main/fluxus-treasurer/.env.example):
```
export username=YOURUSERNAME.testnet
export exchange_contract_id="ref-finance-101.testnet"
export token_in="uxuu.leopollum.testnet"
export pool_token_in=466
export token_out="wrap.testnet"
export total_gas=300000000000000
# fee percentage to withdraw
export fee=50
```

7 - Setting up the .env file in the safe contract: Create a .env file inside the the fluxus-contracts/fluxus-safe/scripts/PROJECT (PROJECT: Pembrock, Ref_finance or ref_stable) depending of witch one do you want to build. It is a .env_example in every project so, create your .env file based on them.



&nbsp;
## Deploy and build the contract
1 - The first deployment needs to be the Treasurer contract because this contract_id will be necessary for the 
fluxus-safe deployment. To do it, go to the /script folder in the fluxus treasurer:

```
cd fluxus-contracts/fluxus-treasurer/scripts 
```

Then, run the run.sh file in the terminal. It will build the treasurer-project, deploy a near 
dev-account, then initialize and register the contract using the initialize.sh script.
```
./run.sh
```
Ok, now the treasurer is deployed. 

2 - Lets deploy the safe contract.
First of all, copy the treasurer contract_id that can be found in the neardev folder. 
Now, paste in the treasure_contract_id at the .env file. After this, go to the script folder in the 
fluxus-safe, and then execute the ./run.sh file to initialize the safe-contract, add a new strategy and 
call the stake process for the first time.

```
cd fluxus-contracts/fluxus-safe/scripts 
```
```
./run.sh
```
Alright, safe and treasurer are deployed in the near testnet. Well done.

&nbsp;
## Scripts explanation 
This session will focus on describing some of the most important scripts that are available in the safe and treasurer folders. These files were created to facilitate some important processes like deploying and registering the safe at other necessary contracts.

**1 - Safe scripts**

- build.sh: Responsible to build the project and create the wasm file.
```sh
#!/bin/bash
set -e

if [ -d "../res" ]; then
  echo ""
else
  mkdir ../res
fi

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release

cp ../../target/wasm32-unknown-unknown/release/fluxus_safe.wasm ../res/

```

- initialize.sh: Responsible to register the safe at contracts and also initialize fluxus-safe constructor.

EX: ref_finance/initialize.sh
```sh

#### Initialize contract
near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "treasure_contract_id": "'$treasure_contract_id'" }' --accountId $CONTRACT_NAME

#### Register contract 

#At ref
near call $CONTRACT_NAME call_user_register '{"exchange_contract_id": "'$exchange_contract_id'", "account_id": "'$CONTRACT_NAME'"}' --accountId $CONTRACT_NAME

#At the farm
near call $farm_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1

```


- add_strategy.sh: Responsible to create a strategy in the safe and register the safe in the token's contract.
EX: ref_finance/initialize.sh
```sh
near call $CONTRACT_NAME create_strategy '{
    "_strategy": "",
    "strategy_fee": 5,
    "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    "sentry_fee": 10,
    "exchange_contract_id": "'$exchange_contract_id'", 
    "farm_contract_id": "'$farm_contract_id'",
    "token1_address": "'$token1_address'", 
    "token2_address": "'$token2_address'", 
    "pool_id": '$pool_id', 
    "seed_min_deposit": "1000000000000000000" 
    }' --accountId $CONTRACT_NAME --gas $total_gas

near call $CONTRACT_NAME add_farm_to_strategy '{
    "seed_id": "'$seed_id'",
    "pool_id_token1_reward": '$pool_id_token1_reward', 
    "pool_id_token2_reward": '$pool_id_token2_reward', 
    "reward_token": "'$reward_token'",
    "farm_id": "'$farm_id'" 
}' --accountId $CONTRACT_NAME --gas $total_gas

# Register the contract in the pool 
#### TODO: move this call to create_auto_compounder method
near call $exchange_contract_id mft_register '{ "token_id" : ":'$pool_id'", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1

#At reward token
near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Register reward_token in the exchange in the contracts account whitelisted tokens
# only necessary for tokens that arent registered in the exchange already
near call $exchange_contract_id register_tokens '{ "token_ids" : [ "'$reward_token'" ] }' --accountId $CONTRACT_NAME  --gas 300000000000000 --depositYocto 1

```

- stake_process.sh: Responsible to do a transfer call from the user to the safe. This way, the user will stake a ref-lp in our contract.
EX: ref_finance/stake_process.sh
```sh
# # #### Add shares to contract and stake on farm
near call $exchange_contract_id mft_transfer_call '{"token_id": ":'$pool_id'", "receiver_id": "'$CONTRACT_NAME'", "amount": "1000000000000000000", "msg": "" }' --accountId $username --gas $total_gas --depositYocto 1
```

- unstake_process.sh: Responsible to withdraw the lps staked in the safe_contract. This way, the user will unstake a ref-lp.
EX:ref_finance/unstake_process.sh
```sh
#### Unstake given amount from contract
near call $CONTRACT_NAME unstake '{ "token_id": ":'$pool_id'", "amount_withdrawal": "500000000000000000" }' --accountId $username --gas 300000000000000 
```
**PS:** The scripts name and functions can be different depending of what part of the fluxus-contracts we are looking at, but the core implementation and logic are the same.

&nbsp;

**2 - Treasurer scripts**


- build.sh: Responsible to build the project and create the wasm file.
```sh
#!/bin/bash
set -e

if [ -d "../res" ]; then
  echo ""
else
  mkdir ../res
fi

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release

cp ../../target/wasm32-unknown-unknown/release/fluxus_treasurer.wasm ../res/
```

- initialize.sh: Responsible to register the treasurer in the ref exchange, the token contract and also initialize our constructor.
```sh
#### Initialize contract
near call $CONTRACT_NAME new '{"owner_id":'$username', "token_out": "'$token_out'", "exchange_contract_id": "ref-finance-101.testnet"}' --accountId leopollum.testnet

#### Register contract 

#At ref
near call $exchange_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 1

#At token_out
near call $token_out storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Make tokens available to swap on exchange (calls storage_deposit and register_tokens)
near call $CONTRACT_NAME register_token '{ "token" : "'$token_in'", "pool_id": '$pool_token_in' }' --accountId $CONTRACT_NAME --gas $total_gas 

# Add stakeholder account
near call $CONTRACT_NAME add_stakeholder '{ "account_id": '$username', "fee": '$fee' }' --accountId $CONTRACT_NAME

# Get contracts stakeholders
near call $CONTRACT_NAME get_stakeholders '{}' --accountId $CONTRACT_NAME

```


&nbsp;
## Contract files and Main methods

**1 - Safe files**

- **account_deposit**.rs: Manage user's account balance of tokens.

    - **register_tokens**: Receive a vector of tokens and register to the user that is calling. Parameter_ex: {"token_id":["token1.testnet", "token2.testnet"]}
    ```rs
        pub fn register_tokens(&mut self, token_ids: Vec<AccountId>) {
            assert_one_yocto();
            self.assert_contract_running();
            let sender_id = env::predecessor_account_id();
            let mut account = self.internal_unwrap_account(&sender_id);
            account.register(&token_ids);
            self.internal_save_account(&sender_id, account);
        }
    ```

    - **unregister_tokens**: Receive a vector of tokens and unregister to the user that is calling. Parameter_ex: {"token_id":["token1.testnet", "token2.testnet"]}
    ```rs
        pub fn unregister_tokens(&mut self, token_ids: Vec<AccountId>) {
            assert_one_yocto();
            self.assert_contract_running();
            let sender_id = env::predecessor_account_id();
            let mut account = self.internal_unwrap_account(&sender_id);
            for token_id in token_ids {
                account.unregister(&token_id);
            }
            self.internal_save_account(&sender_id, account);
        }
    ```

    - **withdraw**: Withdraws given token from the deposits of given user. Parameter_ex: { "token_id":"token1.testnet", "amount":"600000000", "unregister":false}
    ```rs
    pub fn withdraw(
        &mut self,
        token_id: AccountId,
        amount: U128,
        unregister: Option<bool>,
    ) -> Promise {
        assert_one_yocto();
        self.assert_contract_running();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        assert!(amount > 0, "{}", "E29: Illegal withdraw amount");
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        // Note: subtraction and deregistration will be reverted if the promise fails.
        account.withdraw(&token_id, amount);
        if unregister == Some(true) {
            account.unregister(&token_id);
        }
        self.internal_save_account(&sender_id, account);
        self.internal_send_tokens(&sender_id, &token_id, amount)
    }
    ```

&nbsp;
- **actions_of_strat**.rs: Manage user's account balance of tokens.

    - **create_strategy**: Responsible to create a new strategy to a new token_id (ref lp) (pool_id_example: 239). 
    ```rs
    pub fn create_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        ...
        ...
        ...
    }
    ```
    call example:
    ```sh
    near call $CONTRACT_NAME create_strategy '{
        "_strategy": "",
        "strategy_fee": 5,
        "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
        "sentry_fee": 10,
        "token1_address": "'$token1_address'", 
        "token2_address": "'$token2_address'", 
        "pool_id": '$pool_id', 
        "seed_min_deposit": "1000000000000000000" 
        }' --accountId $CONTRACT_NAME --gas $total_gas
    ```

    - **create_stable_strategy**: Responsible to create a new stable strategy. 
    ```rs
    pub fn create_stable_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        ...
        ...
        ...
    }
    ```

    - **create_jumbo_strategy**: Responsible to create a new jumbo strategy. 
    ```rs
    pub fn create_jumbo_strategy(
        &mut self,
        _strategy: String,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        farm_contract_id: AccountId,
        token1_address: AccountId,
        token2_address: AccountId,
        pool_id: u64,
        seed_min_deposit: U128,
    ) -> String {
        ...
        ...
        ...
    }
    ```

    - **pembrock_create_strategy**: Responsible to create a new pembrock strategy. 
    ```rs
    pub fn pembrock_create_strategy(
        &mut self,
        strategy_fee: u128,
        strat_creator: AccountFee,
        sentry_fee: u128,
        exchange_contract_id: AccountId,
        pembrock_contract_id: AccountId,
        pembrock_reward_id: AccountId,
        token_name: String,
        token1_address: AccountId,
        pool_id: u64,
        reward_token: AccountId,
    ) -> String {
        ...
        ...
        ...
    }
    ```


    - **add_farm_to_jumbo_strategy**: After a strategy creation, it is necessary to add a farm for it (farm_id_example: 0). 
    ```rs
    pub fn add_farm_to_jumbo_strategy(
        &mut self,
        seed_id: String,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
    ) -> String {
        ...
        ...
        ...
    }
    ```


    - **add_farm_to_strategy**: After a strategy creation, it is necessary to add a farm for it (farm_id_example: 0). 
    ```rs
    pub fn add_farm_to_strategy(
        &mut self,
        pool_id: u64,
        pool_id_token1_reward: u64,
        pool_id_token2_reward: u64,
        reward_token: AccountId,
        farm_id: String,
    ) -> String {
        ...
        ...
        ...
    }
    ```
    call example:
    ```sh
        near call $CONTRACT_NAME add_farm_to_strategy '{
            "pool_id": '$pool_id', 
            "pool_id_token1_reward": '$pool_id_token1_reward', 
            "pool_id_token2_reward": '$pool_id_token2_reward', 
            "reward_token": "'$reward_token'",
            "farm_id": "'$farm_id'" 
        }' --accountId $CONTRACT_NAME --gas $total_gas
    ```

    - **add_farm_to_stable_strategy**: After a strategy creation, it is necessary to add a farm for it. 
    ```rs
    pub fn add_farm_to_stable_strategy(
        &mut self,
        seed_id: String,
        token_address: AccountId,
        pool_id_token_reward: u64,
        token_position: u64,
        reward_token: AccountId,
        available_balance: Vec<Balance>,
        farm_id: String,
    ) -> String {
        ...
        ...
        ...
    }
    ```


    - **harvest**: It is a function that calls harvest_proxy that is responsible for the auto-compounder process. If the compounder is called for some pembrock strategy, the farm_id_str parameter needs to be empty (because we are doing the compounder for the pembrock_lend, where there are no farms), and the strat_name in passed (strat_name_ex: pembrock@461).

    ```rs
    pub fn harvest(&mut self, farm_id_str: String, strat_name: String) -> PromiseOrValue<u128> {
        let treasury = self.data().treasury.clone();

        let strat = if !strat_name.is_empty() {
            self.get_strat_mut(&strat_name)
        } else {
            let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());
            self.get_strat_mut(&seed_id)
        };

        strat.harvest_proxy(farm_id_str, strat_name, treasury)
    }
    ```
    call example:
    ```sh
    near call $CONTRACT_NAME harvest '{"farm_id_str": "'$farm_id_str'", "strat_name": "pembrock@461"}' --accountId $username --gas $total_gas
    ```


&nbsp;
- **fluxus_strat.rs**: It has util functions related to the all type of strategies.


    - **harvest_proxy**: It is the function called by the harvest function. The harvest_proxy identify the strategy type and than call the next auto_compounder cycle based on that. This identification is need because witch compounder has a specific set of functions and compounder steps.
    ```rs
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
    ```




&nbsp;
- **lib.rs**: It has the function that initializes the constructor.

    - **new**: Responsible to initialize the contract by passing some "key" parameters.
    ```rs
    pub fn new(owner_id: AccountId, treasure_contract_id: AccountId) -> Self {
        ...
        ...
        ...
    }

    ```
    call example:
    ```sh
    #### Initialize contract
    near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "treasure_contract_id": "'$treasure_contract_id'" }' --accountId $CONTRACT_NAME
    ````

&nbsp;
- **multi_fungible_token.rs**: Contain functions related to the fft_shares that are the token minted for to represent his shares of the vault when he makes a deposit of ref_lps (seed_ids).

    - **fft_token_seed_id**: Return the FFT token to a seed_id. Parameter_ex: {"seed_id":"ref-exchange-101.testnet@239"}
    ```rs
        pub fn fft_token_seed_id(&self, seed_id: String) -> String {
            let data = self.data();
            let fft_name: String = if let Some(fft_resp) = data.fft_share_by_seed_id.get(&seed_id) {
                fft_resp.to_owned()
            } else {
                env::panic_str("E1: seed_id doesn't exist");
            };
            fft_name
        }
    ```
    
    - **users_fft_share_amount**: Return the u128 amount of an user for an specific fft_share (ref lp token). Parameter_ex: {"fft_share":"fft_share_1", "user":"pollum.testnet"}
    ```rs
        pub fn users_fft_share_amount(&self, fft_share: String, account_id: String) -> u128 {
            let map = self.data().users_balance_by_fft_share.get(&fft_share);
            if let Some(shares) = map {
                if let Some(user_balance) = shares.get(&user) {
                    return user_balance;
                }
            }

            0
        }
    ```

    - **user_share_seed_id**: Return the u128 amount a user has in seed_id. Parameter_ex: {"seed_id":"ref-exchange-101.testnet@239", "user":"pollum.testnet"}
    ```rs
        pub fn user_share_seed_id(&self, seed_id: String, user: String) -> u128 {
           ...
           ...
           ...
        }
    ```

   - **user_share_seed_id**: Register a seed into the users_balance_by_fft_share. Parameter_ex: {"fft_share":"fft_share_1"}
    ```rs
        pub fn register_seed(&mut self, fft_share: String) {
            let temp = LookupMap::new(StorageKey::SeedRegister {
                fft_share: fft_share.clone(),
            });
            self.data_mut()
                .users_balance_by_fft_share
                .insert(&fft_share, &temp);
        }
    ```

   - **total_supply_amount**: Return the total_supply of an specific fft_share. Parameter_ex: {"fft_share":"fft_share_1"}
    ```rs
        pub fn total_supply_amount(&mut self, fft_share: String) {
            self.data()
                .total_supply_by_fft_share
                .get(&fft_share)
                .unwrap_or(0u128)
        }
    ```


   - **total_supply_amount**: Return the total_supply of an specific seed_id (ref lp token). Parameter_ex: {"token_id":"239"}
    ```rs
        pub fn total_supply_by_pool_id(&mut self, seed_id: String) -> u128 {
            let fft_share_id = self
                .data_mut()
                .fft_share_by_seed_id
                .get(&seed_id)
                .unwrap()
                .clone();

            let result = self.data_mut().total_supply_by_fft_share.get(&fft_share_id);
            if let Some(res) = result {
                res
            } else {
                0u128
            }
        }
    ```


    - **mft_mint**: Mints fft token of a specific strategy for a  a fft_share value to an user for a specific fft_share (ref lp token) and increment the total_supply of this seed's fft_share.Parameter_ex: {"fft_share":"fft_share_1","balance":99090999,"user":"pollum.testnet"}
    ```rs
        pub fn mft_mint(&mut self, fft_share: String, balance: u128, user: String) -> {
             //Add balance to the user for this seed
            let old_amount: u128 = self.users_fft_share_amount(fft_share.clone(), user.clone());

            let new_balance = old_amount + balance;
            log!("{} + {} = new_balance {}", old_amount, balance, new_balance);

            let mut map_temp = self
                .data()
                .users_balance_by_fft_share
                .get(&fft_share)
                .expect("err: fft does not exist");

            map_temp.insert(&user, &new_balance);

            self.data_mut()
                .users_balance_by_fft_share
                .insert(&fft_share, &map_temp);

            //Add balance to the total supply
            let old_total = self.total_supply_amount(fft_share.clone());
            self.data_mut()
                .total_supply_by_fft_share
                .insert(&fft_share, &(old_total + balance));

            //Returning the new balance
            new_balance
        }   
    ```

     - **mft_burn**: Burn fft_share value for an user in a specific fft_share (ref lp token) and decrement the total_supply of this seed's fft_share. Parameter_ex: {"fft_share":"fft_share_1","balance":99090999,"user":"pollum.testnet"}
    ```rs
        pub fn mft_burn(&mut self, fft_share: String, balance: u128, user: String) -> u128 {
            //Sub balance to the user for this seed
            let old_amount: u128 = self.users_fft_share_amount(fft_share.clone(), user.clone());
            assert!(old_amount >= balance);
            let new_balance = old_amount - balance;
            log!("{} - {} = new_balance {}", old_amount, balance, new_balance);

            let mut map_temp = self
                .data()
                .users_balance_by_fft_share
                .get(&fft_share)
                .expect("err: fft does not exist");

            map_temp.insert(&user, &new_balance);

            self.data_mut()
                .users_balance_by_fft_share
                .insert(&fft_share, &map_temp);

            //Sub balance to the total supply
            let old_total = self.total_supply_amount(fft_share.clone());
            self.data_mut()
                .total_supply_by_fft_share
                .insert(&fft_share, &(old_total - balance));

            //Returning the new balance
            new_balance
        }
    ```


    - **mft_transfer_call**: Transfer fft_shares internally (account to account), call mft_on_transfer in the receiver contract and refound something if it is necessary.
    ```rs
    pub fn mft_transfer_call(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        ...
        ...
        ...
    }
    ```
    call example:
    ```sh
    near call $exchange_contract_id mft_transfer_call '{"token_id": ":'$pool_id'", "receiver_id": "'$CONTRACT_NAME'", "amount": "1000000000000000000", "msg": "" }' --accountId $username --gas $total_gas --depositYocto 1
    ```



&nbsp;
- **owner.rs**: Contain functions that can only be called by the contract owner.

     - **update_contract_state**: Update the contract`s state (it can be running, paused or ended). Parameter_ex: {"state": Running}
    ```rs
        pub fn update_contract_state(&mut self, state: RunningState) -> String {
            self.is_owner();
            self.data_mut().state = state;
            format!("{} is {:#?}", env::current_account_id(), self.data().state)
        }
    ```

    - **update_compounder_state**: Update the compounder`s state (it can be running, paused or ended). Parameter_ex: {"farm_id_str": "ref-exchange-101.testnet@239#0", "state": Running}
    ```rs
    pub fn update_compounder_state(
        &mut self,
        farm_id_str: String,
        state: AutoCompounderState,
    ) -> String {
        self.is_owner();

        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());
        let compounder_mut = self.get_strat_mut(&token_id.to_string()).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);

        if farm_info_mut.state != state {
            farm_info_mut.state = state;
        }

        format!("The current state is {:#?}", farm_info_mut.state)
    }
    ```

    - **update_strat_slippage**: Update the strat`s slippage. Parameter_ex: {"farm_id_str": "ref-exchange-101.testnet@239#0", "new_slippage": 5}
    ```rs
    pub fn update_strat_slippage(&mut self, farm_id_str: String, new_slippage: u128) -> String {
        assert!(self.is_owner_or_guardians(), "ERR_");
        // TODO: what maximum slippage should be accepted?
        // Should not accept, say, 0 slippage
        let (seed_id, token_id, farm_id) = get_ids_from_farm(farm_id_str.to_string());

        let compounder_mut = self.get_strat_mut(&token_id.to_string()).get_mut();
        let farm_info_mut = compounder_mut.get_mut_farm_info(farm_id);
        farm_info_mut.slippage = 100 - new_slippage;

        format!(
            "The current slippage for {} is {}",
            token_id, farm_info_mut.slippage
        )
    }
    ```



&nbsp;
- **storage_impl.rs**: Responsible to the deposit-near functions (that are called to register some other contract in our).

    - **storage_deposit**: Register the user in the contract and deposit near. Parameter_ex: {"account_id": "pollum.testnet", "registration_only": true}
    ```rs
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        ...
        ...
        ...
    }
    ```

    - **storage_withdraw**: Withdraw the near storage deposited. Parameter_ex: {"amount": 20000000}
    ```rs
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance { 
        ...
        ...
        ...
    }
    ```
    

&nbsp;
- **views.rs**: It has view methods. 

    - **get_contract_state**: Return the contract state.
    ```rs
    pub fn get_contract_state(&self) -> String {
        format!("{} is {}", env::current_account_id(), self.data().state)
    }
    ```

    - **get_allowed_tokens**: Return all tokens registered in the contract.
    ```rs
    pub fn get_allowed_tokens(&self) -> Vec<String> {
        let mut seeds: Vec<String> = Vec::new();

        for (token_id, _) in self.data().strategies.iter() {
            seeds.push(token_id.clone());
        }

        seeds
    }
    ```

    - **get_running_farm_ids**: Return all farm_ids that has a running strategy in the contract.
    ```rs
    pub fn get_running_farm_ids(&self) -> Vec<String> {
        let mut running_strategies: Vec<String> = Vec::new();

        for token in self.data().token_ids.clone() {
            let strat = self.get_strat(token);
            let compounder = strat.get_ref();
            for farm in compounder.farms.iter() {
                if farm.state == AutoCompounderState::Running {
                    let farm_id = format!("{}#{}", compounder.seed_id, farm.id);
                    running_strategies.push(farm_id);
                }
            }
        }
        running_strategies
    }
    ```

    - **get_strategies**: Return all strategies in the contract.
    ```rs
    pub fn get_strategies(self) -> Vec<AutoCompounderInfo> {
        ...
        ...
        ...
    }
    ```

    - **get_strategies_info_for_ref_finance**: Return ref_finance strategies in the contract.
    ```rs
    pub fn get_strategies_info_for_ref_finance(&self) -> Vec<StratFarmInfo> {
        let mut info: Vec<StratFarmInfo> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"AUTO_COMPOUNDER" {
                let compounder = strat.get_compounder_ref();
                for farm in compounder.farms.iter() {
                    info.push(farm.clone())
                }
            }
        }
        info
    }
    ```
    - **get_strategies_info_for_stable_ref_finance**: Return stable_ref_finance strategies in the contract.
    ```rs
    pub fn get_strategies_info_for_stable_ref_finance(&self) -> Vec<StableStratFarmInfo> {
        let mut info: Vec<StableStratFarmInfo> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"STABLE_AUTO_COMPOUNDER" {
                let compounder = strat.get_stable_compounder_ref();
                for farm in compounder.farms.iter() {
                    info.push(farm.clone())
                }
            }
        }

        info
    }
    ```
    - **get_strategies_info_for_jumbo**: Return jumbo strategies in the contract.
    ```rs
        pub fn get_strategies_info_for_jumbo(&self) -> Vec<JumboStratFarmInfo> {
        let mut info: Vec<JumboStratFarmInfo> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"JUMBO_AUTO_COMPOUNDER" {
                for farm in strat.get_jumbo_ref().farms.iter() {
                    info.push(farm.clone());
                }
            }
        }

        info
    }
    ```
     - **get_strategies_info_for_pembrock**: Return pembrock strategies in the contract.
    ```rs
    pub fn get_strategies_info_for_pembrock(&self) -> Vec<PembrockAutoCompounder> {
        let mut info: Vec<PembrockAutoCompounder> = Vec::new();
        for (_, strat) in self.data().strategies.iter() {
            if strat.kind() == *"PEMBROCK_AUTO_COMPOUNDER" {
                info.push(strat.pemb_get_ref().clone());
            }
        }

        info
    }
    ```
    - **get_strategy_for_ref_finance**: Return some ref-finance strategy in the contract.
    ```rs
    pub fn get_strategy_for_ref_finance(self, farm_id_str: String) -> AutoCompounderState {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                let farm_info = compounder.get_farm_info(&farm_id);

                farm_info.state
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                let farm_info = compounder.get_farm_info(&farm_id);

                farm_info.state
            }
            _ => unimplemented!(),
        }
    }
    ```

    - **get_strategy_for_jumbo**: Return some jumbo strategy in the contract.
    ```rs
    pub fn get_strategy_for_jumbo(self, farm_id_str: String) -> JumboAutoCompounderState {
        let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);

        let compounder = self.get_strat(&seed_id).get_jumbo();
        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

        farm_info.state
    }
    ```

    - **get_strategy_for_pembrock**: Return some pembrock strategy in the contract.
    ```rs
    pub fn get_strategy_for_pembrock(self, strat_name: String) -> PembAutoCompounderState {
        let compounder = self.get_strat(&strat_name).pemb_get();

        compounder.state
    }
    ```




    - **get_running_strategies**: Return all strategies running in the contract. Parameter_example: {"farm_id_str": "ref-exchange-101.testnet@239#0"}
    ```rs
    pub fn get_running_strategies(&self, farm_id_str: String) -> String {
        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str);

        let strat = self.get_strat(token_id);
        let compounder = strat.get_ref();
        let farm_info = compounder.get_farm_info(&farm_id);

        farm_info.reward_token.into()
    }
    ```


    - **number_of_strategies**: Return the number of strategies in the contract.
    ```rs
    pub fn number_of_strategies(&self) -> U128 {
        let mut count: u128 = 0;

        for (_, strat) in self.data().strategies.iter() {
            count += match strat {
                VersionedStrategy::AutoCompounder(compounder) => compounder.farms.len() as u128,
                VersionedStrategy::StableAutoCompounder(compounder) => {
                    compounder.farms.len() as u128
                }
                VersionedStrategy::JumboAutoCompounder(compounder) => {
                    compounder.farms.len() as u128
                }
                VersionedStrategy::PembrockAutoCompounder(_) => 1,
            }
        }

        U128(count)
    }
    ```


    - **is_strategy_active**: Return if a strategy is active or not.
    ```rs
    pub fn is_strategy_active(&self, seed_id: String) -> bool {
        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                for farm in compounder.farms.iter() {
                    if farm.state == AutoCompounderState::Running {
                        return true;
                    }
                }
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                for farm in compounder.farms.iter() {
                    if farm.state == AutoCompounderState::Running {
                        return true;
                    }
                }
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                for farm in compounder.farms.iter() {
                    if farm.state == JumboAutoCompounderState::Running {
                        return true;
                    }
                }
            }
            VersionedStrategy::PembrockAutoCompounder(compounder) => {
                if compounder.state == PembAutoCompounderState::Running {
                    return true;
                }
            }
        }

        false
    }
    ```

    - **current_strat_step**: Return the last step done in the harvest. Parameter_example: {"farm_id_str": "ref-exchange-101.testnet@239#0", "strat_name":"pembrock@461"}
    ```rs
    pub fn current_strat_step(&self, farm_id_str: String, strat_name: String) -> String {
        match strat_name.is_empty() {
            false => String::from(&self.get_strat(&strat_name).pemb_get().cycle_stage),
            true => {
                let (seed_id, _, farm_id) = get_ids_from_farm(farm_id_str);
                let strat = self.get_strat(&seed_id);

                match strat {
                    VersionedStrategy::AutoCompounder(compounder) => {
                        let farm_info = compounder.get_farm_info(&farm_id);

                        String::from(&farm_info.cycle_stage)
                    }
                    VersionedStrategy::StableAutoCompounder(compounder) => {
                        let farm_info = compounder.get_farm_info(&farm_id);

                        String::from(&farm_info.cycle_stage)
                    }
                    VersionedStrategy::JumboAutoCompounder(compounder) => {
                        let farm_info = compounder.get_jumbo_farm_info(&farm_id);

                        String::from(&farm_info.cycle_stage)
                    }
                    _ => unimplemented!(),
                }
            }
        }
    }
    ```

    - **get_farm_ids_by_seed**: Return farms of a specific seed_id (ref_lp). Parameter_example: {"token_id": "239"}
    ```rs
    pub fn get_farm_ids_by_seed(&self, token_id: String) -> Vec<String> {
        let mut strats: Vec<String> = vec![];

        let compounder = self.get_strat(token_id.clone()).get_ref().clone();

        for farm in compounder.farms.iter() {
            strats.push(format!("{}#{}", token_id, farm.id));
        }

        strats
    }
    ```

    - **get_harvest_timestamp**: Return the last step done in the harvest. Parameter_example: {"token_id": "239"}
    ```rs
     pub fn get_harvest_timestamp(&self, seed_id: String) -> String {
        let strat = self.get_strat(&seed_id);

        match strat {
            VersionedStrategy::AutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
            VersionedStrategy::StableAutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
            VersionedStrategy::JumboAutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
            VersionedStrategy::PembrockAutoCompounder(compounder) => {
                compounder.harvest_timestamp.to_string()
            }
        }
    ```


&nbsp;
- **ref_finance/auto_compounder.rs**: It has the main auto-compounder's functions (harvest functions).

    - **claim_reward**: Function to claim the reward from the farm contract. Parameter_ex: {farm_id_str: exchange@pool_id#farm_id}
    ```rs
    pub(crate) fn claim_reward(&self, farm_id_str: String) -> Promise {
        log!("claim_reward");
        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());

        ext_ref_farming::list_seed_farms(
            seed_id,
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(callback_ref_finance::callback_list_farms_by_seed(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    ```


    - **withdraw_of_reward**: Function to withdraw the reward from the farm contract.  Parameter_ex: {farm_id_str: exchange@pool_id#farm_id}
    ```rs
    pub(crate) fn withdraw_of_reward(
        &self,
        farm_id_str: String,
        treasury_current_amount: u128,
    ) -> Promise {
        ...
        ...
        ...
    }
    ```

    - **autocompounds_swap**: Function to deal with all the swaps needed. Parameter_ex: {farm_id_str: exchange@pool_id#farm_id}
    ```rs
    pub(crate) fn autocompounds_swap(&self, farm_id_str: String, treasure: AccountFee) -> Promise {
        ...
        ...
        ...
    }
    ```


    - **autocompounds_liquidity_and_stake**: Function that stake the amount of tokens available. Parameter_ex: {farm_id_str: exchange@pool_id#farm_id}
    ```rs
    pub(crate) fn autocompounds_liquidity_and_stake(&self, farm_id_str: String) -> Promise {
        log!("autocompounds_liquidity_and_stake");

        // send reward to contract caller
        self.send_reward_to_sentry(farm_id_str, env::predecessor_account_id())
    }
    ```





&nbsp;
- **jumbo/jumbo_auto_compounder.rs**: It has the main jumbo auto-compounder's functions (harvest functions).

    - **claim_reward**: Function to claim the reward from the farm contract. 
    ```rs
    pub fn claim_reward(&mut self, farm_id_str: String) -> Promise {
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("claim_reward");

        let (seed_id, _, _) = get_ids_from_farm(farm_id_str.to_string());

        ext_jumbo_farming::list_farms_by_seed(
            seed_id,
            self.farm_contract_id.clone(),
            0,
            Gas(40_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_list_farms_by_seed(
            farm_id_str,
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }
    ```

    - **withdraw_of_reward**: Function to withdraw the reward from the farm contract. 
    ```rs
    pub fn withdraw_of_reward(
        &mut self,
        farm_id_str: String,
        treasury_current_amount: u128,
    ) -> Promise {
      ...
      ...
      ...
    }
    ```


    - **autocompounds_swap**: Function that swap the reward received the the correct tokens. (parameter_ex: farm_id_str: exchange@pool_id#farm_id)
    ```rs
    pub fn autocompounds_swap(&mut self, farm_id_str: String, treasure: AccountFee) -> Promise {
       ...
       ...
       ...
    }
    ```


    - **autocompounds_swap_second_token**: Transfer reward token to ref-exchange then swap the amount the contract has in the exchange. (parameter_ex: farm_id_str: exchange@pool_id#farm_id)

    ```rs
    pub fn autocompounds_swap_second_token(&mut self, farm_id_str: String) -> Promise {
        // TODO: take string as ref
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("autocompounds_swap_second_token");

        let (_, token_id, farm_id) = get_ids_from_farm(farm_id_str.clone());
        let farm_info = self.get_jumbo_farm_info(&farm_id);

        let reward_amount_left = farm_info.last_reward_amount;

        // 130 TGAS
        ext_jumbo_exchange::get_return(
            farm_info.pool_id_token2_reward,
            farm_info.reward_token,
            U128(reward_amount_left),
            self.token2_address.clone(),
            self.exchange_contract_id.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_jumbo_exchange::callback_jumbo_get_token2_return(
            farm_id_str,
            U128(reward_amount_left),
            env::current_account_id(),
            0,
            Gas(120_000_000_000_000),
        ))
    }
    ```


    - **autocompounds_liquidity_and_stake**: Get amount of tokens available then stake it. (parameter_ex: farm_id_str: exchange@pool_id#farm_id)
    ```rs
    pub fn autocompounds_liquidity_and_stake(&mut self, farm_id_str: String) -> Promise {
        // self.assert_strategy_not_cleared(&farm_id_str);
        log!("autocompounds_liquidity_and_stake");

        // send reward to contract caller
        self.jumbo_send_reward_to_sentry(farm_id_str, env::predecessor_account_id())
    }
    ```



&nbsp;
- **pembrock/pembrock_auto_compounder.rs**: It has the main pembrock auto-compounder's functions (harvest functions).

    - **claim_reward**: Function to claim the reward from the lend contract. (parameter_ex: strat_name: pembrock@461)
    ```rs
       pub fn claim_reward(&mut self, strat_name: String) -> Promise {
        ext_pembrock::claim(self.pembrock_reward_id.clone(), 1, Gas(100_000_000_000_000)).then(
            callback_pembrock::callback_pembrock_rewards(
                strat_name,
                env::current_account_id(),
                0,
                Gas(120_000_000_000_000),
            ),
        )
    }
    ```

    - **swap_and_lend**: Function to make the swaps in ref and lend the amount_out in the pembrock function. (parameter_ex: strat_name: pembrock@461)
    ```rs
    pub fn swap_and_lend(&self, strat_name: String) -> Promise {
        let sentry_acc_id = env::predecessor_account_id();

        ext_reward_token::storage_balance_of(
            sentry_acc_id.clone(),
            self.reward_token.clone(),
            0,
            Gas(10_000_000_000_000),
        )
        .then(callback_pembrock::callback_pembrock_post_sentry(
            strat_name,
            sentry_acc_id,
            self.reward_token.clone(),
            env::current_account_id(),
            0,
            Gas(260_000_000_000_000),
        ))
    }
    ```


&nbsp;
**2 - Treasurer files**

- **lib.rs**: It has view methods. 

    - **new**: Function that initialize the contract.
    ```rs
    pub fn new(owner_id: AccountId, token_out: AccountId, exchange_contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let allowed_accounts: Vec<AccountId> = vec![env::current_account_id()];

        Self {
            data: VersionedContractData::V0001(ContractData {
                owner_id,
                stakeholders_fees: HashMap::new(),
                stakeholders_amount_available: HashMap::new(),
                token_out,
                token_to_pool: HashMap::new(),
                state: RunningState::Running,
                exchange_contract_id,
            }),
        }
    }
    ```

    call example:
    ```sh
        #### Initialize contract
        near call $CONTRACT_NAME new '{"owner_id":'$username', "token_out": "'$token_out'", "exchange_contract_id": "ref-finance-101.testnet"}' --accountId $username
    ```


    - **execute_swaps_and_distribute**: Function responsible for swapping rewards tokens for the token distributed.
    ```rs
    pub fn execute_swaps_and_distribute(&self, token: AccountId) -> Promise {
        // self.assert_contract_running();
        // self.is_owner();

        let token_pool = self.data().token_to_pool.get(&token);
        let mut pool: u64 = 0;

        match token_pool {
            Some(pool_id) => {
                pool = *pool_id;
            }
            _ => env::panic_str("TREASURER::TOKEN_NOT_REGISTERED"),
        };

        ext_exchange::get_deposit(
            env::current_account_id(),
            token.clone(),
            self.exchange_acc(),
            1,
            Gas(9_000_000_000_000),
        )
        .then(ext_self::get_token_return_and_swap(
            token,
            pool,
            env::current_account_id(),
            0,
            Gas(70_000_000_000_000),
        ))
        .then(ext_self::callback_post_swap(
            env::current_account_id(),
            0,
            Gas(100_000_000_000_000),
        ))
    }
    ```


    - **withdraw**: Transfer caller's current available amount from contract to caller.
    ```rs
    pub fn withdraw(&self) -> Promise {
        let (caller_id, contract_id) = self.get_predecessor_and_current_account();

        assert!(
            self.data().stakeholders_fees.contains_key(&caller_id),
            "TREASURER::ERR_ACCOUNT_DOES_NOT_EXIST"
        );

        let amount: &u128 = self
            .data()
            .stakeholders_amount_available
            .get(&caller_id)
            .unwrap();

        assert_ne!(*amount, 0u128, "TREASURER::ERR_WITHDRAW_ZERO_AMOUNT");

        ext_input_token::ft_transfer(
            caller_id.clone(),
            U128(*amount),
            Some(String::from("")),
            self.data().token_out.clone(),
            1,
            Gas(100_000_000_000_000),
        )
        .then(ext_self::callback_withdraw(
            caller_id,
            contract_id,
            0,
            Gas(50_000_000_000_000),
        ))
    }
    ```

&nbsp;
- **stakeholders.rs**: It has stakeholder`s function. 

    - **add_stakeholder**: Function that add a new stakeholder in the contract. Parameters_ex: {"account_id": "pollum.testnet", "fee": 5}
    ```rs
    pub fn add_stakeholder(&mut self, account_id: AccountId, fee: u128) -> String {
        self.is_owner();

        let mut total_fees: u128 = 0u128;

        for (acc_id, account_fee) in self.data().stakeholders_fees.iter() {
            assert!(
                *acc_id != account_id,
                "TREASURER::ERR_ADDRESS_ALREADY_EXIST"
            );
            total_fees += account_fee;
        }

        total_fees += fee;

        assert!(
            total_fees <= 100u128,
            "TREASURER::ERR_FEE_EXCEEDS_MAXIMUM_VALUE"
        );

        self.data_mut()
            .stakeholders_fees
            .insert(account_id.clone(), fee);
        self.data_mut()
            .stakeholders_amount_available
            .insert(account_id.clone(), 0u128);

        format!(
            "Account {} was added with {} proportion from value",
            account_id, fee
        )
    }
    ```

    - **remove_stakeholder**: Function that removes a stakeholder in the contract. Parameters_ex: {"account_id": "pollum.testnet"}
    ```rs
    pub fn remove_stakeholder(&mut self, account_id: AccountId) {
        self.is_owner();
        self.data_mut().stakeholders_fees.remove(&account_id);
    }
    ```

    - **update_stakeholder_percentage**: Function that updates a stakeholder percentage in the contract. Parameters_ex: {"account_id": "pollum.testnet", "new_percentage":10}
    ```rs
    pub fn update_stakeholder_percentage(
        &mut self,
        account_id: AccountId,
        new_percentage: u128,
    ) -> String {
        self.is_owner();
        assert!(
            self.data().stakeholders_fees.contains_key(&account_id),
            "TREASURER::ERR_ACCOUNT_DOES_NOT_EXIST"
        );

        let mut total_fees = new_percentage;
        for (account, percentage) in self.data().stakeholders_fees.iter() {
            if account != &account_id {
                total_fees += percentage;
            }
        }

        assert!(
            total_fees <= 100u128,
            "TREASURER::ERR_FEE_EXCEEDS_MAXIMUM_VALUE"
        );

        self.data_mut()
            .stakeholders_fees
            .insert(account_id.clone(), new_percentage);

        format! { "The percentage for {} is now {}", account_id, new_percentage}
    }
    ```
