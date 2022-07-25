source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

#### Get shares from pool
# near view $exchange_contract_id get_pool_shares '{ "pool_id": '$pool_id', "account_id" : "'$username'" }' 

# ### Get previously staked shares
# # near view $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }'

# # #### Add shares to contract and stake on farm
near call $exchange_contract_id mft_transfer_call '{"token_id": ":'$pool_id'", "receiver_id": "'$CONTRACT_NAME'", "amount": "1000000000000000000", "msg": "" }' --accountId $username --gas $total_gas --depositYocto 1

# ### Should have the previous amount plus the user shares
near view $farm_contract_id list_user_seeds '{ "account_id": "'$CONTRACT_NAME'" }'

# ### Should be the same amount as passed in mft_transfer_call
near view $CONTRACT_NAME user_share_seed_id '{ "seed_id": "'$seed_id'", "user": "'$username'" }'
