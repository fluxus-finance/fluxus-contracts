
######## Relevant methods to interact with pools and farms

source .env
source neardev/dev-account.env
echo $CONTRACT_NAME


######## Old farm contract

#### Get all farms from given seed
# near view $farm_contract_id list_farms_by_seed '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'

### Get farm state, Running, Ended, etc.
# near view $farm_contract_id get_farm '{ "farm_id": "'$exchange_contract_id'@'$pool_id'#'$farm_id'" }'

### Get min deposit for seed
# near view $farm_contract_id list_seeds_info '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'

#### Get unclaimed reward
# near view $farm_contract_id get_unclaimed_reward '{ "account_id": "'$CONTRACT_NAME'", "farm_id": "'$exchange_contract_id'@'$pool_id'#'$farm_id'" }'

######## Boost farm contract
# near view $farm_contract_id list_seed_farms '{ "seed_id": "'$exchange_contract_id'@'$pool_id'" }'
# near view $farm_contract_id list_farmer_seeds '{ "farmer_id": "'$CONTRACT_NAME'" }'
# near view $farm_contract_id list_farmer_rewards '{ "farmer_id": "'$CONTRACT_NAME'" }'
# near view $farm_contract_id get_unclaimed_rewards '{ "farmer_id": "'$CONTRACT_NAME'", "seed_id": "'$seed_id'" }'
# near view $farm_contract_id list_seeds_info '{ "from_index": 0, "limit": 300  }'

######## Safe contract

#### Get the current contract state
near view $CONTRACT_NAME get_contract_state
# 'contract_id is Running'

#### Get exchange and farm contracts
near view $CONTRACT_NAME get_contract_info
# {
#   exchange_address: 'ref-finance-101.testnet',
#   farm_address: 'boostfarm.ref-finance.testnet'
# }

### Get tokens_id, :10, for strategies that are running
near view $CONTRACT_NAME get_allowed_tokens '{}'
# [ ':50' ]

#### Get all strats infos, such as state, token_id, reward_token, min_deposit
near view $CONTRACT_NAME get_strategies '{}'
# [
#   {
#     token_id: ':50',
#     is_active: true,
#     reward_tokens: [ 'skyward.fakes.testnet' ]
#   }
# ]

#### Get state from strat, if its Running, Ended, etc.
near view $CONTRACT_NAME get_strat_state '{"farm_id_str": "'$farm_id_str'" }'
# 'Running'

#### Returns number of shares the user has for given seed_id
near view $CONTRACT_NAME user_share_seed_id '{ "seed_id": "'$seed_id'", "user": "'$username'" }'
# 1000000000000000000

#### Get guardians
near view $CONTRACT_NAME get_guardians '{}'
# []

#### Get total amount staked on contract
near view $CONTRACT_NAME get_contract_amount '{}'
# '0'

#### Returns the total number of strategies/farms in this contract
near view $CONTRACT_NAME number_of_strategies '{}'
# 1

#### Returns the total staked for given seed
near view $CONTRACT_NAME seed_total_amount '{ "token_id": "'$token_id'" }'
# 1000000000000000000

#### Get fee percentage
near view $CONTRACT_NAME check_fee_by_strategy '{ "token_id": "'$token_id'" }'
# '5%'

#### Returns true/false for given strategy
near view $CONTRACT_NAME is_strategy_active '{ "token_id": "'$token_id'" }'

#### Returns strat step, ['claim_reward, 'withdraw', 'swap', 'stake']
near view $CONTRACT_NAME current_strat_step '{ "farm_id_str": "'$farm_id_str'" }'
# 'claim_reward'

#### Returns farm ids from given token_id
near view $CONTRACT_NAME get_farm_ids_by_seed '{ "token_id": "'$token_id'" }'
# [ ':50#1' ]

#### Returns the timestamp of the last harvest for given token_id, '0' if it never occurred
near view $CONTRACT_NAME get_harvest_timestamp  '{ "token_id": "'$token_id'" }'
# 1659470914303

#### Returns all infos for all strategies
near view $CONTRACT_NAME get_strategies_info  '{}'
# [
#   {
#     state: 'Running',
#     cycle_stage: 'ClaimReward',
#     slippage: 99,
#     last_reward_amount: 0,
#     last_fee_amount: 0,
#     pool_id_token1_reward: 9999,
#     pool_id_token2_reward: 50,
#     reward_token: 'skyward.fakes.testnet',
#     available_balance: [ 0, 0 ],
#     id: '1'
#   }
# ]

near view $CONTRACT_NAME get_strategy_kind '{}'
# 'AUTO_COMPOUNDER'