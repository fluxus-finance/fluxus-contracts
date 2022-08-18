source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token

#### Create first strategy
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

near call $CONTRACT_NAME add_farm_to_strategy '{
    "pool_id": '$pool_id', 
    "pool_id_token1_reward": '$pool_id_token1_reward', 
    "pool_id_token2_reward": '$pool_id_token2_reward', 
    "reward_token": "'$reward_token'",
    "farm_id": "'$farm_id'" 
}' --accountId $CONTRACT_NAME --gas $total_gas

#At reward token
near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Register reward_token in the exchange in the contracts account whitelisted tokens
# only necessary for tokens that arent registered in the exchange already
near call $exchange_contract_id register_tokens '{ "token_ids" : [ "'$reward_token'" ] }' --accountId $CONTRACT_NAME  --gas 300000000000000 --depositYocto 1
