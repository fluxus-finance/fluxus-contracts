source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
echo $master

#### Create strategies
#hapi - ref (114)
near call safe-010.fluxusfi.testnet create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$master'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"token1_address": "hapi.fakes.testnet", "token2_address": "ref.fakes.testnet", "pool_id": 114, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 114, "pool_id_token1_reward": 114, "pool_id_token2_reward": 9999, "reward_token": "ref.fakes.testnet","farm_id": "0" }' --accountId $master --gas 300000000000000

#ref - near (17)
near call safe-010.fluxusfi.testnet create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$master'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"token1_address": "ref.fakes.testnet", "token2_address": "wrap.fakes.testnet", "pool_id": 17, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 17, "pool_id_token1_reward": 9999, "pool_id_token2_reward": 17, "reward_token": "ref.fakes.testnet","farm_id": "0" }' --accountId $master --gas 300000000000000
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 17, "pool_id_token1_reward": 9999, "pool_id_token2_reward": 17, "reward_token": "ref.fakes.testnet","farm_id": "1" }' --accountId $master --gas 300000000000000
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 17, "pool_id_token1_reward": 811, "pool_id_token2_reward": 49, "reward_token": "dai.fakes.testnet","farm_id": "2" }' --accountId $master --gas 300000000000000

#DBIO-NEAR (53)
near call safe-010.fluxusfi.testnet create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$master'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"token1_address": "dbio.fakes.testnet", "token2_address": "wrap.fakes.testnet", "pool_id": 53, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 53, "pool_id_token1_reward": 9999, "pool_id_token2_reward": 53, "reward_token": "dbio.fakes.testnet", "farm_id": "0" }' --accountId $master --gas 300000000000000
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 53, "pool_id_token1_reward": 1035, "pool_id_token2_reward": 17, "reward_token": "ref.fakes.testnet", "farm_id": "1" }' --accountId $master --gas 300000000000000

#ref - near (1033)
near call safe-010.fluxusfi.testnet create_strategy '{"_strategy": "","strategy_fee": 5,"strat_creator": { "account_id": "'$master'", "fee_percentage": 5, "current_amount" : 0 },"sentry_fee": 10,"token1_address": "ref.fakes.testnet", "token2_address": "wrap.fakes.testnet", "pool_id": 1033, "seed_min_deposit": "1000000000000000000" }' --accountId $username --gas $total_gas
near call safe-010.fluxusfi.testnet add_farm_to_strategy '{"pool_id": 1033, "pool_id_token1_reward": 9999, "pool_id_token2_reward": 1033, "reward_token": "ref.fakes.testnet","farm_id": "0" }' --accountId $master --gas 300000000000000


#### To delete the strategies:
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@114#0"}' --accountId $master.testnet
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@17#0"}' --accountId $master.testnet
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@17#1"}' --accountId $master.testnet
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@17#2"}' --accountId $master.testnet
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@53#0"}' --accountId $master.testnet
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@53#1"}' --accountId $master.testnet
# near call safe-010.fluxusfi.testnet delete_strategy_by_farm_id '{"farm_id_str":"ref-finance-101.testnet@1033#0"}' --accountId $master.testnet
