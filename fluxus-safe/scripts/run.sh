#Running run.sh: 

# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/fluxus_safe.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
 

#### initializes the contract, create strategy and registers in the necessary contracts
# ./initialize.sh

# #### create strategy from .env
# ./add_strategy.sh

# #### storage_deposit + wrap_near + stake   
# ./stake_process.sh

#### unstake_and_remove_liquidity + withdraw_to_contract
# ./unstake_process.sh
 
 