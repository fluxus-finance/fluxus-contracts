#Running run2_0.sh: 

# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/auto_compounder.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
 

# initializes the contract and registers the necessary 
./initialize.sh

# storage_deposit + wrap_near + stake   
# ./stake_process.sh

# unstake_and_remove_liquidity + withdraw_to_contract
# ./unstake_process.sh
 
 