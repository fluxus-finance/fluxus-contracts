
source .env
echo $username
source neardev/dev-account.env
echo $CONTRACT_NAME

#### Deploy with near-dev
near dev-deploy --wasmFile ../../res/fluxus_safe.wasm

#### Initialize contract
near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "treasure_contract_id": "'$treasure_contract_id'" }' --accountId $CONTRACT_NAME

#### Register contract 
near call $exchange_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME
near call $farm_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1


#### Add strategies
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

near call $CONTRACT_NAME add_farm_to_jumbo_strategy {
  "seed_id": "dev-1660920856823-70071820486313@0",
  "pool_id_token1_reward": 3,
  "pool_id_token2_reward": 2,
  "reward_token": "dai.fakes.testnet",
  "farm_id": "2"
}