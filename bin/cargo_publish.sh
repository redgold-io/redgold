#!/bin/bash

set -e

cd schema
cargo publish
cd ..

cd data
cargo publish
cd ..

cd executor
cargo publish
cd ..

cargo publish