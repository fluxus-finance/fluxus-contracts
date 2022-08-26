source .env
source neardev/dev-account.env

#At Pembrock
near call $farm_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": true}' --accountId $CONTRACT_NAME --deposit 0.1
#At Pemb token
near call token.pembrock.testnet storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": true}' --accountId $CONTRACT_NAME --deposit 0.1
#At Pemb reward contract
near call reward-v1.slovko.testnet storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": true}' --accountId $CONTRACT_NAME --deposit 0.1
