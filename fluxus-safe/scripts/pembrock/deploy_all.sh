# rm -r neardev

export treasure_contract_id=dev-1656420526638-61041719201929
export ref_farming_id="boostfarm.ref-finance.testnet"
export ref_exchange_id="ref-finance-101.testnet"

export pembrock_contract_id="dev-v1.slovko.testnet"
export pembrock_reward_id="reward-v1.slovko.testnet"
export pembrock_reward_token="token.pembrock.testnet"

export jumbo_exchange_id="dev-1660920856823-70071820486313"
export jumbo_farming_id="dev-1660920822779-22369253951404"

#build wasm
set -e
if [ -d "../../res" ]; then
  echo ""
else
  mkdir ../../res
fi

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release

cp ../../../target/wasm32-unknown-unknown/release/fluxus_safe.wasm ../../res/

#Deploy with near-dev
near dev-deploy --wasmFile ../../res/fluxus_safe.wasm

source .env
source neardev/dev-account.env

# ## initializes the contract, create strategy and registers in the necessary contracts
near call $CONTRACT_NAME new '{ "owner_id":"'$username'", "treasure_contract_id": "'$treasure_contract_id'" }' --accountId $CONTRACT_NAME

#### Register contract 
near call $CONTRACT_NAME storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2

# At ref
near call $ref_exchange_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2
near call $ref_farming_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2

# At jumbo
near call $jumbo_exchange_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2
near call $jumbo_farming_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2

# At pembrock
near call $pembrock_contract_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2
near call $pembrock_reward_id storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2 --gas 300000000000000
near call $pembrock_reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.2 --gas 300000000000000
near call $ref_exchange_id register_tokens '{ "token_ids" : [ "'$pembrock_reward_token'" ] }' --accountId $CONTRACT_NAME  --gas 300000000000000 --depositYocto 1


### create stable strategies
export token_address="dai.fakes.testnet"
export token_position=2
export reward_token="ref.fakes.testnet"
export pool_id_token_reward=811
export farm_id=0
export pool_id=218
export token_id=":$pool_id"
export seed_id="$ref_exchange_id@$pool_id"
export farm_id_str="$seed_id#$farm_id"
# find min_deposit in views.sh
export seed_min_deposit="1000000000000000000"

# echo "creating stable"

# near call $CONTRACT_NAME create_stable_strategy '{
#     "_strategy": "",
#     "strategy_fee": 5,
#     "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
#     "sentry_fee": 10,
#     "exchange_contract_id": "'$ref_exchange_id'", 
#     "farm_contract_id": "'$ref_farming_id'",
#     "pool_id": '$pool_id', 
#     "seed_min_deposit": "1000000000000000000" 
#     }' --accountId $CONTRACT_NAME --gas $total_gas

# near call $CONTRACT_NAME add_farm_to_stable_strategy '{
#     "seed_id": "'$seed_id'",
#     "token_address": "'$token_address'", 
#     "pool_id_token_reward": '$pool_id_token_reward', 
#     "token_position": '$token_position',
#     "reward_token": "'$reward_token'",
#     "available_balance": [0, 0, 0],
#     "farm_id": "'$farm_id'" 
# }' --accountId $CONTRACT_NAME --gas $total_gas
# near call $ref_exchange_id mft_register '{ "token_id" : ":'$pool_id'", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
# #register at token reward
# near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125


# ### create ref strategies
# export seed_id="$ref_exchange_id@114"
# echo "creating ref_finance"

# #hapi - ref (114)
# near call $ref_exchange_id mft_register '{ "token_id" : ":114", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
# near call $CONTRACT_NAME create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"exchange_contract_id": "'$ref_exchange_id'", "farm_contract_id": "'$ref_farming_id'", "token1_address": "hapi.fakes.testnet", "token2_address": "ref.fakes.testnet", "pool_id": 114, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 114, "pool_id_token2_reward": 9999, "reward_token": "ref.fakes.testnet","farm_id": "0" }' --accountId $username --gas 300000000000000

# #ref - near (17)
# export seed_id="$exchange_contract_id@17"
# near call $exchange_contract_id mft_register '{ "token_id" : ":17", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
# near call $CONTRACT_NAME create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10, "exchange_contract_id": "'$ref_exchange_id'", "farm_contract_id": "'$ref_farming_id'", "token1_address": "ref.fakes.testnet", "token2_address": "wrap.testnet", "pool_id": 17, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 9999, "pool_id_token2_reward": 17, "reward_token": "ref.fakes.testnet","farm_id": "0" }' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 9999, "pool_id_token2_reward": 17, "reward_token": "ref.fakes.testnet","farm_id": "1" }' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 811, "pool_id_token2_reward": 49, "reward_token": "dai.fakes.testnet","farm_id": "2" }' --accountId $username --gas 300000000000000

# #DBIO-NEAR (53)
# export seed_id="$exchange_contract_id@53"
# near call $CONTRACT_NAME create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"exchange_contract_id": "'$ref_exchange_id'", "farm_contract_id": "'$ref_farming_id'", "token1_address": "dbio.fakes.testnet", "token2_address": "wrap.testnet", "pool_id": 53, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
# near call $exchange_contract_id mft_register '{ "token_id" : ":53", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 9999, "pool_id_token2_reward": 53, "reward_token": "dbio.fakes.testnet", "farm_id": "0" }' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 1035, "pool_id_token2_reward": 17, "reward_token": "ref.fakes.testnet", "farm_id": "1" }' --accountId $username --gas 300000000000000

