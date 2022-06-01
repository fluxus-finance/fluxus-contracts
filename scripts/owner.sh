source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

near call $CONTRACT_NAME update_compounder_state ' {"token_id": "'$token_id'", "state":"Running" }' --accountId $CONTRACT_NAME
