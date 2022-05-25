
source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username


near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "exchange_contract_id": "'$exchange_contract_id'", 
    "farm_contract_id": "'$farm_contract_id'" }' --accountId $CONTRACT_NAME