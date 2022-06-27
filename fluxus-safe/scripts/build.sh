#!/bin/bash
set -e

if [ -d "../res" ]; then
  echo ""
else
  mkdir ../res
fi

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release

cp ../target/wasm32-unknown-unknown/release/auto_compounder.wasm ../res/

