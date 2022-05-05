
source neardev/dev-account.env
echo $CONTRACT_NAME

source .env


near call $exchange_contract_id mft_transfer_call '{ 
    "token_id": ":410", "receiver_id": "'$CONTRACT_NAME'", "amount": "439338891236896303998", "msg": "" }' --accountId mesto.testnet --gas 300000000000000 --depositYocto 1
 