source .env
source neardev/dev-account.env

#At Pembrock
near call $pembrock_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1
#At Pemb token
near call $pembrock_reward_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": true}' --accountId $CONTRACT_NAME --deposit 0.1
#At Pemb reward contract
near call $pembrock_reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": true}' --accountId $CONTRACT_NAME --deposit 0.1
