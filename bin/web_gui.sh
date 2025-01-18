rustup target add wasm32-unknown-unknown
cargo build -p redgold-gui --target wasm32-unknown-unknown
cd crates/gui || exit;
trunk build --release;
cd ../..;