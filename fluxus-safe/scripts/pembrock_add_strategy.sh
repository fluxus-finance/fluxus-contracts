source neardev/dev-account.env
source .env

near call $CONTRACT_NAME pembrock_create_strategy '{
    "strategy_fee": 5,
    "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    "sentry_fee": 10,
    "exchange_contract_id": "'$exchange_contract_id'", 
    "pembrock_contract_id": "'$pembrock_contract_id'",
    "pembrock_reward_id": "'$pembrock_reward_id'",
    "token1_address": "'$token_address'", 
    "token_name": "'$token_name'", 
    "pool_id": '$pembrock_pool_id',
    "reward_token": "'$pembrock_reward_token'"
    }' --accountId $CONTRACT_NAME --gas $total_gas


# near call $CONTRACT_NAME add_farm_to_strategy '{
#     "seed_id": "'$seed_id'",
#     "pool_id_token1_reward": '$pool_id_token1_reward', 
#     "pool_id_token2_reward": '$pool_id_token2_reward', 
#     "reward_token": "'$reward_token'",
#     "farm_id": "'$farm_id'" 
# }' --accountId $CONTRACT_NAME --gas $total_gas