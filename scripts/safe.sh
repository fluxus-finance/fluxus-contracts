# build wasm
./build.sh

#Deploy with near-dev
near dev-deploy --wasmFile ../res/auto_compounder.wasm

source neardev/dev-account.env
echo $CONTRACT_NAME

source .env
echo $username

./init.sh

./create.sh

./initialize.sh

./stake_process.sh

./unstake_process.sh


