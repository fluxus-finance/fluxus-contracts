source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

# Add croncat manager to allowed_accounts
near call $CONTRACT_NAME add_allowed_account '{"account_id" : "'$croncat_manager'"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME get_allowed_accounts '{}' --accountId $CONTRACT_NAME

export args='{"token_id": "'$token_id'"}'
export args_encoded=$(echo $args | base64)

#### Start croncat tasks responsible for auto-compound 
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "claim_reward","cadence": "* 30 * * * *","recurring": false,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000 
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "withdraw_of_reward","cadence": "* 33 * * * *","recurring": false,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000 
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "autocompounds_swap","cadence": "* 36 * * * *","recurring": false,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000
# near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "autocompounds_liquidity_and_stake","cadence": "* 39 * * * *","recurring": false,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000

#### Functions managed by auto-compound
near call $CONTRACT_NAME claim_reward '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas

near call $CONTRACT_NAME withdraw_of_reward '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas 

near call $CONTRACT_NAME autocompounds_swap '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas 

near call $CONTRACT_NAME autocompounds_liquidity_and_stake '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas 

