source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $reward_token

#### Create first strategy
near call $CONTRACT_NAME create_strategy '{
    "_strategy": "",
    "protocol_fee": 10,
    "token1_address": "'$token1_address'", 
    "token2_address": "'$token2_address'", 
    "pool_id_token1_reward": '$pool_id_token1_reward', 
    "pool_id_token2_reward": '$pool_id_token2_reward', 
    "reward_token": "'$reward_token'",
    "farm": "'$farm_id'", 
    "pool_id": '$pool_id', 
    "seed_min_deposit": "1000000000000000000" 
    }' --accountId $CONTRACT_NAME --gas $total_gas


# Register the contract in the pool 
#### TODO: move this call to create_auto_compounder method
near call $exchange_contract_id mft_register '{ "token_id" : ":'$pool_id'", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
