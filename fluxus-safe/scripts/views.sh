
######## Relevant methods to interact with pools and farms

source .env
source neardev/dev-account.env
echo $CONTRACT_NAME

######## Farm contract

#### Get all farms from given seed
# near view $farm_contract_id list_farms_by_seed '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'

#### Get farm state, Running, Ended, etc.
# near view $farm_contract_id get_farm '{ "farm_id": "'$exchange_contract_id'@'$pool_id'#'$farm_id'" }'

#### Get min deposit for seed
# near view $farm_contract_id get_seed_info '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'

#### Get unclaimed reward
# near view $farm_contract_id get_unclaimed_reward '{ "account_id": "'$CONTRACT_NAME'", "farm_id": "'$exchange_contract_id'@'$pool_id'#'$farm_id'" }'

######## Safe contract

#### Get the current contract state
# near view $CONTRACT_NAME get_contract_state

# #### Get exchange and farm contracts
# near view $CONTRACT_NAME get_contract_info

# #### Get tokens_id, :10, for strategies that are running
# near view $CONTRACT_NAME get_allowed_tokens '{}'

# #### Get all strats infos, such as state, token_id, reward_token, min_deposit
# near view $CONTRACT_NAME get_strats '{}'

# #### Get state from strat, if its Running, Ended, etc.
# near view $CONTRACT_NAME get_strat_state '{"token_id": "'$token_id'" }'

# #### Returns number of shares the user has for given seed_id {deposited: x, total: y}
# near view $CONTRACT_NAME user_share_seed_id '{ "seed_id": "'$seed_id'", "user": "'$username'" }'

# #### Get guardians
# near view $CONTRACT_NAME get_guardians '{}'

# # #### Get total amount staked on contract
# near view $CONTRACT_NAME get_contract_amount '{}'

near view $farm_contract_id get_unclaimed_reward '{ "account_id" : "dev-1658854662494-59284302906723", "farm_id": "ref-finance-101.testnet@17#1" }' # ref
near view $farm_contract_id get_unclaimed_reward '{ "account_id" : "dev-1658854662494-59284302906723", "farm_id": "ref-finance-101.testnet@17#8" }' # dbio