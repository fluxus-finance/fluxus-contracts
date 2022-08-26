source .env
source neardev/dev-account.env

#Pass this to the create_strategy folder:
#At lend token
near call usdt.fakes.testnet storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1


near call $CONTRACT_NAME storage_deposit '{"account_id": "'$username'", "registration_only": false}' --accountId $username --deposit 0.01

near call usdt.fakes.testnet ft_transfer_call '{
    "receiver_id": "'$CONTRACT_NAME'",
    "amount": "1000000",
    "msg": "deposit"}' --accountId themans.testnet --gas 300000000000000 --depositYocto 1