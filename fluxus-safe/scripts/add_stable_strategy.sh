source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token

#### Create first strategy
near call $CONTRACT_NAME create_stable_strategy '{
    "_strategy": "",
    "strategy_fee": 5,
    "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    "sentry_fee": 10,
    "exchange_contract_id": "'$exchange_contract_id'", 
    "farm_contract_id": "'$farm_contract_id'",
    "token_address": "'$token_address'", 
    "pool_id": '$pool_id', 
    "seed_min_deposit": "1000000000000000000" 
    }' --accountId $CONTRACT_NAME --gas $total_gas

near call $CONTRACT_NAME add_farm_to_stable_strategy '{
    "seed_id": "'$seed_id'",
    "pool_id_token_reward": '$pool_id_token1_reward', 
    "reward_token": "'$reward_token'",
    "farm_id": "'$farm_id'" 
}' --accountId $CONTRACT_NAME --gas $total_gas

# # Register the contract in the pool 
# #### TODO: move this call to create_auto_compounder method
near call $exchange_contract_id mft_register '{ "token_id" : ":'$pool_id'", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1

#At reward token
near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Register reward_token in the exchange in the contracts account whitelisted tokens
# only necessary for tokens that arent registered in the exchange already
near call $exchange_contract_id register_tokens '{ "token_ids" : [ "'$reward_token'" ] }' --accountId $CONTRACT_NAME  --gas 300000000000000 --depositYocto 1
