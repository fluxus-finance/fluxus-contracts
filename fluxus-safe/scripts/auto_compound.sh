source neardev/dev-account.env
echo $CONTRACT_NAME

source .env



#### Functions managed by auto-compound
# near call $CONTRACT_NAME claim_reward '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas

# near call $CONTRACT_NAME withdraw_of_reward '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas 

# near call $CONTRACT_NAME autocompounds_swap '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas 

# near call $CONTRACT_NAME autocompounds_liquidity_and_stake '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas 
