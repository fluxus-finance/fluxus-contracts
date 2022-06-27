source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username


#### Initialize contract
near call $CONTRACT_NAME new '{"owner_id":"'$username'", "token_out": "'$token_out'", "exchange_contract_id": "'$exchange_contract_id'"}' --accountId $username


#### Register contract 

#At ref
near call $exchange_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 1

#At token_out
near call $token_out storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125

# Make tokens available to swap on exchange (calls storage_deposit and register_tokens)
near call $CONTRACT_NAME register_token '{ "token" : "'$token_in'", "pool_id": '$pool_token_in' }' --accountId $CONTRACT_NAME --gas $total_gas 

# Add stakeholder account
near call $CONTRACT_NAME add_stakeholder '{ "account_id": "'$username'", "fee": '$fee' }' --accountId $CONTRACT_NAME

# Get contracts stakeholders
near call $CONTRACT_NAME get_stakeholders '{}' --accountId $CONTRACT_NAME

# Update contract to Paused, allowing only withdraw operations
# near call $CONTRACT_NAME update_contract_state '{ "state": "Paused" }' --accountId $CONTRACT_NAME