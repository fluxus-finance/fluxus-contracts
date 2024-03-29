source neardev/dev-account.env
source .env

near call $CONTRACT_NAME pembrock_create_strategy '{
    "strategy_fee": 5,
    "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    "sentry_fee": 10,
    "exchange_contract_id": "'$exchange_contract_id'", 
    "pembrock_contract_id": "'$pembrock_contract_id'",
    "pembrock_reward_id": "'$pembrock_reward_id'",
    "token_address": "'$token_address'", 
    "pool_id": '$pembrock_pool_id',
    "reward_token": "'$pembrock_reward_token'"
    }' --accountId $CONTRACT_NAME --gas $total_gas