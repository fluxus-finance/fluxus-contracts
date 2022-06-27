source .env

source neardev/dev-account.env
echo $CONTRACT_NAMEONTRACT_NAME

# Add croncat manager to allowed_accounts
near call $CONTRACT_NAME add_allowed_account '{"account_id" : "'$croncat_manager'"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME get_allowed_accounts '{}' --accountId $CONTRACT_NAME

export args='{"token_id": "'$token_id'"}'
export args_encoded=$(echo $args | base64)

### Start croncat tasks responsible for auto-compound, use https://crontab.guru/ and add an extra '*' to the left in the cadence
near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "claim_reward","cadence": "* 0 * * * *","recurring": true,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000 
near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "withdraw_of_reward","cadence": "* 2 * * * *","recurring": true,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000 
near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "autocompounds_swap","cadence": "* 4 * * * *","recurring": true,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000
near call manager_v1.croncat.testnet create_task '{"contract_id": "'$CONTRACT_NAME'", "function_id": "autocompounds_liquidity_and_stake","cadence": "* 6 * * * *","recurring": true,"deposit": "0","gas": 240000000000000, "arguments":"'$args_encoded'"}' --accountId $CONTRACT_NAME --amount 1 --gas 300000000000000

