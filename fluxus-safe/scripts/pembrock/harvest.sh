
source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

near call $CONTRACT_NAME harvest '{"farm_id_str": "", "strat_name":"pembrock@usdt.fakes.testnet"}' --accountId $username --gas 300000000000000

near call $CONTRACT_NAME harvest '{"farm_id_str": "", "strat_name":"pembrock@usdt.fakes.testnet"}' --accountId $username --gas 300000000000000
