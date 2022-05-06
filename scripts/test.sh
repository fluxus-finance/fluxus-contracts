# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/auto_compounder.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

# near call $exchange_contract_id get_pool_shares '{ "pool_id": '$pool_id', "account_id" : "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME

# near call $exchange_contract_id mft_transfer_call '
#     {"token_id": ":'$pool_id'", "receiver_id": "'$farm_contract_id'", "amount": "175647535566565099942", "msg": "" }' --accountId $CONTRACT_NAME --gas $total_gas --depositYocto 1

# near call $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME

# near call $CONTRACT_NAME unstake '{}' --accountId $CONTRACT_NAME --gas $total_gas

# near call $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME

near call $exchange_contract_id get_pool_shares '{ "pool_id": '$pool_id', "account_id" : "'$username'" }' --accountId $CONTRACT_NAME
