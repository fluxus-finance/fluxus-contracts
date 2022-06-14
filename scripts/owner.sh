source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

#### Update contract to Paused, making stake and auto-compound unavailable
near call $CONTRACT_NAME update_contract_state '{ "state": "Paused" }' --accountId $CONTRACT_NAME

#### Update strategy to Ended, where only unstake is available from given token_id
near call $CONTRACT_NAME update_compounder_state ' {"token_id": "'$token_id'", "state":"Ended" }' --accountId $CONTRACT_NAME

#### Give extra permissions to given addresses
near call $CONTRACT_NAME extend_guardians '{ "guardians": ["'$username'"] }' --accountId $username --depositYocto 1

### Only used by contracts admins, returns the same as get_stras plus current users infos
near call $CONTRACT_NAME get_strats_info '{}' --accountId $CONTRACT_NAME