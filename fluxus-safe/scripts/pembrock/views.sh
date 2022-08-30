
source neardev/dev-account.env
source .env

near view $pembrock_contract_id get_tokens '{"account_id":"'$CONTRACT_NAME'","from_index":0,"limit":100}'

near view $pembrock_contract_id get_user_positions '{"account_id":"'$CONTRACT_NAME'","from_index":0,"limit":100}'

near view $pembrock_reward_id get_claimed_rewards '{"account_id":"'$CONTRACT_NAME'"}'

near view $pembrock_contract_id get_account '{"account_id":"'$CONTRACT_NAME'"}'

near view $pembrock_reward_token ft_balance_of '{"account_id":"'$CONTRACT_NAME'"}'

near view $CONTRACT_NAME user_share_seed_id '{ "seed_id": "'$seed_id'", "user": "'$username'" }'

