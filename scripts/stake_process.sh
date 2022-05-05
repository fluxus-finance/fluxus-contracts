source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

#### Swap, add liquidity, save new lp user balance, stake, claim, withdraw
near call $CONTRACT_NAME stake '{}' --accountId $username --gas 300000000000000 --deposit 0.01

