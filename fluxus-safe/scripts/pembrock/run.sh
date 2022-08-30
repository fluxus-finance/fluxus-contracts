#Running run.sh: 

# build wasm
../build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../../res/fluxus_safe.wasm

#### initializes the contract, create strategy and registers in the necessary contracts
./initialize.sh

./register.sh

./add_strategy.sh

./stake.sh

./harvest.sh

./unstake.sh
 