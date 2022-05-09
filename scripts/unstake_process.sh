source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

# #### User shares on auto-compouder contract
near call $CONTRACT_NAME get_user_shares '{ "account_id": "'$username'"}' --accountId $CONTRACT_NAME --gas $total_gas

### Auto-compoter staked shares
near call $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME

#### Unstake, swap to wnear and send it to auto_compounder contract.
near call $CONTRACT_NAME unstake '{}' --accountId $username --gas 300000000000000 

#### Shoud have the contract shares minus the user shares
near call $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME

### Should be 0 after successful unstake
near call $CONTRACT_NAME get_user_shares '{ "account_id": "'$username'"}' --accountId $CONTRACT_NAME --gas $total_gas

### Should have the previous shares on the auto-compounder contract
near call $exchange_contract_id get_pool_shares '{ "pool_id": '$pool_id', "account_id" : "'$username'" }' --accountId $CONTRACT_NAME


