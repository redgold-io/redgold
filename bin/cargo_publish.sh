#!/bin/bash

set -e

cd crates/schema
cargo publish
cd ..

cd crates/keys
cargo publish
cd ..

cd sdk-client
cargo publish
cd ..

cd sdk
cargo publish
cd ..

cd data
cargo publish
cd ..

cd executor
cargo publish
cd ..

cargo publish