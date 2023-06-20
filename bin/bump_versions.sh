#!/bin/bash

# this still doesn't work with workspaces
# need to get this working with sed or something.
# for now just do it manually
cargo install cargo-bump

set -e

cd schema
cargo bump $1
cd ..

cd data
cargo bump $1
cd ..
