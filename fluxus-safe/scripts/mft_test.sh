source .env

source neardev/dev-account.env
echo $CONTRACT_NAME






near call $CONTRACT_NAME register_seed '{"uxu_share" : "seed1"}' --accountId $CONTRACT_NAME


near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME


#near call $CONTRACT_NAME total_supply_amount '{"uxu_share" : "seed1"}' --accountId $CONTRACT_NAME
#near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

#near call $CONTRACT_NAME total_supply_amount '{"uxu_share" : "seed1"}' --accountId $CONTRACT_NAME
#near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME mft_burn '{"uxu_share" : "seed1", "balance":1, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
#near call $CONTRACT_NAME total_supply_amount '{"uxu_share" : "seed1"}' --accountId $CONTRACT_NAME
#near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

#near call $CONTRACT_NAME mft_burn '{"uxu_share" : "seed1", "balance":2000, "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME


near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_burn '{"uxu_share" : "seed1", "balance":1, "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME


near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME

near call $CONTRACT_NAME mft_transfer '{"token_id" : "seed1", "receiver_id": "zezinhodapaçoca.testnet", "amount":"8"}' --accountId $username--depositYocto 1
near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "zezinhodapaçoca.testnet"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_share_amount '{"uxu_share" : "seed1", "user": "leopollum.testnet"}' --accountId $CONTRACT_NAME
