# Auto-compounder Contract


## Interact with Auto-compounder

#### Add shares to contract and stake on farm
`near call exchange_contract_id mft_transfer_call '{"token_id": ":pool_id", "receiver_id": "AUTO_COMPOUNDER_ID", "amount": "lp_amount", "msg": "" }' --accountId user_address --gas 300000000000000 --depositYocto 1`

#### Unstake from farm and send shares to user account on exchange.
`near call AUTO_COMPOUNDER_ID unstake '{}' --accountId user_address --gas 300000000000000`

#### User shares on auto-compounder contract
`near view AUTO_COMPOUNDER_ID get_user_shares '{ "account_id": "user_address"}'`

## Helper methods from exchange and farm contracts
#### Get shares that the user has in the pool
`near view exchange_contract_id get_pool_shares '{ "pool_id": pool_id, "account_id" : "user_address" }'` 

#### Auto-compounder staked shares
`near view farm_contract_id list_user_seeds '{ "account_id": "AUTO_COMPOUNDER_ID" }'` 



## Safe Architecture

Vec<AutoCompounder> -> { Auto1, Auto2, Auto3 }
Map<String, AutoCompounder> -> { {":17", Auto1}, {":410", Auto2}}