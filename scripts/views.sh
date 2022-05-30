
#### Relevant methods to interact with pools and farms

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

#### consult if farm is running
# near view $farm_contract_id get_farm '{ "farm_id": "'$exchange_contract_id'@'$pool_id'#'$farm_id'" }'

#### consult min deposit for seed
# near view $farm_contract_id get_seed_info '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'

#### Get the current contract state
# near view $CONTRACT_NAME get_contract_state

#### Get exchange and farm contracts
near view $CONTRACT_NAME get_contract_info

# near view $CONTRACT_NAME get_allowed_tokens '{}'

# near view $CONTRACT_NAME get_strats '{}'

# near call $CONTRACT_NAME get_strats_info '{}' --accountId $CONTRACT_NAME

# near view $CONTRACT_NAME get_user_shares '{ "account_id": "'$username'", "token_id": ":'$pool_id'" }'
