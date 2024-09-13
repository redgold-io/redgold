#!/bin/bash

# If no argument supplied, set default path and build the binary.
if [ -z "$1" ]
then
    echo "No argument supplied, building the binary."
    export RUSTFLAGS="-C link-arg=-fuse-ld=lld"; cargo build --profile ci
    REDGOLD_BINARY_PATH="./target/debug/redgold"
else
    REDGOLD_BINARY_PATH="$1"
fi

echo "Using binary at path: $REDGOLD_BINARY_PATH"

export REDGOLD_BINARY_PATH
export RUST_BACKTRACE=1
export RUST_MIN_STACK=20485760 # 20mb


cargo test release_test -- --nocapture