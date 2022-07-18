source .env

source neardev/dev-account.env
echo $CONTRACT_NAME


near call $CONTRACT_NAME register_seed '{"uxu_share" : "seed1"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_mint '{"uxu_share" : "seed1", "balance":100, "user": "'$username'"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_burn '{"uxu_share" : "seed1", "balance":1, "user": '$username'}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_fft_share_amount '{"uxu_share" : "seed1", "user": '$username'}' --accountId $CONTRACT_NAME
