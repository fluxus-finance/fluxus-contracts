

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env

near call $CONTRACT_NAME create_auto_compounder '{
    "token1_address": "'$token1_address'", 
    "token2_address": "'$token2_address'", 
    "pool_id_token1_reward": "'$pool_id_token1_reward'", 
    "pool_id_token2_reward": "'$pool_id_token2_reward'", 
    "reward_token": "'$reward_token'",
    "farm": "'$farm_id'", 
    "pool_id": "'$pool_id'", 
    "seed_min_deposit": "1000000000000000000" 
    }' --accountId $CONTRACT_NAME --gas $total_gas