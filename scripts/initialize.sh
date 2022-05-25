source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token


#### Initialize contract
# near call $CONTRACT_NAME new '{"owner_id":"'$username'", "protocol_shares": 0,
# "token1_address": "'$token1_address'", "token2_address": "'$token2_address'", 
# "pool_id_token1_reward": '$pool_id_token1_reward', "pool_id_token2_reward": '$pool_id_token2_reward', "reward_token": "'$reward_token'",
# "exchange_contract_id": "'$exchange_contract_id'", "farm_contract_id": "'$farm_contract_id'", 
# "farm_id": '$farm_id', "pool_id": '$pool_id', "seed_min_deposit": "1000000000000000000"}' --accountId $username


#### Register contract 

#At ref
near call $CONTRACT_NAME call_user_register '{"account_id": "'$CONTRACT_NAME'"}' --accountId $CONTRACT_NAME

#At the farm
near call $farm_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1

#At near wrap
near call $wrap_near_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

#At reward token
near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Register reward_token in the exchange in the contracts account whitelisted tokens
near call $exchange_contract_id register_tokens '{ "token_ids" : [ "'$reward_token'" ] }' --accountId $CONTRACT_NAME  --gas 300000000000000 --depositYocto 1

# Register the contract in the pool 
near call $exchange_contract_id mft_register '{ "token_id" : ":'$pool_id'", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1

# Update contract to Paused, making stake and auto-compound unavailable
# near call $CONTRACT_NAME update_contract_state '{ "state": "Paused" }' --accountId $CONTRACT_NAME