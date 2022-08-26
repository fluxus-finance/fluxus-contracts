source neardev/dev-account.env
source .env

near call $CONTRACT_NAME pembrock_create_strategy '{
    "_strategy": "",
    "strategy_fee": 5,
    "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    "sentry_fee": 10,
    "exchange_contract_id": "'$exchange_contract_id'", 
    "farm_contract_id": "'$pemb_farm_contract_id'",
    "token1_address": "'$pemb_token_address'", 
    "token_name": "'$token_name'", 
    "seed_min_deposit": "1000000000000000000",
    "pool_id":462 
    }' --accountId $CONTRACT_NAME --gas $total_gas


# near call $CONTRACT_NAME add_farm_to_strategy '{
#     "seed_id": "'$seed_id'",
#     "pool_id_token1_reward": '$pool_id_token1_reward', 
#     "pool_id_token2_reward": '$pool_id_token2_reward', 
#     "reward_token": "'$reward_token'",
#     "farm_id": "'$farm_id'" 
# }' --accountId $CONTRACT_NAME --gas $total_gas