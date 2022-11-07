
source .env
echo $username

source neardev/dev-account.env
echo $CONTRACT_NAME


near call $CONTRACT_NAME harvest '{"farm_id_str": "'$farm_id_str'", "strat_name": "" }' --accountId $username --gas $total_gas
