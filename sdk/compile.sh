
rustup target add wasm32-unknown-unknown
rustup target add wasm32-wasi
#
## Warning host memory allows unsafe access to the host memory, this should only be used for debugging
export NAME="redgold_sdk.wasm"
export CARGO_TARGET_DIR="./target"
#echo "Building the contract executable"
#RUSTFLAGS="-C link-arg=--import-memory" cargo build --release --target wasm32-unknown-unknown && \
#	cp $CARGO_TARGET_DIR/wasm32-unknown-unknown/release/$NAME ./test_contract_host.wasm
#RUSTFLAGS="-C link-arg=--import-memory" cargo build --release --target wasm32-wasi && \
#	cp $CARGO_TARGET_DIR/wasm32-wasi/release/$NAME ./test_contract_host.wasi.wasm
#echo "Compiled using host memory"
#
## Regular compilation for untrusted code
#cargo build --release --target wasm32-unknown-unknown && \
#	cp $CARGO_TARGET_DIR/wasm32-unknown-unknown/release/$NAME ./test_contract_guest.wasm
#cargo build --release --target wasm32-wasi && \
#	cp $CARGO_TARGET_DIR/wasm32-wasi/release/$NAME ./test_contract_guest.wasi.wasm
#echo "Compiled using module memory"

#cargo build --package redgold-sdk --release --target wasm32-unknown-unknown
cargo build --package redgold-sdk --release --target wasm32-wasi
cp $CARGO_TARGET_DIR/wasm32-unknown-unknown/release/$NAME ./test_contract_guest.wasm


