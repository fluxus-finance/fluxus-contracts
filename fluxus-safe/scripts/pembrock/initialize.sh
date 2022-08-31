source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token

#### Initialize contract
near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "treasure_contract_id": "'$treasure_contract_id'" }' --accountId $CONTRACT_NAME

#### Register contract 

#At ref
near call $exchange_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false }' --accountId $CONTRACT_NAME --deposit 1
