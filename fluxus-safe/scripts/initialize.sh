source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token

#### Initialize contract
near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "treasure_contract_id": "'$treasure_contract_id'" }' --accountId $CONTRACT_NAME

#### Register contract 

#At ref
near call $CONTRACT_NAME call_user_register '{"exchange_contract_id": "'$exchange_contract_id'", "account_id": "'$CONTRACT_NAME'"}' --accountId $CONTRACT_NAME

#At the farm
near call $farm_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1


