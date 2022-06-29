source neardev/dev-account.env
echo $CONTRACT_NAME

source .env



#### Functions managed by auto-compound
near call $CONTRACT_NAME harvest '{"token_id": "'$token_id'"}' --accountId $CONTRACT_NAME --gas $total_gas