#Running run2_0.sh: 

# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/fluxus_treasurer.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username
 

#  
./initialize.sh


 