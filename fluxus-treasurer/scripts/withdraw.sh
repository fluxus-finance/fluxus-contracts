source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

#### Helper methdos in order to test contract operations

# Get user amount of token_in
# near view $exchange_contract_id get_deposits '{ "account_id": "'$username'" }'

# Set amount to send to contract
# export amount=

# Send amount from $username to contract
# near call $exchange_contract_id mft_transfer '
# {
#   "token_id": "'$token_in'",
#   "receiver_id": "'$CONTRACT_NAME'",
#   "amount": "'$amount'",
#   "memo": ""
# }' --accountId $username --gas $total_gas --depositYocto 1

# Register $usrename in $token_out contract if needed
# near call $token_out storage_deposit '{ "registration_only": false }' --accountId $username --gas $total_gas --deposit 1

# Get current $token_out amount $username has
# near view $token_out ft_balance_of '{ "account_id" : "'$username'" }'

#### Contract methods 

# Swap contracts amount of $token_in to $token_out
near call $CONTRACT_NAME execute_swaps '{ "token": "'$token_in'" }' --accountId $CONTRACT_NAME --gas $total_gas

# Distribute $token_out between stakeholders
near call $CONTRACT_NAME distribute '{}' --accountId $CONTRACT_NAME --gas $total_gas

# Withdraw $token_out to $username
near call $CONTRACT_NAME withdraw '{}' --accountId $username --gas $total_gas

