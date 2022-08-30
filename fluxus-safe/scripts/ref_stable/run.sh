#Running run.sh: 

# build wasm
../build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../../res/fluxus_safe.wasm

#### initializes the contract, create strategy and registers in the necessary contracts
./initialize.sh

#### create stable strategy
./add_stable_strategy.sh

# #### storage_deposit + wrap_near + stake   
./stake_process.sh

#### unstake_and_remove_liquidity + withdraw_to_contract
# ./unstake_process.sh
 
 
