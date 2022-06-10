source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token


#### Initialize contract
near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "exchange_contract_id": "'$exchange_contract_id'", 
    "farm_contract_id": "'$farm_contract_id'" }' --accountId $CONTRACT_NAME

#### Register contract 

#At ref
near call $CONTRACT_NAME call_user_register '{"account_id": "'$CONTRACT_NAME'"}' --accountId $CONTRACT_NAME

#At the farm
near call $farm_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1

#At reward token
near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Register reward_token in the exchange in the contracts account whitelisted tokens
near call $exchange_contract_id register_tokens '{ "token_ids" : [ "'$reward_token'" ] }' --accountId $CONTRACT_NAME  --gas 300000000000000 --depositYocto 1

