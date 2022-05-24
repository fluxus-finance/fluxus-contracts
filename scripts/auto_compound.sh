source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

# Add croncat manager to allowed_accounts
near call $CONTRACT_NAME add_allowed_account '{"account_id" : "'$croncat_manager'"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME get_allowed_accounts '{}' --accountId $CONTRACT_NAME


#### Start croncat tasks responsible for auto-compound 
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "claim_reward","cadence": "* 0 * * * *","recurring": true,"deposit": "0","gas": 240000000000000}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000 
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "withdraw_of_reward","cadence": "* 3 * * * *","recurring": true,"deposit": "1","gas": 240000000000000}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000 
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "autocompounds_swap","cadence": "* 6 * * * *","recurring": true,"deposit": "1","gas": 240000000000000 }' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "autocompounds_liquidity_and_stake","cadence": "* 9 * * * *","recurring": true,"deposit": "0","gas": 240000000000000}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000

#### Functions managed by auto-compound
# near call $CONTRACT_NAME claim_reward '{}' --accountId $CONTRACT_NAME --gas $total_gas

# near call $CONTRACT_NAME withdraw_of_reward '{}' --accountId $CONTRACT_NAME --gas $total_gas --depositYocto 1

# near call $CONTRACT_NAME autocompounds_swap '{}' --accountId $CONTRACT_NAME --gas $total_gas --depositYocto 1 

# near call $CONTRACT_NAME autocompounds_liquidity_and_stake '{}' --accountId $CONTRACT_NAME --gas $total_gas 

