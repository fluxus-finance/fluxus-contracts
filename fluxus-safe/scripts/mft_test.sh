source .env

source neardev/dev-account.env
echo $CONTRACT_NAME

# Add croncat manager to allowed_accounts
near call $CONTRACT_NAME total_supply_amount '{"seed_id" : "seed1"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_share_amount '{"seed_id" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_mint '{"seed_id" : "seed1", "balance":100, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME total_supply_amount '{"seed_id" : "seed1"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_share_amount '{"seed_id" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME mft_burn '{"seed_id" : "seed1", "balance":1, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME total_supply_amount '{"seed_id" : "seed1"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_share_amount '{"seed_id" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME mft_burn '{"seed_id" : "seed1", "balance":2000, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
