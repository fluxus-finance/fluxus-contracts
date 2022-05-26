source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

# #### User shares on auto-compouder contract
near view $CONTRACT_NAME get_user_shares '{ "account_id": "'$username'", "token_id": "'$token_id'" }'

### Auto-compoter staked shares
near view $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' 

#### Unstake, swap to wnear and send it to auto_compounder contract.
near call $CONTRACT_NAME unstake '{ "token_id": ":'$pool_id'" }' --accountId $username --gas 300000000000000 

#### Shoud have the contract shares minus the user shares
near view $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' 

### Should be 0 after successful unstake
near view $CONTRACT_NAME get_user_shares '{ "account_id": "'$username'", "token_id": "'$token_id'" }'

### Should have the previous shares on the auto-compounder contract
near view $exchange_contract_id get_pool_shares '{ "pool_id": '$pool_id', "account_id" : "'$username'" }' 


