# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/auto_compounder.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env


near call $exchange_contract_id mft_transfer_call '{ 
    "token_id": ":410", "receiver_id": "'$CONTRACT_NAME'", "amount": "439338891236896303998", "msg": "" }' --accountId leopollum.testnet --gas 300000000000000 --depositYocto 1
 