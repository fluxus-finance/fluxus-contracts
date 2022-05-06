# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/auto_compounder.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

near call ref-finance-101.testnet get_pool_shares '{ "pool_id": '$pool_id', "account_id" : "'$username'" }' --accountId $CONTRACT_NAME


# near call $exchange_contract_id mft_transfer_call '
#     {"token_id": ":'$pool_id'", "receiver_id": "'$CONTRACT_NAME'", "amount": "87823767783282549971", "msg": "" }' --accountId $username --gas $total_gas --depositYocto 1
 
near call $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME