
#### Relevant methods to interact with pools and farms

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

#### consult if farm is running
# near view $farm_contract_id get_farm '{ "farm_id": "'$exchange_contract_id'@'$pool_id'#'$farm_id'" }'

#### consult min deposit for seed
# near view $farm_contract_id get_seed_info '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'
