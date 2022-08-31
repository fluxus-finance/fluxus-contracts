
######## Relevant methods to interact with pools and farms

# source .env
source neardev/dev-account.env
echo $CONTRACT_NAME

export CONTRACT_NAME=dev-1661899762592-85993301486400

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
# # 'contract_id is Running'

# ### Get tokens_id, :10, for strategies that are running
near view $CONTRACT_NAME get_allowed_tokens '{}'
# # [ ':50' ]

# #### Get all strats infos, such as state, token_id, reward_token, min_deposit
near view $CONTRACT_NAME get_strategies '{}'
# [
#   {
#     token_id: ':50',
#     is_active: true,
#     reward_tokens: [ 'skyward.fakes.testnet' ]
#   }
# ]

export farm_id_str=ref-finance-101.testnet@114#0
# # #### Get state from strat, if its Running, Ended, etc.
near view $CONTRACT_NAME get_strategy_for_ref_finance '{ "farm_id_str": "'$farm_id_str'" }'

export farm_id_str=dev-1660920856823-70071820486313@0#2
# near view $CONTRACT_NAME get_strategy_for_jumbo '{ "farm_id_str": "'$farm_id_str'" }'

near view $CONTRACT_NAME get_strategy_for_pembrock '{ "strat_name": "pembrock@wrap" }'
# 'Running'

export seed_id=ref-finance-101.testnet@114
export username=mesto.testnet
# #### Returns number of shares the user has for given seed_id
near view $CONTRACT_NAME user_share_seed_id '{ "seed_id": "'$seed_id'", "user": "'$username'" }'
# # 1000000000000000000

# #### Get guardians
near view $CONTRACT_NAME get_guardians '{}'
# # []

# TODO
# #### Get total amount staked on contract
# near view $CONTRACT_NAME get_contract_amount '{}'
# # '0'

# #### Returns the total number of strategies/farms in this contract
near view $CONTRACT_NAME number_of_strategies_by_seed '{ "seed_id": "'$seed_id'" }'
# # "1"

# TODO
# #### Returns the total number of strategies/farms in this contract
# near view $CONTRACT_NAME number_of_strategies'{}'
# # "1"

# #### Returns the total staked for given seed
near view $CONTRACT_NAME seed_total_amount '{ "seed_id": "'$seed_id'" }'
# # 1000000000000000000

# #### Get fee percentage
near view $CONTRACT_NAME check_fee_by_strategy '{ "seed_id": "'$seed_id'" }'
# # '5%'

# #### Returns true/false for given strategy
near view $CONTRACT_NAME is_strategy_active '{ "seed_id": "'$seed_id'" }'

export farm_id_str=ref-finance-101.testnet@114#0
# #### Returns strat step, ['claim_reward, 'withdraw', 'swap', 'stake']
near view $CONTRACT_NAME current_strat_step '{ "farm_id_str": "'$farm_id_str'", "strat_name": "" }'
near view $CONTRACT_NAME current_strat_step '{ "farm_id_str": "", "strat_name": "pembrock@wrap" }'
# # 'claim_reward'

# #### Returns the timestamp of the last harvest for given token_id, '0' if it never occurred
near view $CONTRACT_NAME get_harvest_timestamp  '{ "seed_id": "'$seed_id'" }'
# # 1659470914303

# #### Returns all infos for all strategies
near view $CONTRACT_NAME get_strategies_info_for_ref_finance  '{}'
# {
#     state: 'Running',
#     cycle_stage: 'ClaimReward',
#     slippage: 99,
#     last_reward_amount: 0,
#     last_fee_amount: 0,
#     pool_id_token1_reward: 3,
#     pool_id_token2_reward: 2,
#     reward_token: 'dai.fakes.testnet',
#     available_balance: [ 0, 0 ],
#     id: '2'
#   }

near view $CONTRACT_NAME get_strategies_info_for_stable_ref_finance  '{}'
# {
#     state: 'Running',
#     cycle_stage: 'ClaimReward',
#     slippage: 99,
#     last_reward_amount: 0,
#     last_fee_amount: 0,
#     token_address: 'dai.fakes.testnet',
#     pool_id_token_reward: 811,
#     token_position: 2,
#     reward_token: 'ref.fakes.testnet',
#     available_balance: [ 0, 0, 0 ],
#     id: '0'
#   }

near view $CONTRACT_NAME get_strategies_info_for_pembrock  '{}'
# {
#     admin_fees: {
#       strategy_fee: 5,
#       strat_creator: {
#         account_id: 'mesto.testnet',
#         fee_percentage: 5,
#         current_amount: 0
#       },
#       sentries_fee: 10,
#       sentries: {}
#     },
#     exchange_contract_id: 'ref-finance-101.testnet',
#     pembrock_contract_id: 'dev-v1.slovko.testnet',
#     pembrock_reward_id: 'reward-v1.slovko.testnet',
#     token1_address: 'wrap.testnet',
#     token_name: 'wrap',
#     state: 'Running',
#     cycle_stage: 'ClaimReward',
#     slippage: 99,
#     last_reward_amount: 0,
#     last_fee_amount: 0,
#     pool_id_token1_reward: 461,
#     reward_token: 'token.pembrock.testnet',
#     available_balance: 0,
#     harvest_timestamp: 0,
#     harvest_value_available_to_stake: 0
#   }


# near view $CONTRACT_NAME get_strategy_kind '{}'
# # 'AUTO_COMPOUNDER'