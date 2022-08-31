source neardev/dev-account.env
source .env

near call $CONTRACT_NAME pembrock_unstake '{ "token_name": "'$token_name'" }' --accountId $username --gas 300000000000000 
