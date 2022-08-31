
source .env

source neardev/dev-account.env

# near call $CONTRACT_NAME pembrock_create_strategy '{
#     "strategy_fee": 5,
#     "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
#     "sentry_fee": 10,
#     "exchange_contract_id": "'$exchange_contract_id'", 
#     "pembrock_contract_id": "'$pembrock_contract_id'",
#     "pembrock_reward_id": "'$pembrock_reward_id'",
#     "token1_address": "usdt.fakes.testnet", 
#     "token_name": "usdt", 
#     "pool_id": 462,
#     "reward_token": "'$pembrock_reward_token'"
# }' --accountId $CONTRACT_NAME --gas $total_gas
# near call usdt.fakes.testnet storage_deposit '{ "account_id": "'$CONTRACT_NAME'", "registration_only": false }' --accountId $CONTRACT_NAME --deposit 1


# near call $CONTRACT_NAME pembrock_create_strategy '{
#     "strategy_fee": 5,
#     "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
#     "sentry_fee": 10,
#     "exchange_contract_id": "'$exchange_contract_id'", 
#     "pembrock_contract_id": "'$pembrock_contract_id'",
#     "pembrock_reward_id": "'$pembrock_reward_id'",
#     "token1_address": "eth.fakes.testnet", 
#     "token_name": "eth", 
#     "pool_id": 1409,
#     "reward_token": "'$pembrock_reward_token'"
# }' --accountId $CONTRACT_NAME --gas $total_gas
near call eth.fakes.testnet storage_deposit '{ "account_id": "'$CONTRACT_NAME'", "registration_only": false }' --accountId $CONTRACT_NAME --deposit 1


# near call $CONTRACT_NAME pembrock_create_strategy '{
#     "strategy_fee": 5,
#     "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
#     "sentry_fee": 10,
#     "exchange_contract_id": "'$exchange_contract_id'", 
#     "pembrock_contract_id": "'$pembrock_contract_id'",
#     "pembrock_reward_id": "'$pembrock_reward_id'",
#     "token1_address": "ref.fakes.testnet", 
#     "token_name": "ref", 
#     "pool_id": 1408,
#     "reward_token": "'$pembrock_reward_token'"
# }' --accountId $CONTRACT_NAME --gas $total_gas
near call ref.fakes.testnet storage_deposit '{ "account_id": "'$CONTRACT_NAME'", "registration_only": false }' --accountId $CONTRACT_NAME --deposit 1


# near call $CONTRACT_NAME pembrock_create_strategy '{
#     "strategy_fee": 5,
#     "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
#     "sentry_fee": 10,
#     "exchange_contract_id": "'$exchange_contract_id'", 
#     "pembrock_contract_id": "'$pembrock_contract_id'",
#     "pembrock_reward_id": "'$pembrock_reward_id'",
#     "token1_address": "wrap.testnet", 
#     "token_name": "wrap", 
#     "pool_id": 461,
#     "reward_token": "'$pembrock_reward_token'"
# }' --accountId $CONTRACT_NAME --gas $total_gas