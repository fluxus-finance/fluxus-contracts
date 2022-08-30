source .env
source neardev/dev-account.env

#Pass this to the create_strategy folder:
#At lend token
near call $token_address storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1


near call $CONTRACT_NAME storage_deposit '{"account_id": "'$username'", "registration_only": false}' --accountId $username --deposit 0.01

near call $token_address ft_transfer_call '{
    "receiver_id": "'$CONTRACT_NAME'",
    "amount": "100000000000000000000000",
    "msg": "deposit"}' --accountId $username --gas 300000000000000 --depositYocto 1