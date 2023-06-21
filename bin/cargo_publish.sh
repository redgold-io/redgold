#!/bin/bash

set -e

cd schema
cargo publish
cd ..

cd data
cargo publish
cd ..

cargo publish