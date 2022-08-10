source .env

source neardev/dev-account.env
echo $CONTRACT_NAME


near call $CONTRACT_NAME register_seed '{"fft_share" : "seed1"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_mint '{"fft_share" : "seed1", "balance":100, "user": "'$username'"}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME mft_burn '{"fft_share" : "seed1", "balance":1, "user": '$username'}' --accountId $CONTRACT_NAME
near call $CONTRACT_NAME users_fft_share_amount '{"fft_share" : "seed1", "user": '$username'}' --accountId $CONTRACT_NAME
