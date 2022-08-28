
source .env

# near view $pemb_farm_contract_id get_tokens '{"account_id":"mesto-pem.testnet","from_index":0,"limit":100}'

# near view dev-v1.slovko.testnet get_user_positions '{"account_id":"mesto-pem.testnet","from_index":0,"limit":100}'

# near view reward-v1.slovko.testnet get_claimed_rewards '{"account_id":"mesto-pem.testnet"}'

near view token.pembrock.testnet ft_balance_of '{"account_id":"mesto-pem.testnet"}'

# near view dev-v1.slovko.testnet get_account '{"account_id":"mesto-pem.testnet"}'

# near call token.pembrock.testnet storage_deposit '{ "account_id": "mesto-pem.testnet", "registration_only": false }' --accountId $username

# near view token.pembrock.testnet ft_balance_of '{ "account_id": "mesto.testnet" }'
# near view token.pembrock.testnet ft_balance_of '{ "account_id": "reward-v1.slovko.testnet" }'

# near call token.pembrock.testnet ft_transfer '{ "receiver_id": "reward-v1.slovko.testnet", "amount": "48476796799032904617", "memo": "" }' --accountId $username --depositYocto 1