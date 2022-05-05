source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

#### Unstake, swap to wnear and send it to auto_compounder contract.
near call $CONTRACT_NAME unstake '{}' --accountId $username --gas 300000000000000 