# #ref - near (1033)
# export seed_id="$exchange_contract_id@1033"
# near call $CONTRACT_NAME create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"exchange_contract_id": "'$ref_exchange_id'", "farm_contract_id": "'$ref_farming_id'", "token1_address": "ref.fakes.testnet", "token2_address": "wrap.testnet", "pool_id": 1033, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
# near call $exchange_contract_id mft_register '{ "token_id" : ":1033", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
# near call $CONTRACT_NAME add_farm_to_strategy '{"seed_id": "'$seed_id'", "pool_id_token1_reward": 9999, "pool_id_token2_reward": 1033, "reward_token": "ref.fakes.testnet","farm_id": "0" }' --accountId $username --gas 300000000000000



# #### create pembrock strategies
# export token_name="wrap"
# export token_address="wrap.testnet"
# export seed_id="pembrock@wrap"
# export pembrock_pool_id=461

# echo "creating pembrock"

# near call $token_address storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --deposit 0.1
# near call $CONTRACT_NAME pembrock_create_strategy '{
#     "strategy_fee": 5,
#     "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
#     "sentry_fee": 10,
#     "exchange_contract_id": "'$ref_exchange_id'", 
#     "pembrock_contract_id": "'$pembrock_contract_id'",
#     "pembrock_reward_id": "'$pembrock_reward_id'",
#     "token1_address": "'$token_address'", 
#     "token_name": "'$token_name'", 
#     "pool_id": '$pembrock_pool_id',
#     "reward_token": "'$pembrock_reward_token'"
#     }' --accountId $CONTRACT_NAME --gas $total_gas


### create jumbo strategies
export token1_address="usdt.fakes.testnet"
export token2_address="usdc.fakes.testnet"
export pool_id_token1_reward=3
export pool_id_token2_reward=2
export reward_token="dai.fakes.testnet"
export farm_id=2
export pool_id=0
export token_id=":$pool_id"
export seed_id="$jumbo_exchange_id@$pool_id"
export farm_id_str="$seed_id#$farm_id"
#find min_deposit in views.sh
export seed_min_deposit="1000000000000000000"


# echo "creating jumbo"

near call $CONTRACT_NAME create_jumbo_strategy '{
    "_strategy": "",
    "strategy_fee": 5,
    "strat_creator": { "account_id": "'$username'", "fee_percentage": 5, "current_amount" : 0 },
    "sentry_fee": 10,
    "exchange_contract_id": "'$jumbo_exchange_id'", 
    "farm_contract_id": "'$jumbo_farming_id'",
    "token1_address": "'$token1_address'", 
    "token2_address": "'$token2_address'", 
    "pool_id": '$pool_id', 
    "seed_min_deposit": "1000000000000000000" 
    }' --accountId $CONTRACT_NAME --gas $total_gas

near call $CONTRACT_NAME add_farm_to_jumbo_strategy '{
    "seed_id": "'$seed_id'", 
    "pool_id_token1_reward": '$pool_id_token1_reward', 
    "pool_id_token2_reward": '$pool_id_token2_reward', 
    "reward_token": "'$reward_token'",
    "farm_id": "'$farm_id'" 
}' --accountId $CONTRACT_NAME --gas $total_gas

# near call $jumbo_exchange_id mft_register '{ "token_id" : ":'$pool_id'", "account_id": "'$CONTRACT_NAME'" }' --accountId $CONTRACT_NAME --deposit 1
# near call $reward_token storage_deposit '{"account_id": "'$CONTRACT_NAME'", "registration_only": false}' --accountId $CONTRACT_NAME --gas 300000000000000 --deposit 0.00125


# near view $CONTRACT_NAME get_ref_farm_ids_by_seed '{ "seed_id": "ref-finance-101.testnet@114" }'
# near view $CONTRACT_NAME get_strategies '{}'
# near view $CONTRACT_NAME get_reward_ref_strategy '{ "farm_id_str": "ref-finance-101.testnet@17#0" }'
# near view $CONTRACT_NAME get_reward_pembrock_lend_testnet_strategy '{ "seed_id": "pembrock@wrap" }'
near view $CONTRACT_NAME get_allowed_pemb_tokens '{}'
near view $CONTRACT_NAME get_allowed_jumbo_tokens '{}'
near view $CONTRACT_NAME get_allowed_tokens '{}'

# near call $CONTRACT_NAME get_unclaimed_ref_rewards '{ "farm_id_str": "ref-finance-101.testnet@17#0" }' 



# near call wrap.testnet ft_transfer_call '{
#     "receiver_id": "'$CONTRACT_NAME'",
#     "amount": "100000000000000000000000",
#     "msg": "deposit"}' --accountId $username --gas 300000000000000 --depositYocto 1
# near call $exchange_contract_id mft_transfer_call '{"token_id": ":17", "receiver_id": "'$CONTRACT_NAME'", "amount": "1000000000000000000", "msg": "" }' --accountId $username --gas $total_gas --depositYocto 1



# near call $CONTRACT_NAME harvest '{"farm_id_str": "", "strat_name":"pembrock@wrap"}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "", "strat_name":"pembrock@wrap"}' --accountId $username --gas 300000000000000

# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#0", "strat_name":""}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#0", "strat_name":""}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#0", "strat_name":""}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#0", "strat_name":""}' --accountId $username --gas 300000000000000

# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#1", "strat_name":""}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#1", "strat_name":""}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#1", "strat_name":""}' --accountId $username --gas 300000000000000
# near call $CONTRACT_NAME harvest '{"farm_id_str": "ref-finance-101.testnet@17#1", "strat_name":""}' --accountId $username --gas 300000000000000
