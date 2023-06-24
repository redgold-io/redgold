#!/bin/bash

set -e

git clone https://github.com/redgold-io/GraphEmbedding.git || true

cd ./GraphEmbedding || exit 1

rm -rf venv || true

python3 -m venv venv

./venv/bin/python3 -m pip install Cython pkgconfig tensorflow pyinstaller

# also need brew install hdf5 if /opt/homebrew present
# IF MAC only? need to detect
export CPATH="/opt/homebrew/include/"
export HDF5_DIR=/opt/homebrew/

./venv/bin/python3 setup.py install develop

cd ../python/model || exit 1

../../GraphEmbedding/venv/bin/pyinstaller main.py --noconfirm

zip -r dist.zip dist/main